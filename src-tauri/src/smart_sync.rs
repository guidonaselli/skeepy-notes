/// Smart Sync Scheduler (S41)
///
/// Instead of syncing on a fixed interval, we analyse the user's historical
/// app-open timestamps to predict the next "peak" usage time and sleep until
/// then.  If no clear pattern exists we fall back to the configured interval.
use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use rusqlite::params;
use tracing::{debug, info};

use crate::state::AppState;

// ─── Public entry point ───────────────────────────────────────────────────────

/// Record an `app_open` event in the database.
pub fn record_app_open(state: &AppState) {
    let db = state.db.clone();
    let now = Utc::now().to_rfc3339();
    let _ = db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO usage_events (event_type, occurred_at) VALUES ('app_open', ?1)",
            params![now],
        )
        .map(|_| ())
        .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))
    });
}

/// Compute how many seconds to sleep before the next smart sync.
///
/// Algorithm:
/// 1. Load the last 28 days of `app_open` events.
/// 2. Bucket them into 1-hour slots (day-of-week × hour-of-day = 168 slots).
/// 3. Find the upcoming slot with the highest historical activity.
/// 4. Return the seconds until that slot begins.
///
/// Falls back to `fallback_secs` if:
/// - There are fewer than 7 recorded events (too little history).
/// - The predicted next peak is further away than `fallback_secs`.
pub fn seconds_until_next_smart_sync(state: &AppState, fallback_secs: u64) -> u64 {
    match compute_next_peak_secs(state) {
        Some(secs) if secs < fallback_secs => {
            info!(secs, "Smart sync: sleeping until predicted peak");
            secs
        }
        Some(secs) => {
            debug!(
                predicted = secs,
                fallback = fallback_secs,
                "Smart sync: predicted peak is farther than fallback — using fallback"
            );
            fallback_secs
        }
        None => {
            debug!(fallback = fallback_secs, "Smart sync: not enough history — using fallback");
            fallback_secs
        }
    }
}

// ─── Core algorithm ───────────────────────────────────────────────────────────

fn compute_next_peak_secs(state: &AppState) -> Option<u64> {
    let db = state.db.clone();

    let events: Vec<DateTime<Utc>> = db
        .with_conn(|conn| {
            let cutoff = (Utc::now() - chrono::Duration::days(28)).to_rfc3339();
            let mut stmt = conn
                .prepare(
                    "SELECT occurred_at FROM usage_events
                     WHERE event_type = 'app_open' AND occurred_at >= ?1
                     ORDER BY occurred_at ASC",
                )
                .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))?;

            let rows: Result<Vec<_>, _> = stmt
                .query_map(params![cutoff], |row| {
                    let s: String = row.get(0)?;
                    Ok(s)
                })
                .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))?
                .map(|r| r.map_err(|e| skeepy_core::StorageError::Database(e.to_string())))
                .collect();

            rows.map(|v| {
                v.into_iter()
                    .filter_map(|s| s.parse::<DateTime<Utc>>().ok())
                    .collect()
            })
        })
        .unwrap_or_default();

    if events.len() < 7 {
        return None;
    }

    // Build 168-slot histogram: index = weekday (0=Mon) * 24 + hour
    let mut histogram = [0u32; 168];
    for ev in &events {
        let slot = slot_for(ev);
        histogram[slot] += 1;
    }

    // Walk forward from the *next* full hour and find the highest-count slot
    let now = Utc::now();
    let now_slot = slot_for(&now);

    let mut best_offset_hours: Option<u64> = None;
    let mut best_count = 0u32;

    // Look up to 7 days ahead (168 hours)
    for offset_h in 1u64..=168 {
        let future_slot = (now_slot + offset_h as usize) % 168;
        let count = histogram[future_slot];
        if count > best_count {
            best_count = count;
            best_offset_hours = Some(offset_h);
        }
    }

    // Need at least 2 historical events in that slot to trust the prediction
    if best_count < 2 {
        return None;
    }

    best_offset_hours.map(|h| {
        // Align to the start of the target hour
        let minutes_into_hour = now.minute() as u64;
        let seconds_into_minute = now.second() as u64;
        let seconds_until_next_hour = (60 - minutes_into_hour) * 60 - seconds_into_minute;
        seconds_until_next_hour + (h - 1) * 3600
    })
}

fn slot_for(dt: &DateTime<Utc>) -> usize {
    let weekday = match dt.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };
    weekday * 24 + dt.hour() as usize
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_for_monday_midnight() {
        let dt: DateTime<Utc> = "2024-01-01T00:00:00Z".parse().unwrap(); // Mon
        assert_eq!(slot_for(&dt), 0);
    }

    #[test]
    fn slot_for_friday_noon() {
        let dt: DateTime<Utc> = "2024-01-05T12:00:00Z".parse().unwrap(); // Fri
        assert_eq!(slot_for(&dt), 4 * 24 + 12);
    }

    #[test]
    fn slot_for_sunday_last_hour() {
        let dt: DateTime<Utc> = "2024-01-07T23:00:00Z".parse().unwrap(); // Sun
        assert_eq!(slot_for(&dt), 6 * 24 + 23);
    }
}
