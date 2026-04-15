/// Storage ports — these traits are defined in the domain layer and implemented
/// in the `skeepy-storage` crate. The domain never depends on the storage crate.
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::note::{Note, NoteId, NoteLayout};

// ─── Note Repository ──────────────────────────────────────────────────────────

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn find_all(&self) -> Result<Vec<Note>, StorageError>;
    async fn find_by_id(&self, id: &NoteId) -> Result<Option<Note>, StorageError>;
    async fn find_by_provider(&self, provider_id: &str) -> Result<Vec<Note>, StorageError>;

    /// Full-text search via FTS5. Returns at most `limit` results ordered by relevance.
    async fn search_fts(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<NoteSearchResult>, StorageError>;

    /// Insert or update a note (keyed by `note.id`).
    async fn upsert(&self, note: &Note) -> Result<(), StorageError>;

    /// Update only the layout fields — called on drag/resize, not on sync.
    async fn update_layout(&self, id: &NoteId, layout: &NoteLayout) -> Result<(), StorageError>;

    /// Mark a note as trashed without removing it from storage.
    async fn soft_delete(&self, id: &NoteId) -> Result<(), StorageError>;

    /// Lookup by the (provider_id, source_id) pair — used during sync merges.
    async fn find_by_source(
        &self,
        provider_id: &str,
        source_id: &str,
    ) -> Result<Option<Note>, StorageError>;

    /// Retrieve the last sync record for a provider.
    async fn get_provider_sync_state(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderSyncRecord>, StorageError>;

    /// Persist updated sync state for a provider (last sync time, errors, retries).
    async fn update_provider_sync_state(
        &self,
        record: &ProviderSyncRecord,
    ) -> Result<(), StorageError>;
}

// ─── Settings Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_raw(&self, key: &str) -> Result<Option<String>, StorageError>;
    async fn set_raw(&self, key: &str, value: &str) -> Result<(), StorageError>;
}

// ─── Supporting Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSearchResult {
    pub note: Note,
    /// Highlighted excerpt from FTS5 snippet function. May be None for simple backends.
    pub excerpt: Option<String>,
    /// FTS5 rank score (lower = more relevant for SQLite FTS5).
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSyncRecord {
    pub provider_id: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub retry_count: u32,
    pub status: String,
}

impl ProviderSyncRecord {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            last_sync_at: None,
            last_error: None,
            retry_count: 0,
            status: "active".to_string(),
        }
    }
}
