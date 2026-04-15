use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use serde_json;
use uuid::Uuid;

use skeepy_core::{
    ChecklistItem, Label, Note, NoteColor, NoteContent, NoteId, NoteLayout, NoteSearchResult,
    NoteRepository, Point, ProviderSyncRecord, Size, StorageError, SyncState,
};

use crate::db::Database;

// ─── Implementation ───────────────────────────────────────────────────────────

pub struct SqliteNoteRepository {
    db: Arc<Database>,
}

impl SqliteNoteRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NoteRepository for SqliteNoteRepository {
    async fn find_all(&self) -> Result<Vec<Note>, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| find_all_sync(conn))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn find_by_id(&self, id: &NoteId) -> Result<Option<Note>, StorageError> {
        let db = Arc::clone(&self.db);
        let id = *id;
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| find_by_id_sync(conn, &id))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn find_by_provider(&self, provider_id: &str) -> Result<Vec<Note>, StorageError> {
        let db = Arc::clone(&self.db);
        let pid = provider_id.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| find_by_provider_sync(conn, &pid))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn search_fts(&self, query: &str, limit: u32) -> Result<Vec<NoteSearchResult>, StorageError> {
        let db = Arc::clone(&self.db);
        let q = sanitize_fts_query(query);
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| search_fts_sync(conn, &q, limit))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn upsert(&self, note: &Note) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let note = note.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| upsert_sync(conn, &note))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn update_layout(&self, id: &NoteId, layout: &NoteLayout) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let id = *id;
        let layout = layout.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| update_layout_sync(conn, &id, &layout))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn soft_delete(&self, id: &NoteId) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let id = *id;
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "UPDATE notes SET is_trashed = 1, updated_at = ?1 WHERE id = ?2",
                    params![Utc::now().to_rfc3339(), id.to_string()],
                )
                .map(|_| ())
                .map_err(|e| StorageError::Database(e.to_string()))
            })
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn find_by_source(
        &self,
        provider_id: &str,
        source_id: &str,
    ) -> Result<Option<Note>, StorageError> {
        let db = Arc::clone(&self.db);
        let pid = provider_id.to_string();
        let sid = source_id.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| find_by_source_sync(conn, &pid, &sid))
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn get_provider_sync_state(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderSyncRecord>, StorageError> {
        let db = Arc::clone(&self.db);
        let pid = provider_id.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT provider_id, last_sync_at, last_error, retry_count, status
                         FROM provider_sync_state WHERE provider_id = ?1",
                    )
                    .map_err(|e| StorageError::Database(e.to_string()))?;

                let result = stmt
                    .query_row(params![pid], |row| {
                        Ok(ProviderSyncRecord {
                            provider_id: row.get(0)?,
                            last_sync_at: row
                                .get::<_, Option<String>>(1)?
                                .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                            last_error: row.get(2)?,
                            retry_count: row.get::<_, u32>(3)?,
                            status: row.get(4)?,
                        })
                    })
                    .optional()
                    .map_err(|e| StorageError::Database(e.to_string()))?;

                Ok(result)
            })
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }

    async fn update_provider_sync_state(
        &self,
        record: &ProviderSyncRecord,
    ) -> Result<(), StorageError> {
        let db = Arc::clone(&self.db);
        let r = record.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO provider_sync_state
                        (provider_id, last_sync_at, last_error, retry_count, status)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(provider_id) DO UPDATE SET
                        last_sync_at = COALESCE(excluded.last_sync_at, last_sync_at),
                        last_error   = excluded.last_error,
                        retry_count  = excluded.retry_count,
                        status       = excluded.status",
                    params![
                        r.provider_id,
                        r.last_sync_at.map(|d| d.to_rfc3339()),
                        r.last_error,
                        r.retry_count,
                        r.status,
                    ],
                )
                .map(|_| ())
                .map_err(|e| StorageError::Database(e.to_string()))
            })
        })
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
    }
}

// ─── Sync helper functions ────────────────────────────────────────────────────

fn find_all_sync(conn: &Connection) -> Result<Vec<Note>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.source_id, n.provider_id, n.title,
                    n.content_type, n.content_text, n.content_json,
                    n.color, n.is_pinned, n.is_archived, n.is_trashed,
                    n.sync_state, n.sync_state_data,
                    n.created_at, n.updated_at, n.synced_at,
                    nl.pos_x, nl.pos_y, nl.width, nl.height,
                    nl.visible, nl.always_on_top, nl.z_order
             FROM notes n
             LEFT JOIN note_layouts nl ON n.id = nl.note_id
             WHERE n.is_trashed = 0
             ORDER BY n.is_pinned DESC, n.updated_at DESC",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let notes: Result<Vec<Note>, _> = stmt
        .query_map([], row_to_note)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
        .collect();

    let notes = notes?;
    // Load labels for all notes in a second pass
    load_labels_for_notes(conn, notes)
}

