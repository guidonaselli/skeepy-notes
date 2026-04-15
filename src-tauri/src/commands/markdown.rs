use tauri::State;

use crate::state::AppState;

/// Get the currently configured Markdown folder path.
/// Returns `None` if no folder has been configured.
#[tauri::command]
pub async fn markdown_get_folder(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let lock = state
        .markdown_folder
        .read()
        .map_err(|_| "RwLock poisoned".to_string())?;

    Ok(lock.as_ref().map(|p| p.to_string_lossy().to_string()))
}

/// Set (or clear) the Markdown folder path.
/// Pass `None` to disable the provider.
/// The path is persisted in settings so it survives app restarts.
#[tauri::command]
pub async fn markdown_set_folder(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<(), String> {
    let new_path = path.as_deref().filter(|s| !s.is_empty()).map(std::path::PathBuf::from);

    // Update the live provider handle (immediately affects the next sync)
    {
        let mut lock = state
            .markdown_folder
            .write()
            .map_err(|_| "RwLock poisoned".to_string())?;
        *lock = new_path.clone();
    }

    // Persist to settings
    let value = new_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    state
        .settings_repo
        .set_raw("markdown_folder_path", &value)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
