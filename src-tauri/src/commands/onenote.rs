use tauri::{AppHandle, State};
use tracing::info;

use skeepy_core::SettingsRepository;
use skeepy_provider_onenote::{build_auth_url, PkceSession, TokenStorage};
use skeepy_provider_onenote::provider::OneNoteProvider;

use crate::state::AppState;

// Azure AD app registration — compiled in at build time.
const ONENOTE_CLIENT_ID: &str = match option_env!("AZURE_CLIENT_ID") {
    Some(v) => v,
    None => "",
};

async fn resolve_onenote_client_id(
    repo: &dyn SettingsRepository,
) -> Result<String, String> {
    let custom = repo.get_raw("onenote_client_id").await.ok().flatten();
    match custom {
        Some(id) if !id.is_empty() => Ok(id),
        _ => {
            if ONENOTE_CLIENT_ID.is_empty() {
                return Err(
                    "No OneNote credentials configured. Provide AZURE_CLIENT_ID at build \
                     time or enter your own Azure App ID in Settings → OneNote."
                        .to_string(),
                );
            }
            Ok(ONENOTE_CLIENT_ID.to_string())
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct OneNoteAuthInitResponse {
    pub auth_url: String,
    pub code_verifier: String,
    pub state: String,
    pub redirect_uri: String,
}

/// Start the Microsoft OAuth2 PKCE flow.
///
/// Returns the authorization URL the frontend should open in a browser.
#[tauri::command]
pub async fn onenote_start_auth(
    state: State<'_, AppState>,
    redirect_uri: String,
) -> Result<OneNoteAuthInitResponse, String> {
    let client_id = resolve_onenote_client_id(state.settings_repo.as_ref()).await?;
    let session = PkceSession::new(redirect_uri);
    let auth_url = build_auth_url(&client_id, &session);
    info!("OneNote auth URL generated");

    Ok(OneNoteAuthInitResponse {
        auth_url,
        code_verifier: session.code_verifier,
        state: session.state,
        redirect_uri: session.redirect_uri,
    })
}

/// Complete the OAuth2 flow by exchanging the authorization code for tokens.
#[tauri::command]
pub async fn onenote_complete_auth(
    state: State<'_, AppState>,
    _app: AppHandle,
    code: String,
    code_verifier: String,
    redirect_uri: String,
) -> Result<(), String> {
    use skeepy_provider_onenote::auth::exchange_code;

    let client_id = resolve_onenote_client_id(state.settings_repo.as_ref()).await?;
    let http = reqwest::Client::new();
    let session = PkceSession { code_verifier, state: String::new(), redirect_uri };

    let tokens = exchange_code(&http, &client_id, &code, &session)
        .await
        .map_err(|e| e.to_string())?;

    TokenStorage::save(&tokens).map_err(|e| e.to_string())?;
    info!("OneNote tokens saved to keyring");

    let mut providers = state.providers.write().await;
    let already_registered = providers.iter().any(|p| p.id() == "onenote");
    if !already_registered {
        providers.push(Box::new(OneNoteProvider::new(client_id)));
        info!("OneNoteProvider registered in AppState");
    }

    Ok(())
}

/// Remove tokens and unregister OneNoteProvider from AppState.
#[tauri::command]
pub async fn onenote_revoke(state: State<'_, AppState>) -> Result<(), String> {
    let mut providers = state.providers.write().await;
    providers.retain(|p| p.id() != "onenote");

    TokenStorage::delete().map_err(|e| e.to_string())?;
    info!("OneNote authentication revoked");
    Ok(())
}

/// Check if OneNote is authenticated (tokens exist in keyring).
#[tauri::command]
pub async fn onenote_status() -> Result<bool, String> {
    match TokenStorage::load() {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

// ─── BYO Credentials ─────────────────────────────────────────────────────────

/// Get the user's custom Azure App ID override.
#[tauri::command]
pub async fn onenote_credentials_get(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let repo = state.settings_repo.as_ref();
    let client_id = repo
        .get_raw("onenote_client_id")
        .await
        .map_err(|e| e.to_string())?
        .filter(|s| !s.is_empty());
    Ok(client_id)
}

/// Persist or clear user-provided Azure App ID.
#[tauri::command]
pub async fn onenote_credentials_set(
    state: State<'_, AppState>,
    client_id: Option<String>,
) -> Result<(), String> {
    let repo = state.settings_repo.as_ref();
    repo.set_raw("onenote_client_id", client_id.as_deref().unwrap_or(""))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
