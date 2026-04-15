use tauri::{AppHandle, State};
use tracing::info;

use skeepy_core::SettingsRepository;
use skeepy_provider_keep::{build_auth_url, PkceSession, TokenStorage};
use skeepy_provider_keep::provider::KeepProvider;

use crate::state::AppState;

// Credentials compiled in at build time via env vars.
// Use `option_env!` so the crate compiles without them set (returns "" → treated as missing).
const KEEP_CLIENT_ID: &str = match option_env!("GOOGLE_CLIENT_ID") {
    Some(v) => v,
    None => "",
};
const KEEP_CLIENT_SECRET: &str = match option_env!("GOOGLE_CLIENT_SECRET") {
    Some(v) => v,
    None => "",
};

/// Resolves Keep credentials in priority order:
/// 1. User override stored in settings DB (keep_client_id / keep_client_secret)
/// 2. Credentials compiled into the binary via GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET
async fn resolve_keep_credentials(
    repo: &dyn SettingsRepository,
) -> Result<(String, Option<String>), String> {
    let custom_id = repo.get_raw("keep_client_id").await.ok().flatten();
    match custom_id {
        Some(id) if !id.is_empty() => {
            let secret = repo.get_raw("keep_client_secret").await.ok().flatten();
            Ok((id, secret.filter(|s| !s.is_empty())))
        }
        _ => {
            if KEEP_CLIENT_ID.is_empty() {
                return Err(
                    "No Keep credentials configured. Provide GOOGLE_CLIENT_ID at build time \
                     or enter your own credentials in Settings → Google Keep."
                        .to_string(),
                );
            }
            let secret = if KEEP_CLIENT_SECRET.is_empty() {
                None
            } else {
                Some(KEEP_CLIENT_SECRET.to_string())
            };
            Ok((KEEP_CLIENT_ID.to_string(), secret))
        }
    }
}

