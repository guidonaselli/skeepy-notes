use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use skeepy_core::{
    Label, NoteColor, NoteContent, NoteProvider, ProviderCapabilities,
    ProviderError, ProviderStability, ProviderStatus, RemoteNote,
};

use crate::frontmatter::{self, extract_inline_tags, strip_backlinks};

const PROVIDER_ID: &str = "obsidian";

// ─── ObsidianProvider ─────────────────────────────────────────────────────────

/// Reads an Obsidian vault (folder tree) as notes.
///
/// Differences from plain `MarkdownProvider`:
/// - Scans recursively (Obsidian vaults use folder hierarchies).
/// - Ignores the `.obsidian/` config folder and any `_*` or `.*` paths.
/// - Parses Obsidian-specific frontmatter fields: `aliases`, `created`, `updated`.
/// - Converts `[[backlinks]]` to plain text.
/// - Collects inline `#tags` from the note body.
/// - `source_id` is SHA-256 of the vault-relative path for stability.
pub struct ObsidianProvider {
    vault: Arc<RwLock<Option<PathBuf>>>,
}

impl ObsidianProvider {
    pub fn new() -> Self {
        Self { vault: Arc::new(RwLock::new(None)) }
    }

    pub fn with_vault(path: impl Into<PathBuf>) -> Self {
        Self { vault: Arc::new(RwLock::new(Some(path.into()))) }
    }

    /// Shared handle for runtime vault path updates.
    pub fn vault_handle(&self) -> Arc<RwLock<Option<PathBuf>>> {
        Arc::clone(&self.vault)
    }

    fn read_notes_sync(&self) -> Result<Vec<RemoteNote>, ProviderError> {
        let vault = self
            .vault
            .read()
            .map_err(|_| ProviderError::Api("RwLock poisoned".into()))?
            .clone();

        let Some(vault) = vault else {
            debug!("Obsidian vault not configured — returning empty list");
            return Ok(Vec::new());
        };

        if !vault.exists() {
            debug!(path = %vault.display(), "Obsidian vault does not exist");
            return Ok(Vec::new());
        }

        let mut notes = Vec::new();
        walk_vault(&vault, &vault, &mut notes)?;

        debug!(path = %vault.display(), count = notes.len(), "Loaded Obsidian notes");
        Ok(notes)
    }
}

impl Default for ObsidianProvider {
    fn default() -> Self { Self::new() }
}

