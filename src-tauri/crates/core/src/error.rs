use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limited — retry after {retry_after}")]
    RateLimited {
        retry_after: chrono::DateTime<chrono::Utc>,
    },

    #[error("Operation not supported: {operation}")]
    NotSupported { operation: String },

    #[error("Provider API error: {0}")]
    Api(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}
