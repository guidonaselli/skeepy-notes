use serde::Serialize;
use tauri::State;

use skeepy_core::{Note, NoteRepository};

use crate::semantic::indexer;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct SemanticSearchResult {
    pub note: Note,
    pub score: f32,
}

/// Semantic search using the local TF-IDF index.
///
/// Returns up to `limit` notes ordered by cosine similarity to `query`.
/// Falls back to an empty list if the index hasn't been built yet.
#[tauri::command]
pub async fn notes_search_semantic(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SemanticSearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let limit = limit.unwrap_or(10).min(50);
    let scores = indexer::search(&state, &query, limit);

    if scores.is_empty() {
        return Ok(vec![]);
    }

    // Fetch the actual Note objects for each matching ID.
    let mut results = Vec::with_capacity(scores.len());
    for (note_id, score) in scores {
        let id = note_id
            .parse::<uuid::Uuid>()
            .map_err(|e| e.to_string())?;
        if let Ok(Some(note)) = state.notes_repo.find_by_id(&id).await {
            results.push(SemanticSearchResult { note, score });
        }
    }

    Ok(results)
}

/// Trigger background re-indexing of all notes.
/// Call this after a sync or when the user explicitly requests it.
#[tauri::command]
pub async fn semantic_index_rebuild(state: State<'_, AppState>) -> Result<(), String> {
    indexer::index_in_background(state.db.clone());
    Ok(())
}
