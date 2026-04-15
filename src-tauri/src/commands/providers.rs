use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, State};

use skeepy_core::{ProviderCapabilities, ProviderStatus};

use crate::commands::sync::run_sync_for;
use crate::state::AppState;

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatusInfo {
    pub id: String,
    pub display_name: String,
    pub capabilities: ProviderCapabilities,
    pub status: ProviderStatus,
    /// Timestamp of the last successful sync, if any.
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Error message from the last failed sync, if any.
    pub last_error: Option<String>,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Return the status of every currently registered provider.
#[tauri::command]
pub async fn providers_status(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderStatusInfo>, String> {
    let providers = state.providers.read().await;
    let mut result = Vec::with_capacity(providers.len());

    for provider in providers.iter() {
        let id = provider.id().to_string();

        // Load the last sync record from the DB (may not exist yet).
        let sync_record = state
            .notes_repo
            .get_provider_sync_state(&id)
            .await
            .unwrap_or(None);

        result.push(ProviderStatusInfo {
            id: id.clone(),
            display_name: provider.display_name().to_string(),
            capabilities: provider.capabilities(),
            status: provider.status(),
            last_sync_at: sync_record.as_ref().and_then(|r| r.last_sync_at),
            last_error: sync_record.as_ref().and_then(|r| r.last_error.clone()),
        });
    }

    Ok(result)
}

/// Trigger a sync for a single provider by ID.
/// Emits `sync://progress` just like the full sync.
#[tauri::command]
pub async fn sync_provider(
    state: State<'_, AppState>,
    app: AppHandle,
    provider_id: String,
) -> Result<(), String> {
    run_sync_for(&state, &app, &provider_id)
        .await
        .map_err(|e| e.to_string())
}