fn find_by_id_sync(conn: &Connection, id: &NoteId) -> Result<Option<Note>, StorageError> {
    let id_str = id.to_string();
    let mut stmt = conn
        .prepare(NOTE_SELECT_QUERY_BY_ID)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let result = stmt
        .query_row(params![id_str], row_to_note)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))?;

    match result {
        None => Ok(None),
        Some(note) => {
            let notes = load_labels_for_notes(conn, vec![note])?;
            Ok(notes.into_iter().next())
        }
    }
}

fn find_by_provider_sync(conn: &Connection, provider_id: &str) -> Result<Vec<Note>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.source_id, n.provider_id, n.title,
                    n.content_type, n.content_text, n.content_json,
                    n.color, n.is_pinned, n.is_archived, n.is_trashed,
                    n.sync_state, n.sync_state_data,
                    n.created_at, n.updated_at, n.synced_at,
                    nl.pos_x, nl.pos_y, nl.width, nl.height,
                    nl.visible, nl.always_on_top, nl.z_order
             FROM notes n
             LEFT JOIN note_layouts nl ON n.id = nl.note_id
             WHERE n.provider_id = ?1 AND n.is_trashed = 0
             ORDER BY n.updated_at DESC",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let notes: Result<Vec<Note>, _> = stmt
        .query_map(params![provider_id], row_to_note)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
        .collect();

    load_labels_for_notes(conn, notes?)
}

fn find_by_source_sync(
    conn: &Connection,
    provider_id: &str,
    source_id: &str,
) -> Result<Option<Note>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.source_id, n.provider_id, n.title,
                    n.content_type, n.content_text, n.content_json,
                    n.color, n.is_pinned, n.is_archived, n.is_trashed,
                    n.sync_state, n.sync_state_data,
                    n.created_at, n.updated_at, n.synced_at,
                    nl.pos_x, nl.pos_y, nl.width, nl.height,
                    nl.visible, nl.always_on_top, nl.z_order
             FROM notes n
             LEFT JOIN note_layouts nl ON n.id = nl.note_id
             WHERE n.provider_id = ?1 AND n.source_id = ?2",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let result = stmt
        .query_row(params![provider_id, source_id], row_to_note)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))?;

    match result {
        None => Ok(None),
        Some(note) => {
            let notes = load_labels_for_notes(conn, vec![note])?;
            Ok(notes.into_iter().next())
        }
    }
}

