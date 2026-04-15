-- Migration 002: Full-text search via FTS5 (external content table)
-- The `content=notes` directive means FTS5 reads from the `notes` table.
-- Triggers keep the FTS index synchronized with the notes table.
-- content_rowid links FTS rowid to notes.rowid for efficient joins.

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    title,
    content_text,
    content=notes,
    content_rowid=rowid,
    tokenize='porter unicode61'
);

-- AFTER INSERT: index the new note
CREATE TRIGGER IF NOT EXISTS notes_fts_ai
AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, title, content_text)
    VALUES (new.rowid, COALESCE(new.title, ''), COALESCE(new.content_text, ''));
END;

-- AFTER DELETE: remove from FTS index
CREATE TRIGGER IF NOT EXISTS notes_fts_ad
AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, content_text)
    VALUES ('delete', old.rowid, COALESCE(old.title, ''), COALESCE(old.content_text, ''));
END;

-- AFTER UPDATE: update FTS index (delete old entry, insert new)
CREATE TRIGGER IF NOT EXISTS notes_fts_au
AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, content_text)
    VALUES ('delete', old.rowid, COALESCE(old.title, ''), COALESCE(old.content_text, ''));
    INSERT INTO notes_fts(rowid, title, content_text)
    VALUES (new.rowid, COALESCE(new.title, ''), COALESCE(new.content_text, ''));
END;
