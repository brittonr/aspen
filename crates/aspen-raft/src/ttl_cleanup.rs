//! Background TTL cleanup task for expired key garbage collection.
//!
//! This module provides a background task that periodically deletes expired keys
//! from the Redb state machine. It follows the "active expiration" pattern
//! (similar to Redis) to complement the "lazy expiration" filtering at read time.
//!
//! # Architecture
//!
//! The cleanup task:
//! 1. Runs on a configurable interval (default: 60 seconds)
//! 2. Deletes expired keys in batches (default: 1000 per run)
//! 3. Continues until all expired keys are deleted or max iterations reached
//! 4. Tracks metrics for monitoring (expired keys deleted, remaining, etc.)
//!
//! # Tiger Style
//!
//! - Fixed batch size prevents unbounded work per iteration
//! - Max iterations per run prevents starvation of other operations
//! - Uses CancellationToken for graceful shutdown
//! - Metrics exposed for operational visibility

use std::sync::Arc;
use std::time::Duration;

use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::storage_ports::{KvStateRead, KvStateWrite};
use crate::storage_shared::SharedRedbStorage;

/// Configuration for the TTL cleanup task.
#[derive(Debug, Clone)]
pub struct TtlCleanupConfig {
    /// Interval between cleanup runs (default: 60 seconds).
    pub cleanup_interval: Duration,
    /// Maximum keys to delete per batch (default: 1000).
    pub batch_size_keys: u32,
    /// Maximum batches per cleanup run (default: 100).
    /// Prevents the cleanup from running indefinitely if there are many expired keys.
    pub max_batches_per_run: u32,
}

impl Default for TtlCleanupConfig {
    fn default() -> Self {
        Self {
            cleanup_interval: Duration::from_secs(60),
            batch_size_keys: 1000,
            max_batches_per_run: 100,
        }
    }
}

/// Background TTL cleanup task handle for Redb storage.
///
/// Returns a CancellationToken that can be used to stop the task.
pub fn spawn_redb_ttl_cleanup_task(storage: Arc<SharedRedbStorage>, config: TtlCleanupConfig) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_ttl_cleanup_loop(&*storage, config, cancel_clone).await;
    });

    cancel
}

/// Main cleanup loop. Generic over storage port traits so domain logic
/// is testable with in-memory implementations.
async fn run_ttl_cleanup_loop<S: KvStateRead + KvStateWrite>(
    storage: &S,
    config: TtlCleanupConfig,
    cancel: CancellationToken,
) {
    let mut ticker = interval(config.cleanup_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_secs = config.cleanup_interval.as_secs(),
        batch_size = config.batch_size_keys,
        max_batches = config.max_batches_per_run,
        "TTL cleanup task started"
    );

    while wait_for_ttl_cleanup_tick(&mut ticker, &cancel).await {
        run_cleanup_iteration(storage, &config);
    }
}

async fn wait_for_ttl_cleanup_tick(ticker: &mut tokio::time::Interval, cancel: &CancellationToken) -> bool {
    tokio::select! {
        _ = cancel.cancelled() => {
            info!("TTL cleanup task shutting down");
            false
        }
        _ = ticker.tick() => true,
    }
}

#[inline]
fn should_stop_ttl_cleanup(deleted: u32, config: &TtlCleanupConfig) -> bool {
    deleted == 0 || deleted < config.batch_size_keys
}

fn log_ttl_cleanup_results<S: KvStateRead>(storage: &S, total_deleted: u64, batches_run: u32) {
    if total_deleted > 0 {
        let remaining = storage.count_expired_keys().unwrap_or(0);
        let with_ttl = storage.count_keys_with_ttl().unwrap_or(0);
        info!(
            total_deleted,
            batches_run,
            remaining_expired = remaining,
            keys_with_ttl = with_ttl,
            "TTL cleanup iteration completed"
        );
    } else {
        debug!("TTL cleanup: no expired keys to delete");
    }
}

