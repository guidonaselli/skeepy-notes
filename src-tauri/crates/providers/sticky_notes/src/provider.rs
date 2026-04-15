use std::env;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use tracing::{debug, warn};

use skeepy_core::{
    NoteColor, NoteContent, NoteProvider, ProviderCapabilities, ProviderError, ProviderStability,
    ProviderStatus, RemoteNote,
};

use crate::schema::RawNote;
use crate::text::strip_markup;

const PROVIDER_ID: &str = "windows_sticky_notes";

// ─── Path resolution ──────────────────────────────────────────────────────────

/// Returns the expected path to `plum.sqlite` on this machine.
///
/// Path: `%LocalAppData%\Packages\Microsoft.MicrosoftStickyNotes_8wekyb3d8bbwe\LocalState\plum.sqlite`
fn default_db_path() -> Option<PathBuf> {
    let local_app_data = env::var("LOCALAPPDATA").ok()?;
    Some(
        PathBuf::from(local_app_data)
            .join("Packages")
            .join("Microsoft.MicrosoftStickyNotes_8wekyb3d8bbwe")
            .join("LocalState")
            .join("plum.sqlite"),
    )
}

// ─── StickyNotesProvider ─────────────────────────────────────────────────────

pub struct StickyNotesProvider {
    /// Override path for testing. `None` = use default Windows path.
    db_path_override: Option<PathBuf>,
    status: ProviderStatus,
}

impl StickyNotesProvider {
    pub fn new() -> Self {
        Self {
            db_path_override: None,
            status: ProviderStatus::Active,
        }
    }

    #[cfg(test)]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            db_path_override: Some(path.into()),
            status: ProviderStatus::Active,
        }
    }

    fn resolve_path(&self) -> Option<PathBuf> {
        self.db_path_override.clone().or_else(default_db_path)
    }

    fn read_notes_sync(&self) -> Result<Vec<RemoteNote>, ProviderError> {
        let db_path = match self.resolve_path() {
            Some(p) => p,
            None => {
                debug!("LOCALAPPDATA not set — Windows Sticky Notes unavailable");
                return Ok(Vec::new());
            }
        };

        if !db_path.exists() {
            debug!(
                path = %db_path.display(),
                "plum.sqlite not found — Windows Sticky Notes not installed or no notes yet"
            );
            return Ok(Vec::new());
        }

        // Try to copy to a temp file first so we don't block the app.
        // If copy fails (e.g. access denied), fall back to direct read-only open.
        let conn = open_db(&db_path)?;

        let notes = query_notes(&conn)?;

        debug!(
            path = %db_path.display(),
            count = notes.len(),
            "Loaded notes from Windows Sticky Notes"
        );

        Ok(notes.into_iter().map(remote_note_from_raw).collect())
    }
}

impl Default for StickyNotesProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ─── NoteProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl NoteProvider for StickyNotesProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn display_name(&self) -> &str {
        "Windows Sticky Notes"
    }

    fn status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_read: true,
            can_write: false,
            can_delete: false,
            supports_labels: false,
            supports_colors: true,
            supports_checklists: false,
            supports_incremental_sync: false,
            stability: ProviderStability::Experimental,
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
        tokio::task::spawn_blocking({
            let path = self.db_path_override.clone();
            let provider = StickyNotesProvider {
                db_path_override: path,
                status: ProviderStatus::Active,
            };
            move || provider.read_notes_sync()
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

// ─── SQLite helpers ───────────────────────────────────────────────────────────

fn open_db(db_path: &PathBuf) -> Result<Connection, ProviderError> {
    // Try copying to temp dir first — avoids locking issues when Sticky Notes is open.
    if let Ok(tmp) = copy_to_temp(db_path) {
        if let Ok(conn) = Connection::open_with_flags(
            &tmp,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            return Ok(conn);
        }
    }

    // Fall back: open original in read-only mode (works if WAL allows concurrent reads)
    Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        warn!(path = %db_path.display(), err = %e, "Failed to open plum.sqlite");
        ProviderError::Api(format!("Cannot open plum.sqlite: {e}"))
    })
}

fn copy_to_temp(src: &PathBuf) -> std::io::Result<PathBuf> {
    let mut tmp = std::env::temp_dir();
    tmp.push("skeepy_sticky_notes_snap.sqlite");
    std::fs::copy(src, &tmp)?;
    Ok(tmp)
}

