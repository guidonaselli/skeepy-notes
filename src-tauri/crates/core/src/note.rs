use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type NoteId = Uuid;
pub type ProviderId = String;

// ─── Core Note Entity ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: NoteId,
    /// Provider's native ID — stable across syncs.
    pub source_id: String,
    pub provider_id: ProviderId,
    pub title: Option<String>,
    pub content: NoteContent,
    pub labels: Vec<Label>,
    pub color: NoteColor,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    pub sync_state: SyncState,
    pub layout: NoteLayout,
}

impl Note {
    /// Create a new note that lives only in local storage.
    pub fn new_local(content: NoteContent) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();
        Self {
            source_id: id.to_string(),
            id,
            provider_id: "local".to_string(),
            title: None,
            content,
            labels: Vec::new(),
            color: NoteColor::Default,
            is_pinned: false,
            is_archived: false,
            is_trashed: false,
            created_at: now,
            updated_at: now,
            synced_at: None,
            sync_state: SyncState::LocalOnly,
            layout: NoteLayout::default(),
        }
    }

    /// A note is "visible" if it hasn't been trashed.
    /// Archived notes remain visible (they're just categorized differently).
    pub fn is_visible(&self) -> bool {
        !self.is_trashed
    }

    /// Short display name for the note.
    pub fn display_title(&self) -> &str {
        self.title
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or("(sin título)")
    }
}

// ─── Note Content ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum NoteContent {
    Text(String),
    Checklist(Vec<ChecklistItem>),
}

impl NoteContent {
    /// Returns a plain-text preview, truncated to `max_chars`.
    pub fn text_preview(&self, max_chars: usize) -> String {
        match self {
            NoteContent::Text(s) => {
                let trimmed = s.trim();
                if trimmed.chars().count() <= max_chars {
                    trimmed.to_owned()
                } else {
                    let truncated: String = trimmed.chars().take(max_chars).collect();
                    format!("{truncated}…")
                }
            }
            NoteContent::Checklist(items) => {
                let total = items.len();
                let done = items.iter().filter(|i| i.checked).count();
                let preview: String = items
                    .iter()
                    .take(3)
                    .map(|i| i.text.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{done}/{total}] {preview}")
            }
        }
    }

