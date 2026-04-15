-- Migration 001: Initial schema
-- WAL mode and foreign keys are set at connection time, not in migrations.

-- Notes: the core entity
CREATE TABLE IF NOT EXISTS notes (
    id           TEXT PRIMARY KEY,
    source_id    TEXT NOT NULL,
    provider_id  TEXT NOT NULL,
    title        TEXT,
    -- 'text' or 'checklist'
    content_type TEXT NOT NULL DEFAULT 'text',
    -- Plaintext for FTS indexing. For checklists: space-joined item texts.
    content_text TEXT,
    -- JSON array of ChecklistItem for checklist type. NULL for text notes.
    content_json TEXT,
    color        TEXT NOT NULL DEFAULT 'default',
    is_pinned    INTEGER NOT NULL DEFAULT 0,
    is_archived  INTEGER NOT NULL DEFAULT 0,
    is_trashed   INTEGER NOT NULL DEFAULT 0,
    -- Sync state variant: local_only | synced | local_ahead | remote_ahead | conflict | sync_error
    sync_state      TEXT NOT NULL DEFAULT 'local_only',
    -- JSON payload for sync_state variants that carry extra data (Synced.at, SyncError.message+retries)
    sync_state_data TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    synced_at    TEXT,
    UNIQUE(provider_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_notes_source     ON notes(provider_id, source_id);
CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_notes_provider   ON notes(provider_id);
CREATE INDEX IF NOT EXISTS idx_notes_trashed    ON notes(is_trashed);

-- Note layouts: persisted separately so sync writes don't clobber user layout
CREATE TABLE IF NOT EXISTS note_layouts (
    note_id      TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
    pos_x        REAL,
    pos_y        REAL,
    width        REAL,
    height       REAL,
    visible      INTEGER NOT NULL DEFAULT 0,
    always_on_top INTEGER NOT NULL DEFAULT 0,
    z_order      INTEGER NOT NULL DEFAULT 0
);

-- Labels
CREATE TABLE IF NOT EXISTS labels (
    id          TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    source_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    UNIQUE(provider_id, source_id)
);

CREATE TABLE IF NOT EXISTS note_labels (
    note_id  TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (note_id, label_id)
);

-- Provider sync state
CREATE TABLE IF NOT EXISTS provider_sync_state (
    provider_id  TEXT PRIMARY KEY,
    last_sync_at TEXT,
    last_error   TEXT,
    retry_count  INTEGER NOT NULL DEFAULT 0,
    status       TEXT NOT NULL DEFAULT 'active'
);

-- App settings: simple key-value store
CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
