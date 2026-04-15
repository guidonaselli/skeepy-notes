use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::params;

use skeepy_core::{SettingsRepository, StorageError};

use crate::db::Database;

pub struct SqliteSettingsRepository {
    db: Arc<Database>,
}

impl SqliteSettingsRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get_raw(&self, key: &str) -> Result<Option<String>, StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let result = conn
                    .query_row(
                        "SELECT value FROM settings WHERE key = ?1",
                        params![key],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| StorageError::Database(e.to_string()))?;
                Ok(result)
            })
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn set_raw(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        let value = value.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO settings (key, value, updated_at)
                     VALUES (?1, ?2, datetime('now'))
                     ON CONFLICT(key) DO UPDATE SET
                         value      = excluded.value,
                         updated_at = excluded.updated_at",
                    params![key, value],
                )
                .map(|_| ())
                .map_err(|e| StorageError::Database(e.to_string()))
            })
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }
}

// Optional helper for typed settings
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::db::Database;

    fn make_repo() -> SqliteSettingsRepository {
        let db = Arc::new(Database::open_in_memory().expect("in-memory DB"));
        SqliteSettingsRepository::new(db)
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let repo = make_repo();
        let val = repo.get_raw("nonexistent").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn set_and_get_roundtrip() {
        let repo = make_repo();
        repo.set_raw("theme", "dark").await.unwrap();
        let val = repo.get_raw("theme").await.unwrap();
        assert_eq!(val, Some("dark".to_string()));
    }

    #[tokio::test]
    async fn set_overwrites_existing() {
        let repo = make_repo();
        repo.set_raw("key", "v1").await.unwrap();
        repo.set_raw("key", "v2").await.unwrap();
        let val = repo.get_raw("key").await.unwrap();
        assert_eq!(val, Some("v2".to_string()));
    }
}
