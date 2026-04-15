use tauri::State;

use skeepy_core::AppSettings;

use crate::state::AppState;

const SETTINGS_KEY: &str = "app_settings";

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let raw = state
        .settings_repo
        .get_raw(SETTINGS_KEY)
        .await
        .map_err(|e| e.to_string())?;

    match raw {
        Some(json) => serde_json::from_str(&json).map_err(|e| e.to_string()),
        None => Ok(AppSettings::default()),
    }
}

#[tauri::command]
pub async fn settings_set(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    state
        .settings_repo
        .set_raw(SETTINGS_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}
