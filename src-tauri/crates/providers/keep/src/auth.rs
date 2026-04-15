use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use skeepy_core::ProviderError;

use crate::token::TokenSet;

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const SCOPE: &str = "https://www.googleapis.com/auth/keep.readonly";

// ─── PKCE ─────────────────────────────────────────────────────────────────────

/// Carries the PKCE values needed to complete the auth flow.
#[derive(Debug, Clone)]
pub struct PkceSession {
    pub code_verifier: String,
    pub state: String,
    pub redirect_uri: String,
}

impl PkceSession {
    /// Generate a new PKCE session.
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

/// Build the Google OAuth2 authorization URL for a given PKCE session.
pub fn build_auth_url(client_id: &str, session: &PkceSession) -> String {
    let params = [
        ("client_id", client_id),
        ("redirect_uri", &session.redirect_uri),
        ("response_type", "code"),
        ("scope", SCOPE),
        ("code_challenge", &session.code_challenge()),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("state", &session.state),
        ("prompt", "consent"),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{AUTH_URL}?{query}")
}

// ─── Token exchange ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Exchange an authorization code for tokens (PKCE flow).
pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    session: &PkceSession,
) -> Result<TokenSet, ProviderError> {
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", &session.redirect_uri),
        ("code_verifier", &session.code_verifier),
    ];
    if let Some(secret) = client_secret {
        params.push(("client_secret", secret));
    }

    post_token(http, &params).await
}

/// Refresh an expired access token using the stored refresh token.
pub async fn refresh_access_token(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<TokenSet, ProviderError> {
    let mut params = vec![
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token),
    ];
    if let Some(secret) = client_secret {
        params.push(("client_secret", secret));
    }

    let mut tokens = post_token(http, &params).await?;
    // Refresh responses don't include a new refresh_token — carry the old one.
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
    let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);

    Ok(TokenSet {
        access_token: body.access_token,
        refresh_token: body.refresh_token,
        expires_at,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn urlencoded(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c]
            } else {
                let bytes = c.to_string().into_bytes();
                bytes
                    .into_iter()
                    .flat_map(|b| {
                        format!("%{:02X}", b).chars().collect::<Vec<_>>()
                    })
                    .collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_generates_valid_challenge() {
        let session = PkceSession::new("http://localhost:8080");
        assert!(!session.code_verifier.is_empty());
        assert!(!session.state.is_empty());
        let challenge = session.code_challenge();
        assert!(!challenge.is_empty());
        // Challenge and verifier must differ
        assert_ne!(session.code_verifier, challenge);
    }

    #[test]
    fn build_auth_url_contains_required_params() {
        let session = PkceSession::new("http://localhost:9090");
        let url = build_auth_url("my-client-id", &session);
        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("keep.readonly"));
    }
}
