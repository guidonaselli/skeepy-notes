use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use skeepy_core::{
    ChecklistItem, Label, NoteColor, NoteContent, NoteProvider, ProviderCapabilities,
    ProviderError, ProviderStability, ProviderStatus, RemoteNote,
};

use crate::frontmatter;

const PROVIDER_ID: &str = "markdown_folder";

// ─── MarkdownProvider ─────────────────────────────────────────────────────────

/// Reads all top-level `.md` files in a configured folder as notes.
///
/// - `folder_path = None` → provider is inactive, returns `[]`.
/// - Files starting with `.` or `_` are ignored.
/// - Frontmatter (`---` block at file start) is parsed for title/color/tags.
/// - `source_id` is a SHA-256 hash of the relative file path — stable across renames of the folder root.
pub struct MarkdownProvider {
    /// The folder to watch. Shared so the Tauri command can update it at runtime.
    folder: Arc<RwLock<Option<PathBuf>>>,
    status: ProviderStatus,
}

impl MarkdownProvider {
    pub fn new() -> Self {
        Self {
            folder: Arc::new(RwLock::new(None)),
            status: ProviderStatus::Active,
        }
    }

    pub fn with_folder(path: impl Into<PathBuf>) -> Self {
        Self {
            folder: Arc::new(RwLock::new(Some(path.into()))),
            status: ProviderStatus::Active,
        }
    }

    /// Returns a clone of the folder Arc so the Tauri layer can update it.
    pub fn folder_handle(&self) -> Arc<RwLock<Option<PathBuf>>> {
        Arc::clone(&self.folder)
    }

    fn read_notes_sync(&self) -> Result<Vec<RemoteNote>, ProviderError> {
        let folder = self
            .folder
            .read()
            .map_err(|_| ProviderError::Api("RwLock poisoned".into()))?
            .clone();

        let Some(folder) = folder else {
            debug!("Markdown folder not configured — returning empty list");
            return Ok(Vec::new());
        };

        if !folder.exists() {
            debug!(
                path = %folder.display(),
                "Markdown folder does not exist — returning empty list"
            );
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(&folder)
            .map_err(|e| ProviderError::Api(format!("Cannot read directory {}: {e}", folder.display())))?;

        let mut notes = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();

            if !is_eligible(&path) {
                continue;
            }

            match note_from_file(&path, &folder) {
                Ok(note) => notes.push(note),
                Err(e) => warn!(path = %path.display(), err = %e, "Skipping markdown file"),
            }
        }

        debug!(
            path = %folder.display(),
            count = notes.len(),
            "Loaded markdown notes"
        );

        Ok(notes)
    }
}

impl Default for MarkdownProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ─── NoteProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl NoteProvider for MarkdownProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn display_name(&self) -> &str {
        "Carpeta Markdown"
    }

    fn status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_read: true,
            can_write: false,
            can_delete: false,
            supports_labels: true,
            supports_colors: true,
            supports_checklists: false,
            supports_incremental_sync: false,
            stability: ProviderStability::Stable,
        }
    }

    async fn authenticate(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        true
    }

    async fn revoke_auth(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn fetch_notes(
        &self,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<RemoteNote>, ProviderError> {
        let folder = Arc::clone(&self.folder);
        let status = self.status.clone();
        tokio::task::spawn_blocking(move || {
            let provider = MarkdownProvider { folder, status };
            provider.read_notes_sync()
        })
        .await
        .map_err(|e| ProviderError::Api(format!("spawn_blocking panicked: {e}")))?
    }

    async fn fetch_note(&self, source_id: &str) -> Result<RemoteNote, ProviderError> {
        let all = self.fetch_notes(None).await?;
        all.into_iter()
            .find(|n| n.source_id == source_id)
            .ok_or_else(|| ProviderError::Api(format!("Note not found: {source_id}")))
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns true if this path should be included as a note.
fn is_eligible(path: &Path) -> bool {
    let Some(ext) = path.extension() else { return false };
    if !ext.eq_ignore_ascii_case("md") {
        return false;
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return false };
    // Ignore hidden files and "private" files (convention: _ prefix)
    !name.starts_with('.') && !name.starts_with('_')
}

/// Compute a stable source_id from the file's name (not full path).
/// Using the filename makes the ID stable even if the folder root is moved.
fn source_id(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hex::encode(hasher.finalize())
}

fn note_from_file(path: &Path, _folder_root: &Path) -> Result<RemoteNote, ProviderError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| ProviderError::Api(format!("Cannot read {}: {e}", path.display())))?;

    let (fm, body) = frontmatter::parse(&raw);

    // Title: frontmatter > first `# Heading` > stem of filename
    let title = fm.title
        .or_else(|| extract_h1(body))
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.replace(['-', '_'], " "))
                .map(|s| title_case(&s))
        });

    let color = fm.color
        .as_deref()
        .and_then(str_to_color)
        .unwrap_or(NoteColor::Default);

    let labels: Vec<Label> = fm.tags
        .into_iter()
        .map(|t| Label { id: slug(&t), name: t })
        .collect();

    // Timestamps from filesystem metadata
    let meta = std::fs::metadata(path).ok();
    let created_at = meta
        .as_ref()
        .and_then(|m| m.created().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now);
    let updated_at = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now);

    Ok(RemoteNote {
        source_id:   source_id(path),
        title,
        content:     NoteContent::Text(body.to_string()),
        labels,
        color,
        is_pinned:   fm.pinned,
        is_archived: false,
        is_trashed:  false,
        created_at,
        updated_at,
    })
}

