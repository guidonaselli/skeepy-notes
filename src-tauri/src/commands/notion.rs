use tauri::{AppHandle, State};
use tracing::info;

use skeepy_core::SettingsRepository;
use skeepy_provider_notion::{build_auth_url, exchange_code, AuthSession, TokenStorage};
use skeepy_provider_notion::provider::NotionProvider;

use crate::state::AppState;

const NOTION_CLIENT_ID: &str = match option_env!("NOTION_CLIENT_ID") {
    Some(v) => v,
    None => "",
};
const NOTION_CLIENT_SECRET: &str = match option_env!("NOTION_CLIENT_SECRET") {
    Some(v) => v,
    None => "",
};

async fn resolve_notion_credentials(
    repo: &dyn SettingsRepository,
) -> Result<(String, String), String> {
    let custom_id = repo.get_raw("notion_client_id").await.ok().flatten();
    match custom_id {
        Some(id) if !id.is_empty() => {
            let secret = repo
                .get_raw("notion_client_secret")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            Ok((id, secret))
        }
        _ => {
            if NOTION_CLIENT_ID.is_empty() {
                return Err(
                    "No Notion credentials configured. Provide NOTION_CLIENT_ID and \
                     NOTION_CLIENT_SECRET at build time, or enter them in Settings → Notion."
                        .to_string(),
                );
            }
            Ok((NOTION_CLIENT_ID.to_string(), NOTION_CLIENT_SECRET.to_string()))
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct NotionAuthInitResponse {
    pub auth_url: String,
    pub state: String,
    pub redirect_uri: String,
}

/// Start the Notion OAuth2 authorization flow.
#[tauri::command]
pub async fn notion_start_auth(
    state: State<'_, AppState>,
    redirect_uri: String,
) -> Result<NotionAuthInitResponse, String> {
    let (client_id, _) = resolve_notion_credentials(state.settings_repo.as_ref()).await?;
    let session = AuthSession::new(redirect_uri);
    let auth_url = build_auth_url(&client_id, &session);
    info!("Notion auth URL generated");

    Ok(NotionAuthInitResponse {
        auth_url,
        state: session.state,
        redirect_uri: session.redirect_uri,
    })
}

/// Complete the Notion OAuth2 flow.
#[tauri::command]
pub async fn notion_complete_auth(
    state: State<'_, AppState>,
    _app: AppHandle,
    code: String,
    redirect_uri: String,
) -> Result<(), String> {
    let (client_id, client_secret) =
        resolve_notion_credentials(state.settings_repo.as_ref()).await?;

    let http = reqwest::Client::new();
    let tokens = exchange_code(&http, &client_id, &client_secret, &code, &redirect_uri)
        .await
        .map_err(|e: skeepy_core::ProviderError| e.to_string())?;

    TokenStorage::save(&tokens).map_err(|e| e.to_string())?;
    info!("Notion tokens saved to keyring");

    // Load default parent page from settings (optional)
    let parent_id = state
        .settings_repo
        .get_raw("notion_parent_page_id")
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty());

    let mut providers = state.providers.write().await;
    let already_registered = providers.iter().any(|p| p.id() == "notion");
    if !already_registered {
        providers.push(Box::new(NotionProvider::new(client_id, client_secret, parent_id)));
        info!("NotionProvider registered in AppState");
    }

    Ok(())
}

/// Revoke Notion authentication and unregister the provider.
#[tauri::command]
pub async fn notion_revoke(state: State<'_, AppState>) -> Result<(), String> {
    let mut providers = state.providers.write().await;
    providers.retain(|p| p.id() != "notion");
    TokenStorage::delete().map_err(|e| e.to_string())?;
    info!("Notion authentication revoked");
    Ok(())
}

/// Check if Notion is authenticated.
#[tauri::command]
pub async fn notion_status() -> Result<bool, String> {
    match TokenStorage::load() {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NotionCredentials {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub parent_page_id: Option<String>,
}

/// Get user-provided Notion credentials override.
#[tauri::command]
pub async fn notion_credentials_get(
    state: State<'_, AppState>,
) -> Result<NotionCredentials, String> {
    let repo = state.settings_repo.as_ref();
    Ok(NotionCredentials {
        client_id: repo.get_raw("notion_client_id").await.ok().flatten().filter(|s| !s.is_empty()),
        client_secret: repo.get_raw("notion_client_secret").await.ok().flatten().filter(|s| !s.is_empty()),
        parent_page_id: repo.get_raw("notion_parent_page_id").await.ok().flatten().filter(|s| !s.is_empty()),
    })
}

/// Persist user-provided Notion credentials.
#[tauri::command]
pub async fn notion_credentials_set(
    state: State<'_, AppState>,
    client_id: Option<String>,
    client_secret: Option<String>,
    parent_page_id: Option<String>,
) -> Result<(), String> {
    let repo = state.settings_repo.as_ref();
    repo.set_raw("notion_client_id", client_id.as_deref().unwrap_or("")).await.map_err(|e| e.to_string())?;
    repo.set_raw("notion_client_secret", client_secret.as_deref().unwrap_or("")).await.map_err(|e| e.to_string())?;
    repo.set_raw("notion_parent_page_id", parent_page_id.as_deref().unwrap_or("")).await.map_err(|e| e.to_string())?;
    Ok(())
}
