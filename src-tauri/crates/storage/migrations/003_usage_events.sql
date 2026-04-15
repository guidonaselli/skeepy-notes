-- Usage event tracking for smart sync scheduling (S41).
-- Records when the app is opened so we can predict peak usage times.

CREATE TABLE IF NOT EXISTS usage_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT    NOT NULL,  -- 'app_open', 'sync_trigger', 'note_view'
    occurred_at TEXT   NOT NULL   -- ISO-8601 UTC timestamp
);

-- Index for range queries (last N days)
CREATE INDEX IF NOT EXISTS idx_usage_events_occurred_at
    ON usage_events(occurred_at);
