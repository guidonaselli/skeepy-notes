use serde::Serialize;
use tauri::State;
use tracing::info;

use crate::state::AppState;

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct LabelInfo {
    pub name: String,
    /// Providers that have at least one note with this label.
    pub providers: Vec<String>,
    /// Total number of notes that carry this label.
    pub note_count: usize,
    /// True if all notes with this label come from the "local" provider.
    pub is_local: bool,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Return all labels that appear in the note database, with aggregated metadata.
#[tauri::command]
pub async fn labels_get_all(state: State<'_, AppState>) -> Result<Vec<LabelInfo>, String> {
    let notes = state.notes_repo.find_all().await.map_err(|e| e.to_string())?;

    // Aggregate: label_name → (providers set, count)
    use std::collections::{BTreeMap, BTreeSet};
    let mut map: BTreeMap<String, (BTreeSet<String>, usize)> = BTreeMap::new();

    for note in notes.iter().filter(|n| n.is_visible()) {
        for label in &note.labels {
            let entry = map.entry(label.name.clone()).or_default();
            entry.0.insert(note.provider_id.clone());
            entry.1 += 1;
        }
    }

    let mut labels: Vec<LabelInfo> = map
        .into_iter()
        .map(|(name, (providers, note_count))| {
            let is_local = providers.iter().all(|p| p == "local");
            LabelInfo { name, providers: providers.into_iter().collect(), note_count, is_local }
        })
        .collect();

    labels.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(labels)
}

/// Rename a label on all LOCAL notes that carry it.
/// Notes from other providers are not modified (their labels are managed remotely).
#[tauri::command]
pub async fn label_rename(
    state: State<'_, AppState>,
    old_name: String,
    new_name: String,
) -> Result<usize, String> {
    if new_name.trim().is_empty() {
        return Err("New label name cannot be empty".to_string());
    }
    if old_name == new_name {
        return Ok(0);
    }

    let notes = state.notes_repo.find_all().await.map_err(|e| e.to_string())?;
    let mut updated = 0;

    for mut note in notes {
        if note.provider_id != "local" { continue; }
        if !note.labels.iter().any(|l| l.name == old_name) { continue; }

        for label in note.labels.iter_mut() {
            if label.name == old_name {
                label.name = new_name.clone();
            }
        }
        note.updated_at = chrono::Utc::now();
        state.notes_repo.upsert(&note).await.map_err(|e| e.to_string())?;
        updated += 1;
    }

    info!(old = %old_name, new = %new_name, count = updated, "Label renamed");
    Ok(updated)
}

/// Remove a label from all LOCAL notes that carry it.
#[tauri::command]
pub async fn label_delete(
    state: State<'_, AppState>,
    name: String,
) -> Result<usize, String> {
    let notes = state.notes_repo.find_all().await.map_err(|e| e.to_string())?;
    let mut updated = 0;

    for mut note in notes {
        if note.provider_id != "local" { continue; }
        let before = note.labels.len();
        note.labels.retain(|l| l.name != name);
        if note.labels.len() < before {
            note.updated_at = chrono::Utc::now();
            state.notes_repo.upsert(&note).await.map_err(|e| e.to_string())?;
            updated += 1;
        }
    }

    info!(label = %name, count = updated, "Label deleted from local notes");
    Ok(updated)
}
