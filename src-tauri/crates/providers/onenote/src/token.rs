use chrono::{DateTime, Duration, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use skeepy_core::ProviderError;

const KEYRING_SERVICE: &str = "skeepy_onenote";
const KEYRING_ACCOUNT: &str = "refresh_token";

/// OAuth token set for Microsoft Identity Platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
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
    /// Persists only the refresh_token — access tokens are too long for Windows
    /// Credential Manager (2560 char UTF-16 limit) and are ephemeral anyway.
    pub fn save(tokens: &TokenSet) -> Result<(), ProviderError> {
        let refresh = tokens.refresh_token.as_deref().ok_or_else(|| {
            ProviderError::Api("No refresh token to persist".to_string())
        })?;
        Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|e| ProviderError::Api(format!("Keyring error: {e}")))?
            .set_password(refresh)
            .map_err(|e| ProviderError::Api(format!("Keyring set error: {e}")))
    }

    /// Returns a TokenSet with an already-expired access_token so that
    /// get_access_token() always refreshes on first use after startup.
    pub fn load() -> Result<Option<TokenSet>, ProviderError> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|e| ProviderError::Api(format!("Keyring error: {e}")))?;
        match entry.get_password() {
            Ok(refresh_token) => Ok(Some(TokenSet {
                access_token: String::new(),
                refresh_token: Some(refresh_token),
                expires_at: Utc::now() - Duration::seconds(1),
            })),
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
