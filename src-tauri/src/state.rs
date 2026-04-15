use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use skeepy_core::{NoteProvider, NoteRepository, SettingsRepository};
use skeepy_storage::Database;

/// Central application state managed by Tauri.
///
/// Stored via `app.manage(state)` and accessed in commands via `State<'_, AppState>`.
/// All fields are `Arc`-wrapped so they can be shared across async command handlers.
pub struct AppState {
    /// Raw database handle — kept for lifecycle management (keeps the file open).
    #[allow(dead_code)]
    pub db: Arc<Database>,
    /// Note storage port (Hexagonal: inner ring depends on trait, not concrete type).
    pub notes_repo: Arc<dyn NoteRepository>,
    /// Settings key-value storage port.
    pub settings_repo: Arc<dyn SettingsRepository>,
    /// Registered note providers (local, keep, …).
    /// `RwLock` so providers can be added/removed at runtime.
    pub providers: Arc<RwLock<Vec<Box<dyn NoteProvider>>>>,
    /// Shared folder handle for the Markdown provider.
    /// Updating this Arc automatically affects the already-registered provider.
    pub markdown_folder: Arc<std::sync::RwLock<Option<PathBuf>>>,
    /// Shared vault handle for the Obsidian provider.
    pub obsidian_vault: Arc<std::sync::RwLock<Option<PathBuf>>>,
}