/// Extract the text of the first `# Heading` in the body.
fn extract_h1(body: &str) -> Option<String> {
    body.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l[2..].trim().to_string())
        .filter(|s| !s.is_empty())
}

fn str_to_color(s: &str) -> Option<NoteColor> {
    match s {
        "red"       => Some(NoteColor::Red),
        "orange"    => Some(NoteColor::Orange),
        "yellow"    => Some(NoteColor::Yellow),
        "green"     => Some(NoteColor::Green),
        "teal"      => Some(NoteColor::Teal),
        "blue"      => Some(NoteColor::Blue),
        "dark_blue" | "darkblue" | "navy" => Some(NoteColor::DarkBlue),
        "purple"    => Some(NoteColor::Purple),
        "pink"      => Some(NoteColor::Pink),
        "brown"     => Some(NoteColor::Brown),
        "gray" | "grey" => Some(NoteColor::Gray),
        _ => None,
    }
}

fn slug(s: &str) -> String {
    s.to_lowercase().replace(' ', "-")
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_md_file(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join(name);
        std::fs::write(path, content).unwrap();
    }

    #[tokio::test]
    async fn no_folder_configured_returns_empty() {
        let p = MarkdownProvider::new();
        let notes = p.fetch_notes(None).await.unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn nonexistent_folder_returns_empty() {
        let p = MarkdownProvider::with_folder("/does/not/exist/markdown_notes");
        let notes = p.fetch_notes(None).await.unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn reads_plain_markdown_files() {
        let dir = TempDir::new().unwrap();
        make_md_file(&dir, "hello-world.md", "# Hello World\n\nSome content here.");
        let p = MarkdownProvider::with_folder(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title.as_deref(), Some("Hello World"));
    }

    #[tokio::test]
    async fn ignores_hidden_and_underscore_files() {
        let dir = TempDir::new().unwrap();
        make_md_file(&dir, ".hidden.md", "hidden");
        make_md_file(&dir, "_private.md", "private");
        make_md_file(&dir, "visible.md", "visible");
        let p = MarkdownProvider::with_folder(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn reads_frontmatter_title_and_color() {
        let dir = TempDir::new().unwrap();
        make_md_file(&dir, "note.md", "---\ntitle: My Note\ncolor: blue\n---\n\nBody text.");
        let p = MarkdownProvider::with_folder(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes[0].title.as_deref(), Some("My Note"));
        assert_eq!(notes[0].color, NoteColor::Blue);
    }

    #[tokio::test]
    async fn reads_frontmatter_tags_as_labels() {
        let dir = TempDir::new().unwrap();
        make_md_file(&dir, "tagged.md", "---\ntags: [work, ideas]\n---\nContent.");
        let p = MarkdownProvider::with_folder(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes[0].labels.len(), 2);
        assert_eq!(notes[0].labels[0].name, "work");
        assert_eq!(notes[0].labels[1].name, "ideas");
    }

    #[tokio::test]
    async fn stable_source_id_same_filename() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        make_md_file(&dir1, "note.md", "content a");
        make_md_file(&dir2, "note.md", "content b");
        let p1 = MarkdownProvider::with_folder(dir1.path());
        let p2 = MarkdownProvider::with_folder(dir2.path());
        let n1 = p1.fetch_notes(None).await.unwrap();
        let n2 = p2.fetch_notes(None).await.unwrap();
        // Same filename → same source_id regardless of folder
        assert_eq!(n1[0].source_id, n2[0].source_id);
    }

    #[test]
    fn capabilities_are_stable() {
        let p = MarkdownProvider::new();
        assert_eq!(p.capabilities().stability, ProviderStability::Stable);
    }

    #[test]
    fn extract_h1_finds_first_heading() {
        assert_eq!(extract_h1("# Title\n\nBody"), Some("Title".to_string()));
        assert_eq!(extract_h1("No heading here"), None);
    }

    #[test]
    fn title_case_capitalizes_first_char() {
        assert_eq!(title_case("hello world"), "Hello world".to_string());
        assert_eq!(title_case("ALREADY UPPER"), "ALREADY UPPER".to_string());
        assert_eq!(title_case(""), "".to_string());
    }
}