fn search_fts_sync(
    conn: &Connection,
    fts_query: &str,
    limit: u32,
) -> Result<Vec<NoteSearchResult>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.source_id, n.provider_id, n.title,
                    n.content_type, n.content_text, n.content_json,
                    n.color, n.is_pinned, n.is_archived, n.is_trashed,
                    n.sync_state, n.sync_state_data,
                    n.created_at, n.updated_at, n.synced_at,
                    nl.pos_x, nl.pos_y, nl.width, nl.height,
                    nl.visible, nl.always_on_top, nl.z_order,
                    snippet(notes_fts, 1, '<mark>', '</mark>', '…', 24) AS excerpt,
                    notes_fts.rank
             FROM notes_fts
             JOIN notes n ON notes_fts.rowid = n.rowid
             LEFT JOIN note_layouts nl ON n.id = nl.note_id
             WHERE notes_fts MATCH ?1
               AND n.is_trashed = 0
             ORDER BY notes_fts.rank
             LIMIT ?2",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let results: Result<Vec<NoteSearchResult>, _> = stmt
        .query_map(params![fts_query, limit], |row| {
            let note = row_to_note(row)?;
            let excerpt: Option<String> = row.get(23)?;
            let rank: f64 = row.get(24)?;
            Ok(NoteSearchResult { note, excerpt, rank })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?
        .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
        .collect();

    results
}

fn upsert_sync(conn: &Connection, note: &Note) -> Result<(), StorageError> {
    let (content_type, content_text, content_json) = content_to_sql(&note.content);
    let (sync_state, sync_state_data) = sync_state_to_sql(&note.sync_state);

    conn.execute(
        "INSERT INTO notes
            (id, source_id, provider_id, title,
             content_type, content_text, content_json,
             color, is_pinned, is_archived, is_trashed,
             sync_state, sync_state_data,
             created_at, updated_at, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(id) DO UPDATE SET
             title           = excluded.title,
             content_type    = excluded.content_type,
             content_text    = excluded.content_text,
             content_json    = excluded.content_json,
             color           = excluded.color,
             is_pinned       = excluded.is_pinned,
             is_archived     = excluded.is_archived,
             is_trashed      = excluded.is_trashed,
             sync_state      = excluded.sync_state,
             sync_state_data = excluded.sync_state_data,
             updated_at      = excluded.updated_at,
             synced_at       = excluded.synced_at",
        params![
            note.id.to_string(),
            note.source_id,
            note.provider_id,
            note.title,
            content_type,
            content_text,
            content_json,
            color_to_str(&note.color),
            note.is_pinned as i32,
            note.is_archived as i32,
            note.is_trashed as i32,
            sync_state,
            sync_state_data,
            note.created_at.to_rfc3339(),
            note.updated_at.to_rfc3339(),
            note.synced_at.map(|d| d.to_rfc3339()),
        ],
    )
    .map_err(|e| StorageError::Database(e.to_string()))?;

    // Upsert layout
    upsert_layout_sync(conn, &note.id, &note.layout)?;

    // Upsert labels
    upsert_labels_sync(conn, &note.id, &note.labels)?;

    Ok(())
}

fn update_layout_sync(conn: &Connection, id: &NoteId, layout: &NoteLayout) -> Result<(), StorageError> {
    upsert_layout_sync(conn, id, layout)
}

fn upsert_layout_sync(conn: &Connection, id: &NoteId, layout: &NoteLayout) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO note_layouts
            (note_id, pos_x, pos_y, width, height, visible, always_on_top, z_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(note_id) DO UPDATE SET
             pos_x        = excluded.pos_x,
             pos_y        = excluded.pos_y,
             width        = excluded.width,
             height       = excluded.height,
             visible      = excluded.visible,
             always_on_top = excluded.always_on_top,
             z_order      = excluded.z_order",
        params![
            id.to_string(),
            layout.position.map(|p| p.x),
            layout.position.map(|p| p.y),
            layout.size.map(|s| s.width),
            layout.size.map(|s| s.height),
            layout.visible as i32,
            layout.always_on_top as i32,
            layout.z_order,
        ],
    )
    .map(|_| ())
    .map_err(|e| StorageError::Database(e.to_string()))
}

fn upsert_labels_sync(conn: &Connection, note_id: &NoteId, labels: &[Label]) -> Result<(), StorageError> {
    let id_str = note_id.to_string();
    // Remove old label associations for this note
    conn.execute("DELETE FROM note_labels WHERE note_id = ?1", params![id_str])
        .map_err(|e| StorageError::Database(e.to_string()))?;

    for label in labels {
        // Upsert label definition
        conn.execute(
            "INSERT INTO labels (id, provider_id, source_id, name)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name",
            params![label.id, "unknown", label.id, label.name],
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

        // Add association
        conn.execute(
            "INSERT OR IGNORE INTO note_labels (note_id, label_id) VALUES (?1, ?2)",
            params![id_str, label.id],
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
    }

    Ok(())
}

fn load_labels_for_notes(conn: &Connection, mut notes: Vec<Note>) -> Result<Vec<Note>, StorageError> {
    if notes.is_empty() {
        return Ok(notes);
    }

    // Build IN clause
    let ids: Vec<String> = notes.iter().map(|n| format!("'{}'", n.id)).collect();
    let in_clause = ids.join(",");

    let sql = format!(
        "SELECT nl.note_id, l.id, l.name
         FROM note_labels nl
         JOIN labels l ON nl.label_id = l.id
         WHERE nl.note_id IN ({})",
        in_clause
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let label_rows: Result<Vec<(String, Label)>, _> = stmt
        .query_map([], |row| {
            let note_id: String = row.get(0)?;
            let label = Label {
                id: row.get(1)?,
                name: row.get(2)?,
            };
            Ok((note_id, label))
        })
        .map_err(|e| StorageError::Database(e.to_string()))?
        .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
        .collect();

    let label_rows = label_rows?;

    // Distribute labels to their notes
    for (note_id_str, label) in label_rows {
        if let Some(note) = notes.iter_mut().find(|n| n.id.to_string() == note_id_str) {
            note.labels.push(label);
        }
    }

    Ok(notes)
}

// ─── Row → Note conversion ────────────────────────────────────────────────────

/// Column order must match all SELECT statements above.
/// Columns 0-15: note fields, 16-22: layout fields.
fn row_to_note(row: &Row) -> rusqlite::Result<Note> {
    let id_str: String = row.get(0)?;
    let id = id_str.parse::<Uuid>().unwrap_or_else(|_| Uuid::new_v4());

    let content_type: String = row.get(4)?;
    let content_text: Option<String> = row.get(5)?;
    let content_json: Option<String> = row.get(6)?;
    let content = content_from_sql(&content_type, content_text.as_deref(), content_json.as_deref());

    let sync_state_str: String = row.get(11)?;
    let sync_state_data: Option<String> = row.get(12)?;
    let sync_state = sync_state_from_sql(&sync_state_str, sync_state_data.as_deref());

    let created_at = row
        .get::<_, String>(13)?
        .parse::<DateTime<Utc>>()
        .unwrap_or_else(|_| Utc::now());
    let updated_at = row
        .get::<_, String>(14)?
        .parse::<DateTime<Utc>>()
        .unwrap_or_else(|_| Utc::now());
    let synced_at = row
        .get::<_, Option<String>>(15)?
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    // Layout columns (16-22) — NULLs when no layout row exists
    let pos_x: Option<f32> = row.get(16)?;
    let pos_y: Option<f32> = row.get(17)?;
    let width: Option<f32> = row.get(18)?;
    let height: Option<f32> = row.get(19)?;
    let visible: Option<i32> = row.get(20)?;
    let always_on_top: Option<i32> = row.get(21)?;
    let z_order: Option<i32> = row.get(22)?;

    let position = pos_x.zip(pos_y).map(|(x, y)| Point { x, y });
    let size = width.zip(height).map(|(w, h)| Size { width: w, height: h });

    let layout = NoteLayout {
        position,
        size,
        visible: visible.unwrap_or(0) != 0,
        always_on_top: always_on_top.unwrap_or(0) != 0,
        z_order: z_order.unwrap_or(0),
    };

    Ok(Note {
        id,
        source_id: row.get(1)?,
        provider_id: row.get(2)?,
        title: row.get(3)?,
        content,
        labels: Vec::new(), // populated by load_labels_for_notes
        color: color_from_str(&row.get::<_, String>(7)?),
        is_pinned: row.get::<_, i32>(8)? != 0,
        is_archived: row.get::<_, i32>(9)? != 0,
        is_trashed: row.get::<_, i32>(10)? != 0,
        sync_state,
        created_at,
        updated_at,
        synced_at,
        layout,
    })
}

const NOTE_SELECT_QUERY_BY_ID: &str =
    "SELECT n.id, n.source_id, n.provider_id, n.title,
            n.content_type, n.content_text, n.content_json,
            n.color, n.is_pinned, n.is_archived, n.is_trashed,
            n.sync_state, n.sync_state_data,
            n.created_at, n.updated_at, n.synced_at,
            nl.pos_x, nl.pos_y, nl.width, nl.height,
            nl.visible, nl.always_on_top, nl.z_order
     FROM notes n
     LEFT JOIN note_layouts nl ON n.id = nl.note_id
     WHERE n.id = ?1";

// ─── Serialization helpers ────────────────────────────────────────────────────

fn content_to_sql(content: &NoteContent) -> (String, Option<String>, Option<String>) {
    match content {
        NoteContent::Text(text) => ("text".to_string(), Some(text.clone()), None),
        NoteContent::Checklist(items) => {
            let plain: String = items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join(" ");
            let json = serde_json::to_string(items).unwrap_or_default();
            ("checklist".to_string(), Some(plain), Some(json))
        }
    }
}

fn content_from_sql(
    content_type: &str,
    content_text: Option<&str>,
    content_json: Option<&str>,
) -> NoteContent {
    match content_type {
        "checklist" => {
            if let Some(json) = content_json {
                if let Ok(items) = serde_json::from_str::<Vec<ChecklistItem>>(json) {
                    return NoteContent::Checklist(items);
                }
            }
            NoteContent::Checklist(Vec::new())
        }
        _ => NoteContent::Text(content_text.unwrap_or_default().to_string()),
    }
}

fn sync_state_to_sql(state: &SyncState) -> (String, Option<String>) {
    match state {
        SyncState::LocalOnly => ("local_only".to_string(), None),
        SyncState::Synced { at } => (
            "synced".to_string(),
            Some(serde_json::json!({ "at": at.to_rfc3339() }).to_string()),
        ),
        SyncState::LocalAhead => ("local_ahead".to_string(), None),
        SyncState::RemoteAhead => ("remote_ahead".to_string(), None),
        SyncState::Conflict { remote_title, remote_content, remote_updated_at } => {
            let (ct, ct_text, ct_json) = content_to_sql(remote_content);
            let data = serde_json::json!({
                "remote_title": remote_title,
                "remote_content_type": ct,
                "remote_content_text": ct_text,
                "remote_content_json": ct_json,
                "remote_updated_at": remote_updated_at.to_rfc3339(),
            });
            ("conflict".to_string(), Some(data.to_string()))
        }
        SyncState::SyncError { message, retries } => (
            "sync_error".to_string(),
            Some(serde_json::json!({ "message": message, "retries": retries }).to_string()),
        ),
    }
}

fn sync_state_from_sql(state: &str, data: Option<&str>) -> SyncState {
    match state {
        "synced" => {
            let at = data
                .and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok())
                .and_then(|v| v["at"].as_str().and_then(|s| s.parse::<DateTime<Utc>>().ok()))
                .unwrap_or_else(Utc::now);
            SyncState::Synced { at }
        }
        "local_ahead" => SyncState::LocalAhead,
        "remote_ahead" => SyncState::RemoteAhead,
        "conflict" => {
            if let Some(d) = data.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                let remote_title = d["remote_title"].as_str().map(|s| s.to_string());
                let ct = d["remote_content_type"].as_str().unwrap_or("text");
                let ct_text = d["remote_content_text"].as_str();
                let ct_json = d["remote_content_json"].as_str();
                let remote_content = content_from_sql(ct, ct_text, ct_json);
                let remote_updated_at = d["remote_updated_at"]
                    .as_str()
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(Utc::now);
                SyncState::Conflict { remote_title, remote_content, remote_updated_at }
            } else {
                // Legacy rows with no conflict data — treat as local-only
                SyncState::LocalOnly
            }
        }
        "sync_error" => {
            let (message, retries) = data
                .and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok())
                .map(|v| {
                    let msg = v["message"].as_str().unwrap_or("Unknown error").to_string();
                    let ret = v["retries"].as_u64().unwrap_or(0) as u32;
                    (msg, ret)
                })
                .unwrap_or_else(|| ("Unknown error".to_string(), 0));
            SyncState::SyncError { message, retries }
        }
        _ => SyncState::LocalOnly,
    }
}

fn color_to_str(color: &NoteColor) -> &'static str {
    match color {
        NoteColor::Default  => "default",
        NoteColor::Red      => "red",
        NoteColor::Orange   => "orange",
        NoteColor::Yellow   => "yellow",
        NoteColor::Green    => "green",
        NoteColor::Teal     => "teal",
        NoteColor::Blue     => "blue",
        NoteColor::DarkBlue => "dark_blue",
        NoteColor::Purple   => "purple",
        NoteColor::Pink     => "pink",
        NoteColor::Brown    => "brown",
        NoteColor::Gray     => "gray",
    }
}

