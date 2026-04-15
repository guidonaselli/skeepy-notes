use serde::{Deserialize, Serialize};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

/// Returns Some(UpdateInfo) if a newer version is available, None if up-to-date.
#[tauri::command]
pub async fn updater_check(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version.to_string(),
        notes: u.body.unwrap_or_default(),
    }))
}

/// Downloads and installs the latest update. The app restarts automatically after install.
#[tauri::command]
pub async fn updater_install(app: tauri::AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    if let Some(update) = update {
        update
            .download_and_install(|_downloaded, _total| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
