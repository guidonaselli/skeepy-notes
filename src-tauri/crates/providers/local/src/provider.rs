use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::debug;

use skeepy_core::{
    NoteProvider, ProviderCapabilities, ProviderError, ProviderStability, ProviderStatus,
    RemoteNote,
};

use crate::format::LocalNotesFile;

// ─── LocalProvider ────────────────────────────────────────────────────────────

/// Reads notes from a user-managed JSON file on disk.
///
/// The file can be in two formats:
/// 1. A JSON array of note objects (simple/legacy)
/// 2. A JSON object with `{ "version": 1, "notes": [...] }` (full)
///
/// See `format::LocalNote` for the per-note schema.
pub struct LocalProvider {
    /// Path to the notes.json file.
    notes_path: PathBuf,
    status: ProviderStatus,
}

impl LocalProvider {
    pub fn new(notes_path: impl Into<PathBuf>) -> Self {
        Self {
            notes_path: notes_path.into(),
            status: ProviderStatus::Active,
        }
    }

    fn read_file(&self) -> Result<Vec<RemoteNote>, ProviderError> {
        let path = &self.notes_path;

        if !path.exists() {
            debug!(path = %path.display(), "Notes file not found — returning empty list");
            return Ok(Vec::new());
        }

        let data = std::fs::read(path)
            .map_err(|e| ProviderError::Api(format!("Cannot read {}: {e}", path.display())))?;

        let file_mtime = file_mtime(path);
        let parsed = LocalNotesFile::from_json(&data)?;

        let notes: Vec<RemoteNote> = parsed
            .notes
            .into_iter()
            .map(|n| n.into_remote(file_mtime))
            .collect();

        debug!(
            path = %path.display(),
            count = notes.len(),
            "Loaded notes from local file"
        );

        Ok(notes)
    }
}

#[async_trait]
impl NoteProvider for LocalProvider {
    fn id(&self) -> &str {
        "local"
    }

    fn display_name(&self) -> &str {
        "Notas locales"
    }

    fn status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_read: true,
            can_write: false, // V2
            can_delete: false,
            supports_labels: true,
            supports_colors: true,
            supports_checklists: true,
            // Local provider always returns full list — no incremental sync needed.
            supports_incremental_sync: false,
            stability: ProviderStability::Stable,
        }
    }

    async fn authenticate(&mut self) -> Result<(), ProviderError> {
        // No credentials needed for local files.
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
        // Local provider doesn't support incremental sync —
        // always returns the full current file contents.
        self.read_file()
    }

    async fn fetch_note(&self, source_id: &str) -> Result<RemoteNote, ProviderError> {
        let all = self.read_file()?;
        all.into_iter()
            .find(|n| n.source_id == source_id)
            .ok_or_else(|| ProviderError::Api(format!("Note not found: {source_id}")))
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn file_mtime(path: &Path) -> DateTime<Utc> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| DateTime::<Utc>::from(t))
        .unwrap_or_else(|_| Utc::now())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn provider_with_json(json: &str) -> (LocalProvider, NamedTempFile) {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{json}").unwrap();
        let provider = LocalProvider::new(f.path());
        (provider, f)
    }

    #[tokio::test]
    async fn nonexistent_file_returns_empty() {
        let provider = LocalProvider::new("/this/does/not/exist/notes.json");
        let notes = provider.fetch_notes(None).await.unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn reads_array_format() {
        let (provider, _f) = provider_with_json(
            r#"[{"title":"Hello","text":"World","color":"yellow"}]"#,
        );
        let notes = provider.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title.as_deref(), Some("Hello"));
        match &notes[0].content {
            skeepy_core::NoteContent::Text(t) => assert_eq!(t, "World"),
            _ => panic!("expected Text"),
        }
    }

    #[tokio::test]
    async fn reads_object_format() {
        let (provider, _f) = provider_with_json(
            r#"{"version":1,"notes":[{"title":"Note","text":"Body"}]}"#,
        );
        let notes = provider.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn reads_checklist() {
        let json = r#"[{
            "title": "Compras",
            "checklist": [
                {"text":"Leche","checked":false},
                {"text":"Pan","checked":true}
            ]
        }]"#;
        let (provider, _f) = provider_with_json(json);
        let notes = provider.fetch_notes(None).await.unwrap();
        assert_eq!(notes.len(), 1);
        match &notes[0].content {
            skeepy_core::NoteContent::Checklist(items) => {
                assert_eq!(items.len(), 2);
                assert!(!items[0].checked);
                assert!(items[1].checked);
            }
            _ => panic!("expected Checklist"),
        }
    }

    #[tokio::test]
    async fn invalid_json_returns_error() {
        let (provider, _f) = provider_with_json(r#"{ this is not json "#);
        let result = provider.fetch_notes(None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Api(_) => {}
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stable_source_id_for_same_content() {
        let json = r#"[{"title":"T","text":"C"}]"#;
        let (p1, _f1) = provider_with_json(json);
        let (p2, _f2) = provider_with_json(json);
        let n1 = p1.fetch_notes(None).await.unwrap();
        let n2 = p2.fetch_notes(None).await.unwrap();
        assert_eq!(n1[0].source_id, n2[0].source_id);
    }

    #[tokio::test]
    async fn explicit_id_used_as_source_id() {
        let (provider, _f) = provider_with_json(
            r#"[{"id":"my-stable-id","text":"content"}]"#,
        );
        let notes = provider.fetch_notes(None).await.unwrap();
        assert_eq!(notes[0].source_id, "my-stable-id");
    }

    #[test]
    fn provider_is_always_authenticated() {
        let p = LocalProvider::new("/some/path");
        assert!(p.status().is_usable());
    }

    #[test]
    fn capabilities_are_read_only_stable() {
        let p = LocalProvider::new("/some/path");
        let caps = p.capabilities();
        assert!(caps.can_read);
        assert!(!caps.can_write);
        assert_eq!(caps.stability, ProviderStability::Stable);
    }
}
