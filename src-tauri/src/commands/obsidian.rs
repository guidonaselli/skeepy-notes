use tauri::State;

use crate::state::AppState;

/// Get the currently configured Obsidian vault path.
/// Returns `None` if no vault has been configured.
#[tauri::command]
pub async fn obsidian_get_vault(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let lock = state
        .obsidian_vault
        .read()
        .map_err(|_| "RwLock poisoned".to_string())?;

    Ok(lock.as_ref().map(|p| p.to_string_lossy().to_string()))
}

/// Set (or clear) the Obsidian vault path.
/// Pass `None` to disable the provider.
/// The path is persisted in settings so it survives app restarts.
#[tauri::command]
pub async fn obsidian_set_vault(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<(), String> {
    let new_path = path.as_deref().filter(|s| !s.is_empty()).map(std::path::PathBuf::from);

    {
        let mut lock = state
            .obsidian_vault
            .write()
            .map_err(|_| "RwLock poisoned".to_string())?;
        *lock = new_path.clone();
    }

    let value = new_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    state
        .settings_repo
        .set_raw("obsidian_vault_path", &value)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