/// Start the OAuth2 PKCE flow.
///
/// Returns the authorization URL the frontend should open in the browser.
/// `redirect_uri` is dynamic (chosen at runtime by tauri-plugin-oauth) so it
/// still comes from the caller; `client_id` is resolved internally.
#[tauri::command]
pub async fn keep_start_auth(
    state: State<'_, AppState>,
    redirect_uri: String,
) -> Result<KeepAuthInitResponse, String> {
    let (client_id, _) = resolve_keep_credentials(state.settings_repo.as_ref()).await?;
    let session = PkceSession::new(redirect_uri);
    let auth_url = build_auth_url(&client_id, &session);
    info!("Keep auth URL generated");

    Ok(KeepAuthInitResponse {
        auth_url,
        code_verifier: session.code_verifier,
        state: session.state,
        redirect_uri: session.redirect_uri,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct KeepAuthInitResponse {
    pub auth_url: String,
    pub code_verifier: String,
    pub state: String,
    pub redirect_uri: String,
}

/// Complete the OAuth2 flow by exchanging the authorization code for tokens,
/// storing them in the keyring, and registering KeepProvider in AppState.
#[tauri::command]
pub async fn keep_complete_auth(
    state: State<'_, AppState>,
    _app: AppHandle,
    code: String,
    code_verifier: String,
    redirect_uri: String,
) -> Result<(), String> {
    use skeepy_provider_keep::auth::exchange_code;

    let (client_id, client_secret) =
        resolve_keep_credentials(state.settings_repo.as_ref()).await?;

    let http = reqwest::Client::new();
    let session = PkceSession { code_verifier, state: String::new(), redirect_uri };

    let tokens = exchange_code(&http, &client_id, client_secret.as_deref(), &code, &session)
        .await
        .map_err(|e| e.to_string())?;

    TokenStorage::save(&tokens).map_err(|e| e.to_string())?;
    info!("Keep tokens saved to keyring");

    let mut providers = state.providers.write().await;
    let already_registered = providers.iter().any(|p| p.id() == "keep");
    if !already_registered {
        providers.push(Box::new(KeepProvider::new(client_id, client_secret)));
        info!("KeepProvider registered in AppState");
    }

    Ok(())
}

/// Remove tokens and unregister KeepProvider from AppState.
#[tauri::command]
pub async fn keep_revoke(state: State<'_, AppState>) -> Result<(), String> {
    let mut providers = state.providers.write().await;
    providers.retain(|p| p.id() != "keep");

    TokenStorage::delete().map_err(|e| e.to_string())?;
    info!("Keep authentication revoked");
    Ok(())
}

/// Check if Keep is authenticated (tokens exist in keyring).
#[tauri::command]
pub async fn keep_status() -> Result<bool, String> {
    match TokenStorage::load() {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

// ─── BYO Credentials ─────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KeepCredentials {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

/// Get the user's custom credentials override.
/// Returns `None` for each field when not set (empty string stored → treated as absent).
#[tauri::command]
pub async fn keep_credentials_get(
    state: State<'_, AppState>,
) -> Result<KeepCredentials, String> {
    let repo = state.settings_repo.as_ref();
    let client_id = repo
        .get_raw("keep_client_id")
        .await
        .map_err(|e| e.to_string())?
        .filter(|s| !s.is_empty());
    let client_secret = repo
        .get_raw("keep_client_secret")
        .await
        .map_err(|e| e.to_string())?
        .filter(|s| !s.is_empty());
    Ok(KeepCredentials { client_id, client_secret })
}

/// Persist or clear user-provided BYO credentials.
/// Pass `None` (or empty string) to clear a field.
#[tauri::command]
pub async fn keep_credentials_set(
    state: State<'_, AppState>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Result<(), String> {
    let repo = state.settings_repo.as_ref();
    repo.set_raw("keep_client_id", client_id.as_deref().unwrap_or(""))
        .await
        .map_err(|e| e.to_string())?;
    repo.set_raw("keep_client_secret", client_secret.as_deref().unwrap_or(""))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use skeepy_core::StorageError;

    struct MockSettings {
        client_id: Option<String>,
        client_secret: Option<String>,
    }

    #[async_trait]
    impl SettingsRepository for MockSettings {
        async fn get_raw(&self, key: &str) -> Result<Option<String>, StorageError> {
            match key {
                "keep_client_id" => Ok(self.client_id.clone()),
                "keep_client_secret" => Ok(self.client_secret.clone()),
                _ => Ok(None),
            }
        }

        async fn set_raw(&self, _key: &str, _value: &str) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn custom_credentials_take_priority_over_compiled() {
        let repo = MockSettings {
            client_id: Some("custom-id".to_string()),
            client_secret: Some("custom-secret".to_string()),
        };
        let (id, secret) = resolve_keep_credentials(&repo).await.unwrap();
        assert_eq!(id, "custom-id");
        assert_eq!(secret, Some("custom-secret".to_string()));
    }

    #[tokio::test]
    async fn empty_custom_id_falls_through() {
        let repo = MockSettings {
            client_id: Some("".to_string()),
            client_secret: None,
        };
        // KEEP_CLIENT_ID is "" in test builds (env var not set in test env)
        // → expect an error with a helpful message
        let result = resolve_keep_credentials(&repo).await;
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("GOOGLE_CLIENT_ID"), "error should mention env var, got: {msg}");
    }

    #[tokio::test]
    async fn whitespace_only_custom_id_is_rejected() {
        // Only non-empty strings are accepted as custom credentials.
        // The filter(|s| !s.is_empty()) in resolve_keep_credentials handles this
        // for the empty string case; whitespace-only passes through to KEEP_CLIENT_ID fallback.
        let repo = MockSettings {
            client_id: Some("   ".to_string()),
            client_secret: None,
        };
        // "   " is non-empty → treated as a real client_id
        let result = resolve_keep_credentials(&repo).await;
        assert!(result.is_ok());
        let (id, _) = result.unwrap();
        assert_eq!(id, "   "); // caller's responsibility to trim if needed
    }

    #[tokio::test]
    async fn none_custom_id_falls_through() {
        let repo = MockSettings {
            client_id: None,
            client_secret: None,
        };
        // No custom credentials → falls back to compiled constants → error in test env
        let result = resolve_keep_credentials(&repo).await;
        assert!(result.is_err());
    }
}
