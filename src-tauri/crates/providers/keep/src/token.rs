use chrono::{DateTime, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use skeepy_core::ProviderError;

const SERVICE: &str = "skeepy-notes";
const USERNAME: &str = "google-keep-tokens";

/// OAuth2 tokens returned by Google's token endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: DateTime<Utc>,
}

impl TokenSet {
    pub fn is_expired(&self) -> bool {
        // Treat as expired 60 seconds before actual expiry to avoid races.
        Utc::now() >= self.expires_at - chrono::Duration::seconds(60)
    }
}

/// Persists and retrieves tokens via the OS credential store (DPAPI on Windows).
pub struct TokenStorage;

impl TokenStorage {
    pub fn save(tokens: &TokenSet) -> Result<(), ProviderError> {
        let json = serde_json::to_string(tokens)
            .map_err(|e| ProviderError::Api(format!("Token serialize error: {e}")))?;
        Entry::new(SERVICE, USERNAME)
            .map_err(|e| ProviderError::Api(format!("Keyring open error: {e}")))?
            .set_password(&json)
            .map_err(|e| ProviderError::Api(format!("Keyring write error: {e}")))
    }

    pub fn load() -> Result<Option<TokenSet>, ProviderError> {
        let entry = Entry::new(SERVICE, USERNAME)
            .map_err(|e| ProviderError::Api(format!("Keyring open error: {e}")))?;

        match entry.get_password() {
            Ok(json) => {
                let tokens: TokenSet = serde_json::from_str(&json)
                    .map_err(|e| ProviderError::Api(format!("Token deserialize error: {e}")))?;
                Ok(Some(tokens))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ProviderError::Api(format!("Keyring read error: {e}"))),
        }
    }

    pub fn delete() -> Result<(), ProviderError> {
        Entry::new(SERVICE, USERNAME)
            .map_err(|e| ProviderError::Api(format!("Keyring open error: {e}")))?
            .delete_credential()
            .map_err(|e| match e {
                keyring::Error::NoEntry => ProviderError::Api("No token to delete".to_string()),
                _ => ProviderError::Api(format!("Keyring delete error: {e}")),
            })
    }
}
