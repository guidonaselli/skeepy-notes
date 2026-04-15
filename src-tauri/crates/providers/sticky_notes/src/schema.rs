use chrono::{DateTime, TimeZone, Utc};

/// Raw row from the `Note` table in plum.sqlite.
///
/// Schema (modern Windows Sticky Notes 3.x+):
/// ```sql
/// CREATE TABLE Note (
///   Id           TEXT PRIMARY KEY,
///   Text         TEXT,          -- content in Soft XAML markup
///   Theme        TEXT,          -- "Yellow", "Pink", "Green", etc.
///   WindowPosition TEXT,        -- JSON {"X":..., "Y":...} or similar
///   WindowSize     TEXT,        -- JSON {"Width":..., "Height":...}
///   CreatedAt    INTEGER,       -- Unix timestamp in milliseconds (100ns ticks in some versions)
///   UpdatedAt    INTEGER,
///   DeletedAt    INTEGER,       -- NULL = not deleted
///   IsOpen       INTEGER,       -- 0/1
///   IsAlwaysOnTop INTEGER,      -- 0/1
///   IsReminder   INTEGER        -- 0/1
/// );
/// ```
pub struct RawNote {
    pub id: String,
    pub text: Option<String>,
    pub theme: Option<String>,
    pub created_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub deleted_at_ms: Option<i64>,
    pub is_pinned: bool,
}

impl RawNote {
    pub fn created_at(&self) -> DateTime<Utc> {
        parse_timestamp(self.created_at_ms)
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        parse_timestamp(self.updated_at_ms)
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at_ms.is_some_and(|v| v > 0)
    }
}

/// Parse a Windows Sticky Notes timestamp.
///
/// Sticky Notes uses two formats depending on the app version:
/// - Unix milliseconds (recent versions): values ~1.7 × 10^12
/// - Windows FILETIME ticks (100ns since 1601-01-01) in some older rows: values ~1.3 × 10^17
fn parse_timestamp(ms: Option<i64>) -> DateTime<Utc> {
    let Some(v) = ms else { return Utc::now() };
    if v <= 0 {
        return Utc::now();
    }

    // Heuristic: if value > 1e15, it's likely a FILETIME tick count.
    // FILETIME epoch = 1601-01-01T00:00:00Z
    // Unix epoch offset in 100ns ticks = 116444736000000000
    if v > 1_000_000_000_000_000 {
        let unix_100ns = v - 116_444_736_000_000_000_i64;
        let secs = unix_100ns / 10_000_000;
        let nanos = ((unix_100ns % 10_000_000) * 100) as u32;
        return Utc.timestamp_opt(secs, nanos).single().unwrap_or_else(Utc::now);
    }

    // Otherwise treat as Unix milliseconds
    let secs = v / 1000;
    let millis = (v % 1000) as u32;
    Utc.timestamp_opt(secs, millis * 1_000_000)
        .single()
        .unwrap_or_else(Utc::now)
}

