use tauri::State;

use skeepy_core::{Note, NoteId, NoteLayout, NoteSearchResult};

use crate::state::AppState;

#[tauri::command]
pub async fn notes_get_all(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    state.notes_repo.find_all().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn notes_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<NoteSearchResult>, String> {
    let q = query.trim().to_string();
    if q.is_empty() {
        return Ok(vec![]);
    }
    state
        .notes_repo
        .search_fts(&q, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn notes_update_layout(
    state: State<'_, AppState>,
    id: NoteId,
    layout: NoteLayout,
) -> Result<(), String> {
    state
        .notes_repo
        .update_layout(&id, &layout)
        .await
        .map_err(|e| e.to_string())
}
