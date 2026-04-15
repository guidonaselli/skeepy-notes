use std::sync::Arc;

use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use skeepy_core::{
    ChecklistItem, CreateNoteRequest as ProviderCreateRequest, Note, NoteColor, NoteContent,
    NoteId, NoteLayout, NoteRepository, NoteService, SyncState, UpdateNoteRequest,
};

use crate::state::AppState;

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NoteContentRequest {
    Text { content: String },
    Checklist { items: Vec<ChecklistItemRequest> },
}

#[derive(Debug, Deserialize)]
pub struct ChecklistItemRequest {
    pub text: String,
    pub checked: bool,
}

impl From<NoteContentRequest> for NoteContent {
    fn from(req: NoteContentRequest) -> Self {
        match req {
            NoteContentRequest::Text { content } => NoteContent::Text(content),
            NoteContentRequest::Checklist { items } => NoteContent::Checklist(
                items
                    .into_iter()
                    .map(|i| ChecklistItem { text: i.text, checked: i.checked })
                    .collect(),
            ),
        }
    }
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Create a note. `provider_id` defaults to "local".
/// Pass `provider_id = "keep"` to create in Google Keep.
#[tauri::command]
pub async fn note_create(
    state: State<'_, AppState>,
    title: Option<String>,
    content: NoteContentRequest,
    color: Option<NoteColor>,
    provider_id: Option<String>,
) -> Result<Note, String> {
    let provider_id = provider_id.unwrap_or_else(|| "local".to_string());

    if provider_id == "local" {
        let now = chrono::Utc::now();
        let id = Uuid::new_v4();
        let note = Note {
            id,
            source_id: id.to_string(),
            provider_id: "local".to_string(),
            title: title.filter(|t| !t.is_empty()),
            content: content.into(),
            labels: Vec::new(),
            color: color.unwrap_or(NoteColor::Default),
            is_pinned: false,
            is_archived: false,
            is_trashed: false,
            created_at: now,
            updated_at: now,
            synced_at: None,
            sync_state: SyncState::LocalOnly,
            layout: NoteLayout::default(),
        };
        state.notes_repo.upsert(&note).await.map_err(|e| e.to_string())?;
        return Ok(note);
    }

    // Provider-backed create (e.g. Keep)
    let providers = state.providers.read().await;
    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' not found"))?;

    if !provider.capabilities().can_write {
        return Err(format!("Provider '{provider_id}' does not support writing"));
    }

    let req = ProviderCreateRequest {
        title: title.filter(|t| !t.is_empty()),
        content: content.into(),
        color: color.unwrap_or(NoteColor::Default),
        is_pinned: false,
        labels: Vec::new(),
    };

    let remote = provider.create_note(req).await.map_err(|e| e.to_string())?;
    drop(providers);

    // Merge the created note into local storage
    let svc = NoteService::new(Arc::clone(&state.notes_repo) as Arc<dyn NoteRepository>);
    svc.merge_remote(remote.clone(), &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Return the locally stored version
    let stored = state.notes_repo
        .find_by_source(&provider_id, &remote.source_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Note created remotely but not found in local storage")?;

    Ok(stored)
}

/// Update the content and/or title of an existing note.
///
/// - Local notes: direct SQLite update.
/// - Provider-backed notes (e.g. OneNote): delegates to `provider.update_note()`,
///   then syncs the result into local storage.
#[tauri::command]
pub async fn note_update(
    state: State<'_, AppState>,
    id: NoteId,
    title: Option<String>,
    content: NoteContentRequest,
    color: Option<NoteColor>,
) -> Result<Note, String> {
    let existing = state
        .notes_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {id} not found"))?;

    if existing.provider_id == "local" {
        let updated = Note {
            title: title.filter(|t| !t.is_empty()),
            content: content.into(),
            color: color.unwrap_or(existing.color.clone()),
            updated_at: chrono::Utc::now(),
            sync_state: SyncState::LocalOnly,
            ..existing
        };
        state.notes_repo.upsert(&updated).await.map_err(|e| e.to_string())?;
        return Ok(updated);
    }

    // Provider-backed update.
    let providers = state.providers.read().await;
    let provider = providers
        .iter()
        .find(|p| p.id() == existing.provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", existing.provider_id))?;

    if !provider.capabilities().can_write {
        return Err(format!(
            "Provider '{}' does not support writing notes",
            existing.provider_id
        ));
    }

    let new_title = title.filter(|t| !t.is_empty());
    let new_content: NoteContent = content.into();

    // Mark LocalAhead optimistically before attempting the push.
    // If the push fails the note stays LocalAhead so the next sync
    // can detect a conflict when the remote also changed.
    let ahead_note = Note {
        title: new_title.clone(),
        content: new_content.clone(),
        color: color.clone().unwrap_or(existing.color.clone()),
        updated_at: chrono::Utc::now(),
        sync_state: SyncState::LocalAhead,
        ..existing.clone()
    };
    state.notes_repo.upsert(&ahead_note).await.map_err(|e| e.to_string())?;

    let req = UpdateNoteRequest {
        title: new_title,
        content: new_content,
        color,
    };

    let remote = provider
        .update_note(&existing.source_id, req)
        .await
        .map_err(|e| e.to_string())?;

    drop(providers);

    // Push succeeded — mark as Synced with the remote's timestamp.
    let now = chrono::Utc::now();
    let synced_note = Note {
        updated_at: remote.updated_at,
        synced_at: Some(now),
        sync_state: SyncState::Synced { at: now },
        ..ahead_note
    };
    state.notes_repo.upsert(&synced_note).await.map_err(|e| e.to_string())?;

    Ok(synced_note)
}

/// Delete a note. For local notes: soft-delete in SQLite.
/// For provider-backed notes (e.g. Keep): delete via provider API, then soft-delete locally.
#[tauri::command]
pub async fn note_delete(
    state: State<'_, AppState>,
    id: NoteId,
) -> Result<(), String> {
    let existing = state
        .notes_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {id} not found"))?;

    // For non-local providers, try to delete via the provider API first.
    if existing.provider_id != "local" {
        let providers = state.providers.read().await;
        if let Some(provider) = providers.iter().find(|p| p.id() == existing.provider_id) {
            if provider.capabilities().can_delete {
                provider
                    .delete_note(&existing.source_id)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                return Err(format!(
                    "Provider '{}' does not support deleting notes",
                    existing.provider_id
                ));
            }
        }
    }

    // Soft-delete locally regardless (hides from UI)
    state
        .notes_repo
        .soft_delete(&id)
        .await
        .map_err(|e| e.to_string())
}