// ─── NoteProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl NoteProvider for ObsidianProvider {
    fn id(&self) -> &str { PROVIDER_ID }

    fn display_name(&self) -> &str { "Obsidian Vault" }

    fn status(&self) -> ProviderStatus { ProviderStatus::Active }

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

    async fn authenticate(&mut self) -> Result<(), ProviderError> { Ok(()) }
    async fn is_authenticated(&self) -> bool { true }
    async fn revoke_auth(&mut self) -> Result<(), ProviderError> { Ok(()) }

    async fn fetch_notes(
        &self,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<RemoteNote>, ProviderError> {
        let vault = Arc::clone(&self.vault);
        tokio::task::spawn_blocking(move || {
            let p = ObsidianProvider { vault };
            p.read_notes_sync()
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

// ─── Recursive walker ─────────────────────────────────────────────────────────

fn walk_vault(
    root: &Path,
    dir: &Path,
    out: &mut Vec<RemoteNote>,
) -> Result<(), ProviderError> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        ProviderError::Api(format!("Cannot read dir {}: {e}", dir.display()))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        // Skip hidden folders/files, underscore-prefixed, and `.obsidian` config dir
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }

        if path.is_dir() {
            walk_vault(root, &path, out)?;
        } else if is_eligible(&path) {
            match note_from_file(&path, root) {
                Ok(note) => out.push(note),
                Err(e) => warn!(path = %path.display(), err = %e, "Skipping Obsidian file"),
            }
        }
    }

    Ok(())
}

fn is_eligible(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

fn note_from_file(path: &Path, vault_root: &Path) -> Result<RemoteNote, ProviderError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| ProviderError::Api(format!("Cannot read {}: {e}", path.display())))?;

    let (fm, body) = frontmatter::parse(&raw);

    // Strip Obsidian backlinks and collect inline #tags from body
    let clean_body = strip_backlinks(body);
    let inline_tags = extract_inline_tags(body);

    // Title: frontmatter > first H1 > filename stem
    let title = fm.title
        .or_else(|| extract_h1(&clean_body))
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.replace(['-', '_'], " "))
        });

    let color = fm.color.as_deref().and_then(str_to_color).unwrap_or(NoteColor::Default);

    // Merge frontmatter tags + inline tags (deduped)
    let mut all_tags = fm.tags.clone();
    for tag in inline_tags {
        if !all_tags.contains(&tag) {
            all_tags.push(tag);
        }
    }
    let labels: Vec<Label> = all_tags
        .into_iter()
        .map(|t| Label { id: slug(&t), name: t })
        .collect();

    // Vault-relative path for stable source_id
    let rel_path = path.strip_prefix(vault_root).unwrap_or(path);
    let rel_str = rel_path.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(rel_str.as_bytes());
    let source_id = hex::encode(hasher.finalize());

    // Timestamps: prefer frontmatter, fall back to filesystem metadata
    let meta = std::fs::metadata(path).ok();
    let fs_created = meta
        .as_ref()
        .and_then(|m| m.created().ok())
        .map(DateTime::<Utc>::from);
    let fs_updated = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from);

    let created_at = fm.created.or(fs_created).unwrap_or_else(Utc::now);
    let updated_at = fm.updated.or(fs_updated).unwrap_or_else(Utc::now);

    Ok(RemoteNote {
        source_id,
        title,
        content: NoteContent::Text(clean_body),
        labels,
        color,
        is_pinned: fm.pinned,
        is_archived: false,
        is_trashed: false,
        created_at,
        updated_at,
    })
}

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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(dir: &TempDir, name: &str, content: &str) {
        std::fs::write(dir.path().join(name), content).unwrap();
    }

    #[tokio::test]
    async fn no_vault_returns_empty() {
        let p = ObsidianProvider::new();
        assert!(p.fetch_notes(None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reads_markdown_files_recursively() {
        let dir = TempDir::new().unwrap();
        write(&dir, "note.md", "# Root Note\n\nContent.");
        let sub = dir.path().join("folder");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("sub.md"), "# Sub Note\n\nContent.").unwrap();

        let p = ObsidianProvider::with_vault(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 2);
    }

    #[tokio::test]
    async fn skips_obsidian_config_folder() {
        let dir = TempDir::new().unwrap();
        write(&dir, "note.md", "Content");
        let obsidian_dir = dir.path().join(".obsidian");
        std::fs::create_dir(&obsidian_dir).unwrap();
        std::fs::write(obsidian_dir.join("config.md"), "internal").unwrap();

        let p = ObsidianProvider::with_vault(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn parses_obsidian_frontmatter() {
        let dir = TempDir::new().unwrap();
        write(&dir, "note.md", "---\ntitle: My Note\ntags: [rust, obsidian]\ncreated: 2024-01-15\n---\nBody.");
        let p = ObsidianProvider::with_vault(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        assert_eq!(notes[0].title.as_deref(), Some("My Note"));
        assert_eq!(notes[0].labels.len(), 2);
    }

    #[tokio::test]
    async fn converts_backlinks_to_plain_text() {
        let dir = TempDir::new().unwrap();
        write(&dir, "note.md", "See [[Other Note]] and [[Target|Custom Alias]].");
        let p = ObsidianProvider::with_vault(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        let text = match &notes[0].content {
            NoteContent::Text(t) => t.clone(),
            _ => panic!("expected Text"),
        };
        assert!(text.contains("Other Note"));
        assert!(text.contains("Custom Alias"));
        assert!(!text.contains("[["));
    }

    #[tokio::test]
    async fn collects_inline_tags() {
        let dir = TempDir::new().unwrap();
        write(&dir, "note.md", "---\n---\nContent with #rust and #dev tags.");
        let p = ObsidianProvider::with_vault(dir.path());
        let notes = p.fetch_notes(None).await.unwrap();
        let label_names: Vec<&str> = notes[0].labels.iter().map(|l| l.name.as_str()).collect();
        assert!(label_names.contains(&"rust"));
        assert!(label_names.contains(&"dev"));
    }
}
