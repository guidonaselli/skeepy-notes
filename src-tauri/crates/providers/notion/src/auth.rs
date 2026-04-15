use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use serde::Deserialize;

use skeepy_core::ProviderError;

use crate::token::TokenSet;

const AUTH_URL: &str = "https://api.notion.com/v1/oauth/authorize";
const TOKEN_URL: &str = "https://api.notion.com/v1/oauth/token";

/// Session data for the OAuth2 authorization code flow.
///
/// Notion does NOT support PKCE, so we use state-only CSRF protection.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub state: String,
    pub redirect_uri: String,
}

impl AuthSession {
    pub fn new(redirect_uri: impl Into<String>) -> Self {
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        let state = hex::encode(bytes);
        Self { state, redirect_uri: redirect_uri.into() }
    }
}

/// Build the Notion OAuth2 authorization URL.
pub fn build_auth_url(client_id: &str, session: &AuthSession) -> String {
    let params = [
        ("client_id", client_id.to_string()),
        ("redirect_uri", session.redirect_uri.clone()),
        ("response_type", "code".to_string()),
        ("state", session.state.clone()),
        ("owner", "user".to_string()),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{AUTH_URL}?{query}")
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    workspace_name: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Exchange the authorization code for a token.
///
/// Notion requires HTTP Basic auth with `client_id:client_secret`.
pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenSet, ProviderError> {
    // Notion requires Basic auth: base64(client_id:client_secret)
    let credentials = STANDARD.encode(format!("{client_id}:{client_secret}"));

    let resp = http
        .post(TOKEN_URL)
        .header("Authorization", format!("Basic {credentials}"))
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| ProviderError::Api(format!("Token request failed: {e}")))?;

    let body: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Token response parse error: {e}")))?;

    if let Some(err) = body.error {
        return Err(ProviderError::Api(format!(
            "Notion OAuth error {err}: {}",
            body.error_description.unwrap_or_default()
        )));
    }

    let access_token = body
        .access_token
        .ok_or_else(|| ProviderError::Api("No access_token in Notion response".to_string()))?;

    Ok(TokenSet::new(access_token, body.workspace_name))
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c]
            } else {
                let bytes = c.to_string().into_bytes();
                bytes
                    .into_iter()
                    .flat_map(|b| format!("%{:02X}", b).chars().collect::<Vec<_>>())
                    .collect()
            }
        })
        .collect()
}
