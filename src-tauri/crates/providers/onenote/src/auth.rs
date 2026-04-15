use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use skeepy_core::ProviderError;

use crate::token::TokenSet;

const TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const AUTH_URL: &str  = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
// Notes.ReadWrite covers both reading and writing. Notes.Read is read-only.
const SCOPE: &str = "offline_access Notes.ReadWrite";

/// PKCE session — same structure as Keep's.
#[derive(Debug, Clone)]
pub struct PkceSession {
    pub code_verifier: String,
    pub state: String,
    pub redirect_uri: String,
}

impl PkceSession {
    pub fn new(redirect_uri: impl Into<String>) -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(bytes);

        let mut state_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut state_bytes);
        let state = URL_SAFE_NO_PAD.encode(state_bytes);

        Self { code_verifier, state, redirect_uri: redirect_uri.into() }
    }

    fn code_challenge(&self) -> String {
        let hash = Sha256::digest(self.code_verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(hash)
    }
}

/// Build the Microsoft OAuth2 authorization URL.
pub fn build_auth_url(client_id: &str, session: &PkceSession) -> String {
    let params = [
        ("client_id", client_id.to_string()),
        ("redirect_uri", session.redirect_uri.clone()),
        ("response_type", "code".to_string()),
        ("scope", SCOPE.to_string()),
        ("code_challenge", session.code_challenge()),
        ("code_challenge_method", "S256".to_string()),
        ("state", session.state.clone()),
        ("prompt", "select_account".to_string()),
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
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    code: &str,
    session: &PkceSession,
) -> Result<TokenSet, ProviderError> {
    let params = vec![
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", &session.redirect_uri),
        ("code_verifier", &session.code_verifier),
        ("scope", SCOPE),
    ];
    post_token(http, &params).await
}

pub async fn refresh_access_token(
    http: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenSet, ProviderError> {
    let params = vec![
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("scope", SCOPE),
    ];
    let mut tokens = post_token(http, &params).await?;
    if tokens.refresh_token.is_none() {
        tokens.refresh_token = Some(refresh_token.to_string());
    }
    Ok(tokens)
}

async fn post_token(
    http: &reqwest::Client,
    params: &[(&str, &str)],
) -> Result<TokenSet, ProviderError> {
    let resp = http
        .post(TOKEN_URL)
        .form(params)
        .send()
        .await
        .map_err(|e| ProviderError::Api(format!("Token request failed: {e}")))?;

    let body: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Token response parse error: {e}")))?;

    if let Some(err) = body.error {
        return Err(ProviderError::Api(format!(
            "OAuth error {err}: {}",
            body.error_description.unwrap_or_default()
        )));
    }

    let expires_in = body.expires_in.unwrap_or(3600);
    Ok(TokenSet::new(body.access_token, body.refresh_token, expires_in))
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