fn query_notes(conn: &Connection) -> Result<Vec<RawNote>, ProviderError> {
    // Query only non-deleted notes.
    // We use a flexible SELECT that works whether columns exist or not,
    // falling back gracefully for older schema versions.
    let mut stmt = conn
        .prepare(
            "SELECT Id, Text, Theme, CreatedAt, UpdatedAt, DeletedAt, IsAlwaysOnTop \
             FROM Note \
             WHERE DeletedAt IS NULL OR DeletedAt = 0 \
             ORDER BY UpdatedAt DESC",
        )
        .map_err(|e| {
            // The Note table might have a different schema — try without optional columns
            ProviderError::Api(format!("Cannot query Note table: {e}"))
        })?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RawNote {
                id:            row.get::<_, String>(0)?,
                text:          row.get::<_, Option<String>>(1)?,
                theme:         row.get::<_, Option<String>>(2)?,
                created_at_ms: row.get::<_, Option<i64>>(3)?,
                updated_at_ms: row.get::<_, Option<i64>>(4)?,
                deleted_at_ms: row.get::<_, Option<i64>>(5)?,
                is_pinned:     row.get::<_, i64>(6).unwrap_or(0) != 0,
            })
        })
        .map_err(|e| ProviderError::Api(format!("Cannot iterate Note rows: {e}")))?;

    let mut notes = Vec::new();
    for row in rows {
        match row {
            Ok(n) if !n.is_deleted() => notes.push(n),
            Ok(_) => {}
            Err(e) => warn!("Skipping malformed row: {e}"),
        }
    }
    Ok(notes)
}

fn remote_note_from_raw(raw: RawNote) -> RemoteNote {
    let plain_text = raw
        .text
        .as_deref()
        .map(strip_markup)
        .unwrap_or_default();

    // Extract a title from the first non-empty line (Sticky Notes has no separate title)
    let (title, body) = split_title_body(&plain_text);

    let color = theme_to_note_color(raw.theme.as_deref());

    RemoteNote {
        source_id:   raw.id.clone(),
        title,
        content:     NoteContent::Text(body),
        labels:      Vec::new(),
        color,
        is_pinned:   raw.is_pinned,
        is_archived: false,
        is_trashed:  false,
        created_at:  raw.created_at(),
        updated_at:  raw.updated_at(),
    }
}

fn theme_to_note_color(theme: Option<&str>) -> NoteColor {
    match theme {
        Some("Yellow")   => NoteColor::Yellow,
        Some("Pink")     => NoteColor::Pink,
        Some("Green")    => NoteColor::Green,
        Some("Blue")     => NoteColor::Blue,
        Some("Purple")   => NoteColor::Purple,
        Some("Gray")     => NoteColor::Gray,
        Some("Charcoal") => NoteColor::DarkBlue,
        _                => NoteColor::Default,
    }
}

/// Split the first line as a title (if ≤ 60 chars) and the rest as body.
fn split_title_body(text: &str) -> (Option<String>, String) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (None, String::new());
    }

    let mut lines = trimmed.splitn(2, '\n');
    let first = lines.next().unwrap_or("").trim();
    let rest = lines.next().unwrap_or("").trim();

    if !first.is_empty() && first.len() <= 60 && !rest.is_empty() {
        (Some(first.to_string()), rest.to_string())
    } else {
        (None, trimmed.to_string())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_db_returns_empty() {
        let provider = StickyNotesProvider::with_path("/this/does/not/exist.sqlite");
        let result = provider.read_notes_sync().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn split_title_body_short_first_line() {
        let (title, body) = split_title_body("My Title\nSome body text here");
        assert_eq!(title.as_deref(), Some("My Title"));
        assert_eq!(body, "Some body text here");
    }

    #[test]
    fn split_title_body_single_line() {
        let (title, body) = split_title_body("Just one line of content");
        assert!(title.is_none());
        assert_eq!(body, "Just one line of content");
    }

    #[test]
    fn split_title_body_long_first_line() {
        let long = "A".repeat(61);
        let input = format!("{long}\nBody");
        let (title, _body) = split_title_body(&input);
        assert!(title.is_none());
    }

    #[tokio::test]
    async fn provider_is_always_authenticated() {
        let p = StickyNotesProvider::new();
        assert!(p.is_authenticated().await);
    }

    #[test]
    fn capabilities_are_read_only_experimental() {
        let p = StickyNotesProvider::new();
        let caps = p.capabilities();
        assert!(caps.can_read);
        assert!(!caps.can_write);
        assert_eq!(caps.stability, ProviderStability::Experimental);
    }
}