/// Run a single cleanup iteration. Generic over port traits.
fn run_cleanup_iteration<S: KvStateRead + KvStateWrite>(storage: &S, config: &TtlCleanupConfig) {
    let mut total_deleted: u64 = 0;
    let mut batches_run: u32 = 0;

    while batches_run < config.max_batches_per_run {
        let deleted = match storage.delete_expired_keys(config.batch_size_keys) {
            Ok(deleted) => deleted,
            Err(error) => {
                warn!(error = %error, "TTL cleanup batch failed");
                break;
            }
        };

        total_deleted = total_deleted.saturating_add(u64::from(deleted));
        batches_run = batches_run.saturating_add(1);
        debug_assert!(
            batches_run <= config.max_batches_per_run,
            "TTL_CLEANUP: batches_run must stay within max_batches_per_run"
        );
        debug_assert!(
            total_deleted >= u64::from(deleted),
            "TTL_CLEANUP: total_deleted must include the latest deleted batch"
        );
        if should_stop_ttl_cleanup(deleted, config) {
            break;
        }
    }

    if batches_run >= config.max_batches_per_run && config.max_batches_per_run > 0 {
        debug!(
            total_deleted,
            batches_run,
            max_batches = config.max_batches_per_run,
            "TTL cleanup reached max batches limit"
        );
    }
    log_ttl_cleanup_results(storage, total_deleted, batches_run);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TtlCleanupConfig::default();
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
        assert_eq!(config.batch_size_keys, 1000);
        assert_eq!(config.max_batches_per_run, 100);
    }

    #[test]
    fn test_config_custom_values() {
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(30),
            batch_size_keys: 500,
            max_batches_per_run: 50,
        };
        assert_eq!(config.cleanup_interval, Duration::from_secs(30));
        assert_eq!(config.batch_size_keys, 500);
        assert_eq!(config.max_batches_per_run, 50);
    }

    #[test]
    fn test_config_clone() {
        let config = TtlCleanupConfig::default();
        let cloned = config.clone();
        assert_eq!(config.cleanup_interval, cloned.cleanup_interval);
        assert_eq!(config.batch_size_keys, cloned.batch_size_keys);
        assert_eq!(config.max_batches_per_run, cloned.max_batches_per_run);
    }

    #[test]
    fn test_config_debug_format() {
        let config = TtlCleanupConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("TtlCleanupConfig"));
        assert!(debug_str.contains("cleanup_interval"));
        assert!(debug_str.contains("batch_size_keys"));
        assert!(debug_str.contains("max_batches_per_run"));
    }

    #[test]
    fn test_cleanup_capacity_calculation() {
        // Verify the default config can handle reasonable workloads
        let config = TtlCleanupConfig::default();
        let keys_per_run = config.batch_size_keys as u64 * config.max_batches_per_run as u64;
        // Default: 1000 * 100 = 100,000 keys per run
        assert_eq!(keys_per_run, 100_000);

        // At 60 second intervals, that's ~1,667 keys/second capacity
        let keys_per_second = keys_per_run / config.cleanup_interval.as_secs();
        assert!(keys_per_second >= 1000, "cleanup capacity should be at least 1000 keys/sec");
    }

    #[test]
    fn test_config_aggressive_cleanup() {
        // Config for high-throughput TTL workloads
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(10),
            batch_size_keys: 5000,
            max_batches_per_run: 200,
        };
        let keys_per_run = config.batch_size_keys as u64 * config.max_batches_per_run as u64;
        // 5000 * 200 = 1,000,000 keys per run
        assert_eq!(keys_per_run, 1_000_000);
    }

    #[test]
    fn test_config_minimal_cleanup() {
        // Config for low-throughput scenarios
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            batch_size_keys: 100,
            max_batches_per_run: 10,
        };
        let keys_per_run = config.batch_size_keys as u64 * config.max_batches_per_run as u64;
        // 100 * 10 = 1,000 keys per run
        assert_eq!(keys_per_run, 1_000);
    }

    #[tokio::test]
    async fn test_cancellation_token_stops_task() {
        // Verify the cancellation token pattern works correctly
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        // Spawn a task that waits for cancellation
        let handle = tokio::spawn(async move {
            cancel_clone.cancelled().await;
            true
        });

        // Cancel immediately
        cancel.cancel();

        // Task should complete quickly
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok(), "task should complete after cancellation");
        assert!(result.unwrap().unwrap(), "task should return true");
    }

    #[test]
    fn test_batch_size_zero_handling() {
        // Zero batch size is technically valid but useless
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(60),
            batch_size_keys: 0,
            max_batches_per_run: 100,
        };
        let keys_per_run = config.batch_size_keys as u64 * config.max_batches_per_run as u64;
        assert_eq!(keys_per_run, 0);
    }

    #[test]
    fn test_max_batches_zero_handling() {
        // Zero max_batches means no cleanup happens per run
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(60),
            batch_size_keys: 1000,
            max_batches_per_run: 0,
        };
        // This is a valid but useless config - cleanup will do nothing
        assert_eq!(config.max_batches_per_run, 0);
    }

    #[test]
    fn test_cleanup_iteration_uses_port_traits() {
        use crate::storage_ports::{KvStateRead, KvStateWrite};
        use crate::storage_shared::SharedStorageError;
        use aspen_kv_types::KeyValueWithRevision;
        use aspen_storage_types::KvEntry;

        struct NoExpiredKeys;

        impl KvStateRead for NoExpiredKeys {
            fn get(&self, _key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
                Ok(None)
            }
            fn get_with_revision(&self, _key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
                Ok(None)
            }
            fn scan(&self, _prefix: &str, _after: Option<&str>, _limit: Option<u32>) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
                Ok(Vec::new())
            }
            fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
                Ok(0)
            }
            fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
                Ok(0)
            }
            fn get_expired_keys_with_metadata(&self, _batch_limit: u32) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
                Ok(Vec::new())
            }
        }

        impl KvStateWrite for NoExpiredKeys {
            fn delete_expired_keys(&self, _batch_limit: u32) -> Result<u32, SharedStorageError> {
                Ok(0)
            }
        }

        let storage = NoExpiredKeys;
        let config = TtlCleanupConfig::default();
        run_cleanup_iteration(&storage, &config);
    }
}
