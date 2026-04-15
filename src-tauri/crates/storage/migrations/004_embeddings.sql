-- Note embeddings for semantic search (S37).
-- Each row stores the vector representation of a note's text content.
-- Initially populated by the local TF-IDF indexer; can be upgraded to
-- ONNX embeddings without schema changes by bumping the model_version.

CREATE TABLE IF NOT EXISTS note_embeddings (
    note_id       TEXT    NOT NULL PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
    model_version TEXT    NOT NULL DEFAULT 'tfidf-v1',
    -- Embedding stored as a JSON array of f32 values.
    -- sqlite-vec (when loaded) will replace this with a native vector column.
    embedding     TEXT    NOT NULL,
    indexed_at    TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_note_embeddings_model
    ON note_embeddings(model_version);
