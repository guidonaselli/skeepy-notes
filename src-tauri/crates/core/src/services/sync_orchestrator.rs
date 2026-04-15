use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::provider::{NoteProvider, ProviderStatus};
use crate::repository::ProviderSyncRecord;
use crate::services::note_service::NoteService;

// ─── Sync Trigger ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SyncTrigger {
    /// Runs async at startup, non-blocking for the UI.
    Startup,
    /// User explicitly clicked "Sync now".
    Manual,
    /// Fired by the internal interval timer.
    Scheduled,
    /// OS resumed from sleep/hibernate.
    WakeFromSleep,
}

// ─── Backoff Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    /// Exponential base (e.g. 2.0 → each retry doubles the delay).
    pub multiplier: f32,
    /// Random jitter factor (0.1 = ±10%).
    pub jitter_factor: f32,
    /// After this many failures the provider is marked as Error.
    pub max_retries: u32,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(30 * 60), // 30 min
            multiplier: 2.0,
            jitter_factor: 0.1,
            max_retries: 5,
        }
    }
}

impl BackoffConfig {
    /// Returns the delay to wait before retry number `attempt` (0-indexed).
    /// Clamped between `initial_delay` and `max_delay`, with jitter.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = if attempt == 0 {
            self.initial_delay.as_secs_f32()
        } else {
            self.initial_delay.as_secs_f32() * self.multiplier.powi(attempt as i32)
        };
        let clamped = base.min(self.max_delay.as_secs_f32());
        // Deterministic-enough jitter using sub-second timestamp entropy
        let ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let jitter = clamped * self.jitter_factor * ((ns % 1000) as f32 / 1000.0 - 0.5);
        Duration::from_secs_f32((clamped + jitter).max(1.0))
    }
}

// ─── Sync Result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub provider_id: String,
    pub notes_fetched: u32,
    pub notes_updated: u32,
    pub trigger: SyncTrigger,
    pub completed_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl SyncResult {
    pub fn success(
        provider_id: impl Into<String>,
        fetched: u32,
        updated: u32,
        trigger: SyncTrigger,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            notes_fetched: fetched,
            notes_updated: updated,
            trigger,
            completed_at: Utc::now(),
            error: None,
        }
    }

    pub fn failed(
        provider_id: impl Into<String>,
        message: impl Into<String>,
        trigger: SyncTrigger,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            notes_fetched: 0,
            notes_updated: 0,
            trigger,
            completed_at: Utc::now(),
            error: Some(message.into()),
        }
    }
}

// ─── Sync Orchestrator ────────────────────────────────────────────────────────

pub struct SyncOrchestrator {
    note_service: Arc<NoteService>,
    providers: Vec<Arc<Mutex<dyn NoteProvider>>>,
    backoff: BackoffConfig,
    /// Tracks when each provider last successfully synced.
    last_sync: HashMap<String, DateTime<Utc>>,
    /// Minimum wall-clock time between automatic syncs per provider.
    min_interval: Duration,
}

impl SyncOrchestrator {
    pub fn new(
        note_service: Arc<NoteService>,
        providers: Vec<Arc<Mutex<dyn NoteProvider>>>,
    ) -> Self {
        Self {
            note_service,
            providers,
            backoff: BackoffConfig::default(),
            last_sync: HashMap::new(),
            min_interval: Duration::from_secs(60),
        }
    }

    pub fn with_backoff(mut self, config: BackoffConfig) -> Self {
        self.backoff = config;
        self
    }

    pub fn with_min_interval(mut self, interval: Duration) -> Self {
        self.min_interval = interval;
        self
    }

