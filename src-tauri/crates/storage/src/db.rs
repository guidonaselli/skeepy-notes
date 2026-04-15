use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tracing::info;

use skeepy_core::StorageError;

/// Migrations embedded at compile time — paths relative to this source file.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial",      include_str!("../migrations/001_initial.sql")),
    ("002_fts5",         include_str!("../migrations/002_fts5.sql")),
    ("003_usage_events", include_str!("../migrations/003_usage_events.sql")),
    ("004_embeddings",   include_str!("../migrations/004_embeddings.sql")),
];

// ─── Database Handle ──────────────────────────────────────────────────────────

/// Thread-safe handle to a single SQLite connection.
///
/// Operations that need blocking I/O should be dispatched via
/// `tokio::task::spawn_blocking` with a clone of the inner `Arc`.
#[derive(Clone)]
pub struct Database {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (or create) a SQLite database at `path`, configure it, and run migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let conn = Connection::open(path)
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Self::configure(&conn)?;

        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        db.run_migrations()?;

        Ok(db)
    }

    /// Open an in-memory SQLite database — used in tests.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Self::configure(&conn)?;

        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        db.run_migrations()?;

        Ok(db)
    }

    /// Apply PRAGMAs that must be set before any work begins.
    fn configure(conn: &Connection) -> Result<(), StorageError> {
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous  = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA cache_size   = -8000;  -- 8 MB page cache
            PRAGMA temp_store   = MEMORY;
        ").map_err(|e| StorageError::Database(e.to_string()))
    }

    /// Run all pending migrations in order, idempotently.
    fn run_migrations(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        // Bootstrap the migrations table itself
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                name       TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            );"
        ).map_err(|e| StorageError::Migration(e.to_string()))?;

        for (name, sql) in MIGRATIONS {
            let applied: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM _migrations WHERE name = ?1",
                    rusqlite::params![name],
                    |row| row.get(0),
                )
                .map_err(|e| StorageError::Migration(e.to_string()))?;

            if !applied {
                info!(migration = name, "Applying migration");
                conn.execute_batch(sql)
                    .map_err(|e| StorageError::Migration(format!("{name}: {e}")))?;

                conn.execute(
                    "INSERT INTO _migrations (name, applied_at) VALUES (?1, datetime('now'))",
                    rusqlite::params![name],
                ).map_err(|e| StorageError::Migration(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Execute a blocking closure against the underlying connection.
    /// Use this in async repository methods via `tokio::task::spawn_blocking`.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Connection) -> Result<T, StorageError>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }
}
