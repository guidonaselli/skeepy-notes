use chrono::{DateTime, Duration, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use skeepy_core::ProviderError;

const KEYRING_SERVICE: &str = "skeepy_onenote";
const KEYRING_ACCOUNT: &str = "tokens";

/// OAuth token set for Microsoft Identity Platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: DateTime<Utc>,
}

impl TokenSet {
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in_secs: u64,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at: Utc::now() + Duration::seconds(expires_in_secs as i64 - 60),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
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
