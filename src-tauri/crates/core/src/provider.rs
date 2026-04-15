use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::ProviderError;
use crate::note::{Label, NoteColor, NoteContent};

// ─── Remote Note ──────────────────────────────────────────────────────────────

/// A note as returned by a provider — before it gets a local NoteId assigned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNote {
    /// The provider's native ID for this note.
    pub source_id: String,
    pub title: Option<String>,
    pub content: NoteContent,
    pub labels: Vec<Label>,
    pub color: NoteColor,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─── Provider Trait ───────────────────────────────────────────────────────────

#[async_trait]
pub trait NoteProvider: Send + Sync {
    /// Stable identifier for this provider (e.g. "keep", "local").
    fn id(&self) -> &str;
    /// Human-readable name.
    fn display_name(&self) -> &str;
    /// Current operational status.
    fn status(&self) -> ProviderStatus;
    /// What this provider can and cannot do.
    fn capabilities(&self) -> ProviderCapabilities;

    // ── Auth ──

    /// Initiate or complete the authentication flow.
    async fn authenticate(&mut self) -> Result<(), ProviderError>;
    /// True if the provider currently holds valid credentials.
    async fn is_authenticated(&self) -> bool;
    /// Clear stored credentials.
    async fn revoke_auth(&mut self) -> Result<(), ProviderError>;

    // ── Read ──

    /// Fetch notes from the provider.
    /// `since`: if the provider supports incremental sync, only return notes
    /// updated after this timestamp. `None` = full fetch.
    async fn fetch_notes(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<RemoteNote>, ProviderError>;

    async fn fetch_note(&self, source_id: &str) -> Result<RemoteNote, ProviderError>;

    // ── Write (optional — default to NotSupported) ────────────────────────────

    async fn create_note(
        &self,
        _req: CreateNoteRequest,
    ) -> Result<RemoteNote, ProviderError> {
        Err(ProviderError::NotSupported { operation: "create_note".into() })
    }

    /// Update an existing note's title and/or content.
    async fn update_note(
        &self,
        _source_id: &str,
        _req: UpdateNoteRequest,
    ) -> Result<RemoteNote, ProviderError> {
        Err(ProviderError::NotSupported { operation: "update_note".into() })
    }

    async fn delete_note(&self, _source_id: &str) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported { operation: "delete_note".into() })
    }
}

// ─── Provider Capabilities ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
    pub supports_labels: bool,
    pub supports_colors: bool,
    pub supports_checklists: bool,
    /// Provider supports `since` parameter in `fetch_notes` for delta syncs.
    pub supports_incremental_sync: bool,
    pub stability: ProviderStability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStability {
    /// Official API with long-term support expected.
    Stable,
    /// May break; user is warned at setup.
    Experimental,
    /// Being removed; no new users should add this provider.
    Deprecated,
}

// ─── Provider Status ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProviderStatus {
    Active,
    Unauthenticated,
    RateLimited { retry_after: DateTime<Utc> },
    Error { message: String },
    Disabled,
}

impl ProviderStatus {
    /// Returns true only when the provider is ready to sync.
    pub fn is_usable(&self) -> bool {
        matches!(self, ProviderStatus::Active)
    }
}

// ─── Write Requests ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNoteRequest {
    pub title: Option<String>,
    pub content: NoteContent,
    pub color: NoteColor,
    pub is_pinned: bool,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: NoteContent,
    pub color: Option<NoteColor>,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_status_is_usable_only_when_active() {
        assert!(ProviderStatus::Active.is_usable());
        assert!(!ProviderStatus::Unauthenticated.is_usable());
        assert!(!ProviderStatus::Disabled.is_usable());
        assert!(!ProviderStatus::Error { message: "x".into() }.is_usable());
    }
}