    /// Run a sync cycle for all registered providers.
    /// Failures in one provider do NOT stop others from running.
    pub async fn run_sync(&mut self, trigger: SyncTrigger) -> Vec<SyncResult> {
        let mut results = Vec::new();

        for provider_arc in &self.providers {
            let provider = provider_arc.lock().await;
            let pid = provider.id().to_string();

            // ── Cooldown check (skip for Manual) ──────────────────────────────
            if trigger != SyncTrigger::Manual {
                if let Some(last) = self.last_sync.get(&pid) {
                    let elapsed = Utc::now()
                        .signed_duration_since(*last)
                        .to_std()
                        .unwrap_or(Duration::ZERO);
                    if elapsed < self.min_interval {
                        info!(provider = %pid, "Skipping sync — cooldown not elapsed");
                        continue;
                    }
                }
            }

            // ── Provider status check ─────────────────────────────────────────
            match provider.status() {
                ProviderStatus::Disabled => {
                    info!(provider = %pid, "Skipping disabled provider");
                    continue;
                }
                ProviderStatus::RateLimited { retry_after } => {
                    if Utc::now() < retry_after {
                        warn!(provider = %pid, "Skipping rate-limited provider");
                        continue;
                    }
                }
                _ => {}
            }

            // ── Auth check ────────────────────────────────────────────────────
            if !provider.is_authenticated().await {
                warn!(provider = %pid, "Provider not authenticated — skipping sync");
                results.push(SyncResult::failed(&pid, "Not authenticated", trigger.clone()));
                continue;
            }

            // ── Determine incremental vs full fetch ───────────────────────────
            let since = if provider.capabilities().supports_incremental_sync {
                // Use last successful sync time from persistent state
                self.note_service
                    .get_provider_sync_state(&pid)
                    .await
                    .ok()
                    .and_then(|r| r.last_sync_at)
            } else {
                None
            };

            // ── Fetch ─────────────────────────────────────────────────────────
            match provider.fetch_notes(since).await {
                Ok(remote_notes) => {
                    let fetched = remote_notes.len() as u32;
                    let mut updated = 0u32;

                    for remote in remote_notes {
                        match self.note_service.merge_remote(remote, &pid).await {
                            Ok(true) => updated += 1,
                            Ok(false) => {}
                            Err(e) => {
                                error!(provider = %pid, error = %e, "Error merging note — continuing");
                            }
                        }
                    }

                    let now = Utc::now();
                    self.last_sync.insert(pid.clone(), now);

                    // Persist sync state
                    let record = ProviderSyncRecord {
                        provider_id: pid.clone(),
                        last_sync_at: Some(now),
                        last_error: None,
                        retry_count: 0,
                        status: "active".to_string(),
                    };
                    let _ = self.note_service.update_provider_sync_state(&record).await;

                    info!(
                        provider = %pid,
                        fetched = fetched,
                        updated = updated,
                        "Sync completed successfully"
                    );
                    results.push(SyncResult::success(&pid, fetched, updated, trigger.clone()));
                }

                Err(e) => {
                    error!(provider = %pid, error = %e, "Sync failed");

                    let record = ProviderSyncRecord {
                        provider_id: pid.clone(),
                        last_sync_at: None, // keep last successful
                        last_error: Some(e.to_string()),
                        retry_count: 1, // incremented by caller if needed
                        status: "error".to_string(),
                    };
                    let _ = self.note_service.update_provider_sync_state(&record).await;

                    results.push(SyncResult::failed(&pid, e.to_string(), trigger.clone()));
                }
            }
        }

        results
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially() {
        let cfg = BackoffConfig {
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(300),
            multiplier: 2.0,
            jitter_factor: 0.0, // no jitter for determinism
            max_retries: 5,
        };

        let d0 = cfg.delay_for_attempt(0).as_secs_f32();
        let d1 = cfg.delay_for_attempt(1).as_secs_f32();
        let d2 = cfg.delay_for_attempt(2).as_secs_f32();

        // attempt 0 = initial_delay = 5s
        assert!((d0 - 5.0).abs() < 0.5, "d0 = {d0}");
        // attempt 1 = 5 * 2^1 = 10s
        assert!((d1 - 10.0).abs() < 0.5, "d1 = {d1}");
        // attempt 2 = 5 * 2^2 = 20s
        assert!((d2 - 20.0).abs() < 0.5, "d2 = {d2}");
    }

    #[test]
    fn backoff_clamped_at_max() {
        let cfg = BackoffConfig {
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter_factor: 0.0,
            max_retries: 5,
        };

        // attempt 10 = 5 * 2^10 = 5120s → clamped to 30s
        let d = cfg.delay_for_attempt(10).as_secs_f32();
        assert!(d <= 30.5, "delay {d} exceeds max");
    }

    #[test]
    fn backoff_minimum_is_one_second() {
        let cfg = BackoffConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            multiplier: 0.0, // pathological — stays at 0
            jitter_factor: 0.0,
            max_retries: 5,
        };
        let d = cfg.delay_for_attempt(5).as_secs_f32();
        assert!(d >= 1.0, "delay {d} below minimum of 1s");
    }
}
