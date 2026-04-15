use chrono::{DateTime, Duration, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use skeepy_core::ProviderError;

const KEYRING_SERVICE: &str = "skeepy_notion";
const KEYRING_ACCOUNT: &str = "tokens";

/// Notion uses long-lived access tokens (no expiry per OAuth spec).
/// We store them the same way as Keep/OneNote for consistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    /// Notion tokens don't expire, but we keep the field for compatibility.
    pub expires_at: DateTime<Utc>,
    /// The workspace that was authorized.
    pub workspace_name: Option<String>,
}

impl TokenSet {
    pub fn new(access_token: String, workspace_name: Option<String>) -> Self {
        Self {
            access_token,
            // Set expiry 100 years in the future — effectively never.
            expires_at: Utc::now() + Duration::days(365 * 100),
            workspace_name,
        }
    }

    /// Notion tokens don't expire, so this always returns false.
    pub fn is_expired(&self) -> bool {
        false
    }
}

pub struct TokenStorage;

impl TokenStorage {
    pub fn save(tokens: &TokenSet) -> Result<(), ProviderError> {
        let json = serde_json::to_string(tokens)
            .map_err(|e| ProviderError::Api(format!("Token serialization error: {e}")))?;
        Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|e| ProviderError::Api(format!("Keyring error: {e}")))?
            .set_password(&json)
            .map_err(|e| ProviderError::Api(format!("Keyring set error: {e}")))
    }

    pub fn load() -> Result<Option<TokenSet>, ProviderError> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|e| ProviderError::Api(format!("Keyring error: {e}")))?;
        match entry.get_password() {
            Ok(json) => {
                let tokens: TokenSet = serde_json::from_str(&json)
                    .map_err(|e| ProviderError::Api(format!("Token parse error: {e}")))?;
                Ok(Some(tokens))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ProviderError::Api(format!("Keyring get error: {e}"))),
        }
    }

    pub fn delete() -> Result<(), ProviderError> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|e| ProviderError::Api(format!("Keyring error: {e}")))?;
        match entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(ProviderError::Api(format!("Keyring delete error: {e}"))),
        }
    }
}
