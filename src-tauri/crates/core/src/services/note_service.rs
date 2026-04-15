use std::sync::Arc;

use chrono::Utc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::CoreError;
use crate::note::{Note, NoteId, NoteLayout, SyncState};
use crate::provider::RemoteNote;
use crate::repository::{NoteRepository, NoteSearchResult, ProviderSyncRecord};

pub struct NoteService {
    repository: Arc<dyn NoteRepository>,
}

impl NoteService {
    pub fn new(repository: Arc<dyn NoteRepository>) -> Self {
        Self { repository }
    }

    // ── Queries ────────────────────────────────────────────────────────────────

    pub async fn get_all_visible(&self) -> Result<Vec<Note>, CoreError> {
        let notes = self.repository.find_all().await?;
        Ok(notes.into_iter().filter(|n| n.is_visible()).collect())
    }

    pub async fn get_by_id(&self, id: &NoteId) -> Result<Option<Note>, CoreError> {
        Ok(self.repository.find_by_id(id).await?)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<NoteSearchResult>, CoreError> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(vec![]);
        }
        Ok(self.repository.search_fts(q, 50).await?)
    }

    pub async fn get_by_provider(&self, provider_id: &str) -> Result<Vec<Note>, CoreError> {
        Ok(self.repository.find_by_provider(provider_id).await?)
    }

    // ── Mutations ──────────────────────────────────────────────────────────────

    pub async fn update_layout(
        &self,
        id: &NoteId,
        layout: &NoteLayout,
    ) -> Result<(), CoreError> {
        Ok(self.repository.update_layout(id, layout).await?)
    }

    pub async fn soft_delete(&self, id: &NoteId) -> Result<(), CoreError> {
        Ok(self.repository.soft_delete(id).await?)
    }

    // ── Sync Merge ─────────────────────────────────────────────────────────────

    /// Merge a remote note into local storage.
    ///
    /// Merge rules (V1 — pull-only):
    /// - New remote note   → insert as Synced
    /// - Remote is newer   → update content, keep layout
    /// - Remote is same/older → no-op
    /// - Local has unsent changes (LocalAhead/Conflict) → no-op (V2 will handle)
    ///
    /// Returns `true` if a write occurred (for progress reporting).
    pub async fn merge_remote(
        &self,
        remote: RemoteNote,
        provider_id: &str,
    ) -> Result<bool, CoreError> {
        let existing = self
            .repository
            .find_by_source(provider_id, &remote.source_id)
            .await?;

        match existing {
            None => {
                debug!(
                    provider = %provider_id,
                    source_id = %remote.source_id,
                    "Inserting new note from provider"
                );
                let note = self.remote_to_note(remote, provider_id, None);
                self.repository.upsert(&note).await?;
                Ok(true)
            }

            Some(local) => {
                // Clone the sync state so we can still use `local` in the struct update.
                let state = local.sync_state.clone();
                let local_updated_at = local.updated_at;

                match state {
                    // Safe to update from remote
                    SyncState::Synced { .. }
                    | SyncState::RemoteAhead
                    | SyncState::SyncError { .. } => {
                        if remote.updated_at > local_updated_at {
                            debug!(
                                provider = %provider_id,
                                source_id = %remote.source_id,
                                "Updating note from provider (remote is newer)"
                            );
                            let updated = Note {
                                title: remote.title,
                                content: remote.content,
                                labels: remote.labels,
                                color: remote.color,
                                is_pinned: remote.is_pinned,
                                is_archived: remote.is_archived,
                                is_trashed: remote.is_trashed,
                                updated_at: remote.updated_at,
                                synced_at: Some(Utc::now()),
                                sync_state: SyncState::Synced { at: Utc::now() },
                                // Preserve: id, source_id, provider_id, created_at, layout
                                ..local
                            };
                            self.repository.upsert(&updated).await?;
                            Ok(true)
                        } else {
                            debug!(
                                provider = %provider_id,
                                source_id = %remote.source_id,
                                "Skipping note — local is up-to-date"
                            );
                            Ok(false)
                        }
                    }

                    // Local has unsent changes — check if remote also changed (conflict).
                    SyncState::LocalOnly | SyncState::LocalAhead => {
                        if remote.updated_at > local_updated_at {
                            warn!(
                                provider = %provider_id,
                                source_id = %remote.source_id,
                                "Conflict detected: both local and remote changed"
                            );
                            let conflicted = Note {
                                sync_state: SyncState::Conflict {
                                    remote_title: remote.title,
                                    remote_content: remote.content,
                                    remote_updated_at: remote.updated_at,
                                },
                                ..local
                            };
                            self.repository.upsert(&conflicted).await?;
                            Ok(true)
                        } else {
                            warn!(
                                provider = %provider_id,
                                source_id = %remote.source_id,
                                state = ?local.sync_state,
                                "Skipping merge — local is ahead, remote not newer"
                            );
                            Ok(false)
                        }
                    }

                    // Already in conflict — don't overwrite until user resolves.
                    SyncState::Conflict { .. } => {
                        warn!(
                            provider = %provider_id,
                            source_id = %remote.source_id,
                            "Skipping merge — conflict pending user resolution"
                        );
                        Ok(false)
                    }
                }
            }
        }
    }

    // ── Provider Sync State ────────────────────────────────────────────────────

    pub async fn get_provider_sync_state(
        &self,
        provider_id: &str,
    ) -> Result<ProviderSyncRecord, CoreError> {
        Ok(self
            .repository
            .get_provider_sync_state(provider_id)
            .await?
            .unwrap_or_else(|| ProviderSyncRecord::new(provider_id)))
    }

    pub async fn update_provider_sync_state(
        &self,
        record: &ProviderSyncRecord,
    ) -> Result<(), CoreError> {
        Ok(self.repository.update_provider_sync_state(record).await?)
    }

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn remote_to_note(
        &self,
        remote: RemoteNote,
        provider_id: &str,
        existing_id: Option<Uuid>,
    ) -> Note {
        Note {
            id: existing_id.unwrap_or_else(Uuid::new_v4),
            source_id: remote.source_id,
            provider_id: provider_id.to_string(),
            title: remote.title,
            content: remote.content,
            labels: remote.labels,
            color: remote.color,
            is_pinned: remote.is_pinned,
            is_archived: remote.is_archived,
            is_trashed: remote.is_trashed,
            created_at: remote.created_at,
            updated_at: remote.updated_at,
            synced_at: Some(Utc::now()),
            sync_state: SyncState::Synced { at: Utc::now() },
            layout: crate::note::NoteLayout::default(),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

    use crate::error::StorageError;
    use crate::note::{NoteColor, NoteContent};
    use crate::provider::RemoteNote;
    use crate::repository::{NoteSearchResult, ProviderSyncRecord};

    // ── In-memory mock repository ──────────────────────────────────────────────

    #[derive(Default)]
    struct MemRepo {
        notes: Mutex<HashMap<NoteId, Note>>,
        sync_state: Mutex<HashMap<String, ProviderSyncRecord>>,
    }

    #[async_trait]
    impl NoteRepository for MemRepo {
        async fn find_all(&self) -> Result<Vec<Note>, StorageError> {
            Ok(self.notes.lock().unwrap().values().cloned().collect())
        }

        async fn find_by_id(&self, id: &NoteId) -> Result<Option<Note>, StorageError> {
            Ok(self.notes.lock().unwrap().get(id).cloned())
        }

        async fn find_by_provider(&self, pid: &str) -> Result<Vec<Note>, StorageError> {
            Ok(self
                .notes
                .lock()
                .unwrap()
                .values()
                .filter(|n| n.provider_id == pid)
                .cloned()
                .collect())
        }

        async fn search_fts(&self, _q: &str, _limit: u32) -> Result<Vec<NoteSearchResult>, StorageError> {
            Ok(vec![])
        }

        async fn upsert(&self, note: &Note) -> Result<(), StorageError> {
            self.notes.lock().unwrap().insert(note.id, note.clone());
            Ok(())
        }

        async fn update_layout(&self, id: &NoteId, layout: &NoteLayout) -> Result<(), StorageError> {
            if let Some(n) = self.notes.lock().unwrap().get_mut(id) {
                n.layout = layout.clone();
            }
            Ok(())
        }

        async fn soft_delete(&self, id: &NoteId) -> Result<(), StorageError> {
            if let Some(n) = self.notes.lock().unwrap().get_mut(id) {
                n.is_trashed = true;
            }
            Ok(())
        }

        async fn find_by_source(&self, pid: &str, sid: &str) -> Result<Option<Note>, StorageError> {
            Ok(self
                .notes
                .lock()
                .unwrap()
                .values()
                .find(|n| n.provider_id == pid && n.source_id == sid)
                .cloned())
        }

        async fn get_provider_sync_state(&self, pid: &str) -> Result<Option<ProviderSyncRecord>, StorageError> {
            Ok(self.sync_state.lock().unwrap().get(pid).cloned())
        }

        async fn update_provider_sync_state(&self, r: &ProviderSyncRecord) -> Result<(), StorageError> {
            self.sync_state.lock().unwrap().insert(r.provider_id.clone(), r.clone());
            Ok(())
        }
    }

    fn make_remote(source_id: &str, text: &str, updated_at: DateTime<Utc>) -> RemoteNote {
        RemoteNote {
            source_id: source_id.to_string(),
            title: None,
            content: NoteContent::Text(text.to_string()),
            labels: vec![],
            color: NoteColor::Default,
            is_pinned: false,
            is_archived: false,
            is_trashed: false,
            created_at: updated_at,
            updated_at,
        }
    }

    fn svc() -> NoteService {
        NoteService::new(Arc::new(MemRepo::default()))
    }

    #[tokio::test]
    async fn merge_inserts_new_note() {
        let svc = svc();
        let remote = make_remote("keep-1", "hola", Utc::now());
        let wrote = svc.merge_remote(remote, "keep").await.unwrap();
        assert!(wrote);

        let notes = svc.get_all_visible().await.unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].source_id, "keep-1");
        assert_eq!(notes[0].provider_id, "keep");
    }

    #[tokio::test]
    async fn merge_updates_when_remote_is_newer() {
        let svc = svc();
        let t0 = Utc::now();

        // Insert initial
        svc.merge_remote(make_remote("keep-1", "viejo", t0), "keep").await.unwrap();

        // Remote has a newer version
        let t1 = t0 + chrono::Duration::seconds(60);
        let wrote = svc.merge_remote(make_remote("keep-1", "nuevo", t1), "keep").await.unwrap();
        assert!(wrote);

        let notes = svc.get_all_visible().await.unwrap();
        assert_eq!(notes.len(), 1);
        match &notes[0].content {
            NoteContent::Text(t) => assert_eq!(t, "nuevo"),
            _ => panic!("Expected Text content"),
        }
    }

    #[tokio::test]
    async fn merge_noop_when_remote_is_older() {
        let svc = svc();
        let t0 = Utc::now();
        svc.merge_remote(make_remote("keep-1", "actual", t0), "keep").await.unwrap();

        let t_older = t0 - chrono::Duration::seconds(60);
        let wrote = svc.merge_remote(make_remote("keep-1", "viejo", t_older), "keep").await.unwrap();
        assert!(!wrote);

        let notes = svc.get_all_visible().await.unwrap();
        match &notes[0].content {
            NoteContent::Text(t) => assert_eq!(t, "actual"),
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn merge_preserves_layout() {
        let svc = svc();
        let t0 = Utc::now();
        svc.merge_remote(make_remote("keep-1", "hola", t0), "keep").await.unwrap();

        // Set a custom layout
        let notes = svc.get_all_visible().await.unwrap();
        let id = notes[0].id;
        let layout = NoteLayout {
            position: Some(crate::note::Point { x: 100.0, y: 200.0 }),
            size: Some(crate::note::Size { width: 300.0, height: 200.0 }),
            visible: true,
            always_on_top: false,
            z_order: 1,
        };
        svc.update_layout(&id, &layout).await.unwrap();

        // Merge a newer version from remote
        let t1 = t0 + chrono::Duration::seconds(60);
        svc.merge_remote(make_remote("keep-1", "actualizado", t1), "keep").await.unwrap();

        // Layout must be preserved
        let notes = svc.get_all_visible().await.unwrap();
        let pos = notes[0].layout.position.unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 200.0);
    }

    #[tokio::test]
    async fn search_empty_query_returns_empty() {
        let svc = svc();
        let result = svc.search("").await.unwrap();
        assert!(result.is_empty());
        let result = svc.search("   ").await.unwrap();
        assert!(result.is_empty());
    }
}
