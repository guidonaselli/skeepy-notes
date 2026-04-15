use serde::Serialize;
use tauri::State;

use skeepy_core::{Note, NoteContent, NoteId, SyncState, UpdateNoteRequest};

use crate::state::AppState;

// ─── Response types ───────────────────────────────────────────────────────────

/// The two sides of a conflict, ready to render in the UI.
#[derive(Debug, Serialize)]
pub struct ConflictInfo {
    /// Local version (what the user edited in Skeepy).
    pub local_title: Option<String>,
    pub local_content_text: String,
    pub local_updated_at: String,

    /// Remote version (what the provider has).
    pub remote_title: Option<String>,
    pub remote_content_text: String,
    pub remote_updated_at: String,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Return both sides of a conflict for the given note.
/// Fails if the note is not in Conflict state.
#[tauri::command]
pub async fn note_get_conflict(
    state: State<'_, AppState>,
    id: NoteId,
) -> Result<ConflictInfo, String> {
    let note = state
        .notes_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {id} not found"))?;

    match note.sync_state {
        SyncState::Conflict { remote_title, remote_content, remote_updated_at } => {
            Ok(ConflictInfo {
                local_title: note.title,
                local_content_text: content_to_text(&note.content),
                local_updated_at: note.updated_at.to_rfc3339(),
                remote_title,
                remote_content_text: content_to_text(&remote_content),
                remote_updated_at: remote_updated_at.to_rfc3339(),
            })
        }
        _ => Err(format!("Note {id} is not in conflict state")),
    }
}

/// Resolve a conflict by choosing which version to keep.
///
/// - `keep = "local"`:  push the local version to the provider, then mark Synced.
/// - `keep = "remote"`: accept the remote version locally, mark Synced.
#[tauri::command]
pub async fn note_resolve_conflict(
    state: State<'_, AppState>,
    id: NoteId,
    keep: String,
) -> Result<Note, String> {
    let note = state
        .notes_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {id} not found"))?;

    let (remote_title, remote_content, remote_updated_at) = match &note.sync_state {
        SyncState::Conflict { remote_title, remote_content, remote_updated_at } => {
            (remote_title.clone(), remote_content.clone(), *remote_updated_at)
        }
        _ => return Err(format!("Note {id} is not in conflict state")),
    };

    let now = chrono::Utc::now();

    match keep.as_str() {
        "local" => {
            // Push local content to the provider.
            let providers = state.providers.read().await;
            let provider = providers
                .iter()
                .find(|p| p.id() == note.provider_id)
                .ok_or_else(|| format!("Provider '{}' not found", note.provider_id))?;

            if !provider.capabilities().can_write {
                return Err(format!(
                    "Provider '{}' does not support writing",
                    note.provider_id
                ));
            }

            let req = UpdateNoteRequest {
                title: note.title.clone(),
                content: note.content.clone(),
                color: Some(note.color.clone()),
            };

            let remote = provider
                .update_note(&note.source_id, req)
                .await
                .map_err(|e| e.to_string())?;

            drop(providers);

            let resolved = Note {
                updated_at: remote.updated_at,
                synced_at: Some(now),
                sync_state: SyncState::Synced { at: now },
                ..note
            };
            state.notes_repo.upsert(&resolved).await.map_err(|e| e.to_string())?;
            Ok(resolved)
        }

        "remote" => {
            // Accept the remote version — overwrite local content.
            let resolved = Note {
                title: remote_title,
                content: remote_content,
                updated_at: remote_updated_at,
                synced_at: Some(now),
                sync_state: SyncState::Synced { at: now },
                ..note
            };
            state.notes_repo.upsert(&resolved).await.map_err(|e| e.to_string())?;
            Ok(resolved)
        }

        _ => Err(format!("Invalid resolution: '{keep}'. Expected 'local' or 'remote'")),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn content_to_text(content: &NoteContent) -> String {
    match content {
        NoteContent::Text(s) => s.clone(),
        NoteContent::Checklist(items) => items
            .iter()
            .map(|i| {
                let mark = if i.checked { "☑" } else { "☐" };
                format!("{} {}", mark, i.text)
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
