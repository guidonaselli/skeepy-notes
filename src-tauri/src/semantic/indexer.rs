/// Semantic indexer (S37).
///
/// Builds TF-IDF embeddings for all notes and stores them in `note_embeddings`.
/// The indexer runs in the background after every sync and on startup.
/// Cosine-similarity search is done in-process (no sqlite-vec extension needed
/// for the TF-IDF model; the schema is forward-compatible with sqlite-vec).
use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use rusqlite::params;
use serde_json;
use tracing::{debug, info, warn};

use skeepy_core::StorageError;
use skeepy_storage::Database;

use crate::semantic::tfidf::{build_vocab, compute_idf, cosine_sim, tokenize, vectorize};
use crate::state::AppState;

const MODEL_VERSION: &str = "tfidf-v1";
const TOP_K: usize = 10;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Re-index all notes that don't yet have a current-model embedding.
/// Runs in a background thread — never blocks the main event loop.
pub fn index_in_background(db: Arc<Database>) {
    std::thread::spawn(move || {
        if let Err(e) = run_index(&db) {
            warn!(error = %e, "Semantic indexer failed");
        }
    });
}

/// Semantic search: returns the top-K most similar note IDs for `query`.
pub fn search(state: &AppState, query: &str, limit: usize) -> Vec<(String, f32)> {
    match run_search(&state.db, query, limit) {
        Ok(results) => results,
        Err(e) => {
            warn!(error = %e, "Semantic search failed");
            vec![]
        }
    }
}

// ─── Indexing ─────────────────────────────────────────────────────────────────

fn run_index(db: &Database) -> Result<(), StorageError> {

    // Load all visible note texts.
    let notes: Vec<(String, String)> = db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, COALESCE(content_text, '') FROM notes WHERE is_trashed = 0",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let rows: Result<Vec<_>, _> = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| StorageError::Database(e.to_string()))?
            .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
            .collect();
        rows
    })?;

    if notes.is_empty() {
        return Ok(());
    }

    // Find which notes are missing embeddings for the current model version.
    let already_indexed: std::collections::HashSet<String> = db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT note_id FROM note_embeddings WHERE model_version = ?1",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let ids: Result<Vec<String>, _> = stmt
            .query_map(params![MODEL_VERSION], |row| row.get(0))
            .map_err(|e| StorageError::Database(e.to_string()))?
            .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
            .collect();
        ids.map(|v| v.into_iter().collect())
    })?;

    let to_index: Vec<_> = notes
        .iter()
        .filter(|(id, _)| !already_indexed.contains(id.as_str()))
        .collect();

    if to_index.is_empty() {
        debug!("Semantic index up-to-date — nothing to do");
        return Ok(());
    }

    info!(count = to_index.len(), "Building semantic index for new notes");

    // Build corpus vocabulary from ALL notes (including already-indexed ones)
    // so that document frequencies are accurate.
    let all_tokens: Vec<Vec<String>> =
        notes.iter().map(|(_, text)| tokenize(text)).collect();

    let vocab = build_vocab(&all_tokens);
    let df: HashMap<String, usize> = {
        let mut m: HashMap<String, usize> = HashMap::new();
        for tokens in &all_tokens {
            let unique: std::collections::HashSet<_> = tokens.iter().collect();
            for t in unique {
                *m.entry(t.clone()).or_insert(0) += 1;
            }
        }
        m
    };
    let idf = compute_idf(&vocab, &df, notes.len());

    let now = Utc::now().to_rfc3339();
    let mut indexed = 0usize;

    for (note_id, text) in &to_index {
        let tokens = tokenize(text);
        let vec = vectorize(&tokens, &vocab, &idf);

        // Store as JSON array (forward-compatible with sqlite-vec native column later)
        let json = serde_json::to_string(&vec).unwrap_or_else(|_| "[]".to_string());

        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO note_embeddings (note_id, model_version, embedding, indexed_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(note_id) DO UPDATE SET
                     model_version = excluded.model_version,
                     embedding     = excluded.embedding,
                     indexed_at    = excluded.indexed_at",
                params![note_id, MODEL_VERSION, json, now],
            )
            .map(|_| ())
            .map_err(|e| StorageError::Database(e.to_string()))
        })?;

        indexed += 1;
    }

    info!(indexed, "Semantic index updated");
    Ok(())
}

// ─── Search ───────────────────────────────────────────────────────────────────

fn run_search(db: &Database, query: &str, limit: usize) -> Result<Vec<(String, f32)>, StorageError> {

    // Load all embeddings from the current model version.
    let embeddings: Vec<(String, Vec<f32>)> = db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT note_id, embedding FROM note_embeddings WHERE model_version = ?1",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let rows: Result<Vec<_>, _> = stmt
            .query_map(params![MODEL_VERSION], |row| {
                let note_id: String = row.get(0)?;
                let json: String = row.get(1)?;
                Ok((note_id, json))
            })
            .map_err(|e| StorageError::Database(e.to_string()))?
            .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
            .collect();
        rows
    })?
    .into_iter()
    .filter_map(|(id, json)| {
        let vec: Vec<f32> = serde_json::from_str(&json).ok()?;
        Some((id, vec))
    })
    .collect();

    if embeddings.is_empty() {
        return Ok(vec![]);
    }

    // Build vocab from ALL indexed note texts to get the same IDF weights.
    let all_texts: Vec<String> = db.with_conn(|conn| {
        let mut stmt = conn
            .prepare("SELECT COALESCE(content_text, '') FROM notes WHERE is_trashed = 0")
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let rows: Result<Vec<String>, _> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| StorageError::Database(e.to_string()))?
            .map(|r| r.map_err(|e| StorageError::Database(e.to_string())))
            .collect();
        rows
    })?;

    let all_tokens: Vec<Vec<String>> = all_texts.iter().map(|t| tokenize(t)).collect();
    let vocab = build_vocab(&all_tokens);
    let df: HashMap<String, usize> = {
        let mut m = HashMap::new();
        for tokens in &all_tokens {
            let unique: std::collections::HashSet<_> = tokens.iter().collect();
            for t in unique { *m.entry(t.clone()).or_insert(0) += 1; }
        }
        m
    };
    let idf = compute_idf(&vocab, &df, all_texts.len());

    // Vectorize the query.
    let query_tokens = tokenize(query);
    let query_vec = vectorize(&query_tokens, &vocab, &idf);

    // Cosine similarity against all stored embeddings.
    let dim = query_vec.len();
    let mut scores: Vec<(String, f32)> = embeddings
        .into_iter()
        .filter_map(|(id, emb)| {
            if emb.len() != dim { return None; }
            let sim = cosine_sim(&query_vec, &emb);
            if sim > 0.05 { Some((id, sim)) } else { None }
        })
        .collect();

    // Sort descending by score
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(limit.min(TOP_K));

    Ok(scores)
}
