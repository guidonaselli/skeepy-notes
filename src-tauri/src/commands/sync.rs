use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::{error, info, warn};

use skeepy_core::{BackoffConfig, NoteRepository, NoteService, ProviderSyncRecord};

use crate::state::AppState;

/// Payload emitted on the `sync://progress` event after each provider run.
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgressEvent {
    pub provider_id: String,
    /// "ok" | "error"
    pub status: String,
    pub notes_synced: usize,
    pub error: Option<String>,
}

/// IPC command: kick off a full sync cycle across all registered providers.
#[tauri::command]
pub async fn sync_trigger(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    run_sync(&state, &app).await.map_err(|e| e.to_string())
}

/// Called from both the IPC command and the tray "Sincronizar ahora" item.
/// Runs all registered providers.
pub async fn run_sync(
    state: &AppState,
    app: &AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let providers = state.providers.read().await;
    let ids: Vec<String> = providers.iter().map(|p| p.id().to_string()).collect();
    drop(providers);

    for id in ids {
        sync_one(state, app, &id).await;
    }

    Ok(())
}

/// Sync a single provider by ID. No-ops silently if the ID is not registered.
pub async fn run_sync_for(
    state: &AppState,
    app: &AppHandle,
    provider_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sync_one(state, app, provider_id).await;
    Ok(())
}

const MAX_RETRIES: u32 = 5;

/// Core sync logic for a single provider — fetches notes and merges them into SQLite.
/// Applies exponential backoff: if the provider has failed `MAX_RETRIES` times in a row,
/// it is skipped until the backoff delay has elapsed.
async fn sync_one(state: &AppState, app: &AppHandle, provider_id: &str) {
    let svc = NoteService::new(Arc::clone(&state.notes_repo) as Arc<dyn NoteRepository>);

    // Load current sync state so we can apply backoff.
    let prev_state = svc
        .get_provider_sync_state(provider_id)
        .await
        .unwrap_or_else(|_| ProviderSyncRecord::new(provider_id));

    // ── Backoff check ─────────────────────────────────────────────────────────
    // If the provider has been retried MAX_RETRIES times, skip it until enough
    // time has passed (exponential backoff from BackoffConfig::default()).
    if prev_state.retry_count >= MAX_RETRIES {
        let backoff = BackoffConfig::default();
        let required_delay = backoff.delay_for_attempt(prev_state.retry_count);
        if let Some(last_attempt) = prev_state.last_sync_at.or_else(|| {
            // Fall back to "now minus some time" if no record exists
            Some(Utc::now() - chrono::Duration::seconds(1))
        }) {
            let elapsed = Utc::now()
                .signed_duration_since(last_attempt)
                .to_std()
                .unwrap_or(Duration::ZERO);
            if elapsed < required_delay {
                warn!(
                    provider = %provider_id,
                    retries = prev_state.retry_count,
                    "Skipping sync — in backoff period ({:.0}s remaining)",
                    (required_delay - elapsed).as_secs_f32()
                );
                return;
            }
        }
    }

    let providers = state.providers.read().await;
    let Some(provider) = providers.iter().find(|p| p.id() == provider_id) else {
        return;
    };

    info!(provider = %provider_id, "Starting sync");

    match provider.fetch_notes(None).await {
        Ok(remote_notes) => {
            let mut synced = 0usize;
            for remote in remote_notes {
                match svc.merge_remote(remote, provider_id).await {
                    Ok(true) => synced += 1,
                    Ok(false) => {}
                    Err(e) => error!(provider = %provider_id, error = %e, "merge_remote failed"),
                }
            }

            // Reset error state on success.
            let record = ProviderSyncRecord {
                provider_id: provider_id.to_string(),
                last_sync_at: Some(Utc::now()),
                last_error: None,
                retry_count: 0,
                status: "active".to_string(),
            };
            let _ = svc.update_provider_sync_state(&record).await;

            let _ = app.emit(
                "sync://progress",
                SyncProgressEvent {
                    provider_id: provider_id.to_string(),
                    status: "ok".to_string(),
                    notes_synced: synced,
                    error: None,
                },
            );
            info!(provider = %provider_id, synced, "Sync complete");
        }

        Err(e) => {
            error!(provider = %provider_id, error = %e, "Fetch failed");

            // Increment retry counter, preserve last successful sync time.
            let record = ProviderSyncRecord {
                provider_id: provider_id.to_string(),
                last_sync_at: prev_state.last_sync_at, // keep last successful
                last_error: Some(e.to_string()),
                retry_count: prev_state.retry_count.saturating_add(1),
                status: "error".to_string(),
            };
            let _ = svc.update_provider_sync_state(&record).await;

            let _ = app.emit(
                "sync://progress",
                SyncProgressEvent {
                    provider_id: provider_id.to_string(),
                    status: "error".to_string(),
                    notes_synced: 0,
                    error: Some(e.to_string()),
                },
            );
        }
    }
}