    /// Returns the full plain text (used for FTS indexing).
    pub fn as_plain_text(&self) -> String {
        match self {
            NoteContent::Text(s) => s.clone(),
            NoteContent::Checklist(items) => {
                items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join(" ")
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            NoteContent::Text(s) => s.trim().is_empty(),
            NoteContent::Checklist(items) => items.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChecklistItem {
    pub text: String,
    pub checked: bool,
}

// ─── Visual / Color ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoteColor {
    #[default]
    Default,
    Red,
    Orange,
    Yellow,
    Green,
    Teal,
    Blue,
    DarkBlue,
    Purple,
    Pink,
    Brown,
    Gray,
}

// ─── Labels ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Label {
    pub id: String,
    pub name: String,
}

// ─── Sync State ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncState {
    /// Note exists only locally; has never been synced with a remote provider.
    LocalOnly,
    /// Note matches the remote version at `at`.
    Synced { at: DateTime<Utc> },
    /// Local edits not yet pushed to the provider (V2 — write support).
    LocalAhead,
    /// Remote has updates not yet pulled locally.
    RemoteAhead,
    /// Both sides changed and the conflict needs resolution.
    /// Stores the remote version so the user can compare and choose.
    Conflict {
        remote_title: Option<String>,
        remote_content: NoteContent,
        remote_updated_at: DateTime<Utc>,
    },
    /// Last sync attempt failed.
    SyncError { message: String, retries: u32 },
}

impl SyncState {
    pub fn is_conflict(&self) -> bool {
        matches!(self, SyncState::Conflict { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, SyncState::SyncError { .. })
    }

    pub fn with_error(message: impl Into<String>) -> Self {
        SyncState::SyncError { message: message.into(), retries: 1 }
    }

    pub fn increment_retries(self) -> Self {
        match self {
            SyncState::SyncError { message, retries } => {
                SyncState::SyncError { message, retries: retries + 1 }
            }
            _ => SyncState::SyncError { message: "Sync failed".to_string(), retries: 1 },
        }
    }
}

// ─── Layout / Visual State ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteLayout {
    /// Pixel position on screen. None = auto-positioned.
    pub position: Option<Point>,
    /// Override size. None = default card size.
    pub size: Option<Size>,
    /// Whether the note card is currently open on screen.
    pub visible: bool,
    /// Per-note always-on-top override (off by default).
    pub always_on_top: bool,
    /// Stacking order within visible notes.
    pub z_order: i32,
}

impl Default for NoteLayout {
    fn default() -> Self {
        Self {
            position: None,
            size: None,
            visible: false,
            always_on_top: false,
            z_order: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_preview_short_returns_full() {
        let c = NoteContent::Text("hola mundo".to_string());
        assert_eq!(c.text_preview(100), "hola mundo");
    }

    #[test]
    fn text_preview_truncates_at_char_boundary() {
        let c = NoteContent::Text("abcde".to_string());
        assert_eq!(c.text_preview(3), "abc…");
    }

    #[test]
    fn checklist_preview_shows_ratio_and_items() {
        let items = vec![
            ChecklistItem { text: "uno".to_string(), checked: true },
            ChecklistItem { text: "dos".to_string(), checked: false },
        ];
        let c = NoteContent::Checklist(items);
        let preview = c.text_preview(100);
        assert!(preview.starts_with("[1/2]"));
        assert!(preview.contains("uno"));
    }

    #[test]
    fn note_content_empty_detection() {
        assert!(NoteContent::Text("  ".to_string()).is_empty());
        assert!(NoteContent::Checklist(vec![]).is_empty());
        assert!(!NoteContent::Text("x".to_string()).is_empty());
    }

    #[test]
    fn sync_state_error_detection() {
        assert!(SyncState::SyncError { message: "oops".into(), retries: 1 }.is_error());
        assert!(!SyncState::LocalOnly.is_error());
        assert!(!SyncState::Synced { at: Utc::now() }.is_error());
        assert!(!SyncState::Conflict {
            remote_title: None,
            remote_content: NoteContent::Text(String::new()),
            remote_updated_at: Utc::now(),
        }.is_error());
    }

    #[test]
    fn sync_state_increment_retries() {
        let s = SyncState::SyncError { message: "timeout".into(), retries: 2 };
        match s.increment_retries() {
            SyncState::SyncError { retries, message } => {
                assert_eq!(retries, 3);
                assert_eq!(message, "timeout");
            }
            _ => panic!("Expected SyncError"),
        }
    }

    #[test]
    fn sync_state_non_error_increment_starts_at_1() {
        let s = SyncState::LocalOnly;
        match s.increment_retries() {
            SyncState::SyncError { retries, .. } => assert_eq!(retries, 1),
            _ => panic!("Expected SyncError"),
        }
    }

    #[test]
    fn note_is_visible_when_not_trashed() {
        let n = Note::new_local(NoteContent::Text("x".into()));
        assert!(n.is_visible());
    }

    #[test]
    fn note_is_not_visible_when_trashed() {
        let mut n = Note::new_local(NoteContent::Text("x".into()));
        n.is_trashed = true;
        assert!(!n.is_visible());
    }

    #[test]
    fn note_display_title_fallback() {
        let n = Note::new_local(NoteContent::Text("x".into()));
        assert_eq!(n.display_title(), "(sin título)");
    }

    #[test]
    fn note_display_title_uses_title_when_set() {
        let mut n = Note::new_local(NoteContent::Text("x".into()));
        n.title = Some("Mi nota".to_string());
        assert_eq!(n.display_title(), "Mi nota");
    }
}
