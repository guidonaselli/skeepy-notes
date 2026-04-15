/// On-disk JSON format for local notes.
///
/// This is the schema for the user-managed `notes.json` file.
/// It's intentionally simple and human-readable.
///
/// Example file:
/// ```json
/// [
///   {
///     "id": "optional-stable-id",
///     "title": "Mi nota",
///     "text": "Contenido de la nota",
///     "color": "yellow",
///     "pinned": true,
///     "tags": ["trabajo", "urgente"]
///   },
///   {
///     "title": "Lista de compras",
///     "checklist": [
///       { "text": "Leche", "checked": false },
///       { "text": "Pan",   "checked": true  }
///     ]
///   }
/// ]
/// ```
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use skeepy_core::{
    ChecklistItem, Label, NoteColor, NoteContent, ProviderError, RemoteNote,
};

// ─── File-level format ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalNotesFile {
    /// Optional file version for future migration support.
    #[serde(default)]
    pub version: u32,
    pub notes: Vec<LocalNote>,
}

impl LocalNotesFile {
    /// Parse from JSON bytes.
    pub fn from_json(data: &[u8]) -> Result<Self, ProviderError> {
        // Try array format (legacy shorthand) first
        if let Ok(notes) = serde_json::from_slice::<Vec<LocalNote>>(data) {
            return Ok(Self { version: 1, notes });
        }
        // Then try the full object format
        serde_json::from_slice(data)
            .map_err(|e| ProviderError::Api(format!("Invalid notes.json format: {e}")))
    }
}

// ─── Per-note format ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalNote {
    /// Optional stable ID. If absent, a deterministic ID is derived from title+text.
    pub id: Option<String>,
    pub title: Option<String>,
    /// Plain text content. Mutually exclusive with `checklist`.
    pub text: Option<String>,
    /// Checklist content. Takes precedence over `text` if both are present.
    pub checklist: Option<Vec<LocalChecklistItem>>,
    #[serde(default)]
    pub color: LocalColor,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    /// ISO 8601 timestamp. Defaults to the file's modification time if absent.
    pub updated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalChecklistItem {
    pub text: String,
    #[serde(default)]
    pub checked: bool,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LocalColor {
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

// ─── Conversion to domain types ───────────────────────────────────────────────

impl LocalNote {
    /// Convert to a `RemoteNote` for ingestion by the sync engine.
    pub fn into_remote(self, file_mtime: DateTime<Utc>) -> RemoteNote {
        let updated_at = self.updated_at.unwrap_or(file_mtime);
        let created_at = self.created_at.unwrap_or(updated_at);

        let source_id = self.id.unwrap_or_else(|| {
            // Deterministic ID based on title+text so repeated reads are idempotent.
            let key = format!(
                "local:{}:{}",
                self.title.as_deref().unwrap_or(""),
                self.text.as_deref().unwrap_or("")
            );
            format!("{:x}", md5_hex(&key))
        });

        let content = if let Some(items) = self.checklist {
            NoteContent::Checklist(
                items
                    .into_iter()
                    .map(|i| ChecklistItem { text: i.text, checked: i.checked })
                    .collect(),
            )
        } else {
            NoteContent::Text(self.text.unwrap_or_default())
        };

        let labels: Vec<Label> = self
            .tags
            .into_iter()
            .map(|tag| Label { id: tag.clone(), name: tag })
            .collect();

        RemoteNote {
            source_id,
            title: self.title,
            content,
            labels,
            color: map_color(self.color),
            is_pinned: self.pinned,
            is_archived: self.archived,
            is_trashed: false,
            created_at,
            updated_at,
        }
    }
}

fn map_color(c: LocalColor) -> NoteColor {
    match c {
        LocalColor::Default  => NoteColor::Default,
        LocalColor::Red      => NoteColor::Red,
        LocalColor::Orange   => NoteColor::Orange,
        LocalColor::Yellow   => NoteColor::Yellow,
        LocalColor::Green    => NoteColor::Green,
        LocalColor::Teal     => NoteColor::Teal,
        LocalColor::Blue     => NoteColor::Blue,
        LocalColor::DarkBlue => NoteColor::DarkBlue,
        LocalColor::Purple   => NoteColor::Purple,
        LocalColor::Pink     => NoteColor::Pink,
        LocalColor::Brown    => NoteColor::Brown,
        LocalColor::Gray     => NoteColor::Gray,
    }
}

/// Tiny deterministic hash for source_id generation — avoids pulling in `md5` crate.
/// Not cryptographic, just stable for the same input.
fn md5_hex(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