fn color_from_str(s: &str) -> NoteColor {
    match s {
        "red"       => NoteColor::Red,
        "orange"    => NoteColor::Orange,
        "yellow"    => NoteColor::Yellow,
        "green"     => NoteColor::Green,
        "teal"      => NoteColor::Teal,
        "blue"      => NoteColor::Blue,
        "dark_blue" => NoteColor::DarkBlue,
        "purple"    => NoteColor::Purple,
        "pink"      => NoteColor::Pink,
        "brown"     => NoteColor::Brown,
        "gray"      => NoteColor::Gray,
        _           => NoteColor::Default,
    }
}

/// Sanitize a user search query for FTS5.
/// Each word is wrapped in double quotes to prevent injection via FTS5 operators.
fn sanitize_fts_query(query: &str) -> String {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| format!("\"{}\"", w.replace('"', "")))
        .collect();

    if words.is_empty() {
        return String::new();
    }

    words.join(" OR ")
}

// ─── Extension trait for optional query_row result ────────────────────────────

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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use chrono::Utc;
    use skeepy_core::{NoteContent, NoteLayout, Point, Size};
    use crate::db::Database;

    fn make_repo() -> SqliteNoteRepository {
        let db = Arc::new(Database::open_in_memory().expect("in-memory DB"));
        SqliteNoteRepository::new(db)
    }

    fn make_note(provider_id: &str, source_id: &str, text: &str) -> Note {
        let mut n = Note::new_local(NoteContent::Text(text.to_string()));
        n.source_id = source_id.to_string();
        n.provider_id = provider_id.to_string();
        n
    }

    #[tokio::test]
    async fn upsert_and_find_by_id() {
        let repo = make_repo();
        let note = make_note("local", "src-1", "hello world");
        let id = note.id;

        repo.upsert(&note).await.expect("upsert");

        let found = repo.find_by_id(&id).await.expect("find");
        assert!(found.is_some());
        assert_eq!(found.unwrap().source_id, "src-1");
    }

    #[tokio::test]
    async fn upsert_updates_existing_note() {
        let repo = make_repo();
        let mut note = make_note("local", "src-1", "original");
        let id = note.id;
        repo.upsert(&note).await.expect("first upsert");

        note.content = NoteContent::Text("updated".to_string());
        repo.upsert(&note).await.expect("second upsert");

        let found = repo.find_by_id(&id).await.unwrap().unwrap();
        match found.content {
            NoteContent::Text(t) => assert_eq!(t, "updated"),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn find_all_excludes_trashed() {
        let repo = make_repo();
        let mut n1 = make_note("local", "s1", "visible");
        let mut n2 = make_note("local", "s2", "trashed");
        n2.is_trashed = true;

        repo.upsert(&n1).await.unwrap();
        repo.upsert(&n2).await.unwrap();

        let all = repo.find_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].source_id, "s1");
    }

    #[tokio::test]
    async fn soft_delete_marks_trashed() {
        let repo = make_repo();
        let note = make_note("local", "s1", "to delete");
        let id = note.id;
        repo.upsert(&note).await.unwrap();

        repo.soft_delete(&id).await.unwrap();

        let all = repo.find_all().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn update_layout_persists() {
        let repo = make_repo();
        let note = make_note("local", "s1", "note");
        let id = note.id;
        repo.upsert(&note).await.unwrap();

        let layout = NoteLayout {
            position: Some(Point { x: 42.0, y: 88.0 }),
            size: Some(Size { width: 320.0, height: 200.0 }),
            visible: true,
            always_on_top: false,
            z_order: 5,
        };
        repo.update_layout(&id, &layout).await.unwrap();

        let found = repo.find_by_id(&id).await.unwrap().unwrap();
        assert_eq!(found.layout.position.unwrap().x, 42.0);
        assert_eq!(found.layout.z_order, 5);
    }

    #[tokio::test]
    async fn find_by_source_works() {
        let repo = make_repo();
        let note = make_note("keep", "keep-note-123", "keep note");
        repo.upsert(&note).await.unwrap();

        let found = repo.find_by_source("keep", "keep-note-123").await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_source("keep", "does-not-exist").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn fts_search_finds_notes() {
        let repo = make_repo();
        repo.upsert(&make_note("local", "s1", "recordatorio de compras")).await.unwrap();
        repo.upsert(&make_note("local", "s2", "meeting notes for monday")).await.unwrap();
        repo.upsert(&make_note("local", "s3", "lista de tareas")).await.unwrap();

        let results = repo.search_fts("compras", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].note.source_id, "s1");
    }

    #[tokio::test]
    async fn sync_state_round_trip() {
        let repo = make_repo();
        let mut note = make_note("keep", "s1", "nota");
        note.sync_state = SyncState::SyncError {
            message: "timeout".to_string(),
            retries: 3,
        };
        repo.upsert(&note).await.unwrap();

        let found = repo.find_by_id(&note.id).await.unwrap().unwrap();
        match found.sync_state {
            SyncState::SyncError { message, retries } => {
                assert_eq!(message, "timeout");
                assert_eq!(retries, 3);
            }
            other => panic!("Expected SyncError, got {:?}", other),
        }
    }

    #[test]
    fn sanitize_fts_query_wraps_words() {
        let q = sanitize_fts_query("hello world");
        assert_eq!(q, r#""hello" OR "world""#);
    }

    #[test]
    fn sanitize_fts_query_strips_quotes() {
        let q = sanitize_fts_query(r#"say "hello""#);
        assert!(!q.contains("\"\""));
    }
}
