pub mod error;
pub mod note;
pub mod provider;
pub mod repository;
pub mod services;
pub mod settings;

// Re-export the most commonly used types at the crate root.
pub use error::{CoreError, ProviderError, StorageError};
pub use note::{
    ChecklistItem, Label, Note, NoteColor, NoteContent, NoteId, NoteLayout, Point, ProviderId,
    Size, SyncState,
};
pub use provider::{
    CreateNoteRequest, NoteProvider, ProviderCapabilities, ProviderStability, ProviderStatus,
    RemoteNote, UpdateNoteRequest,
};
pub use repository::{
    NoteRepository, NoteSearchResult, ProviderSyncRecord, SettingsRepository,
};
pub use services::{
    note_service::NoteService,
    sync_orchestrator::{BackoffConfig, SyncOrchestrator, SyncResult, SyncTrigger},
};
pub use settings::{AppSettings, Theme};
