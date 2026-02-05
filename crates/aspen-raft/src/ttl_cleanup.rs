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

use crate::storage_shared::SharedRedbStorage;

/// Configuration for the TTL cleanup task.
#[derive(Debug, Clone)]
pub struct TtlCleanupConfig {
    /// Interval between cleanup runs (default: 60 seconds).
    pub cleanup_interval: Duration,
    /// Maximum keys to delete per batch (default: 1000).
    pub batch_size: u32,
    /// Maximum batches per cleanup run (default: 100).
    /// Prevents the cleanup from running indefinitely if there are many expired keys.
    pub max_batches_per_run: u32,
}

impl Default for TtlCleanupConfig {
    fn default() -> Self {
        Self {
            cleanup_interval: Duration::from_secs(60),
            batch_size: 1000,
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
        run_redb_ttl_cleanup_loop(storage, config, cancel_clone).await;
    });

    cancel
}

/// Main cleanup loop for Redb storage.
async fn run_redb_ttl_cleanup_loop(
    storage: Arc<SharedRedbStorage>,
    config: TtlCleanupConfig,
    cancel: CancellationToken,
) {
    let mut ticker = interval(config.cleanup_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_secs = config.cleanup_interval.as_secs(),
        batch_size = config.batch_size,
        max_batches = config.max_batches_per_run,
        "Redb TTL cleanup task started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Redb TTL cleanup task shutting down");
                break;
            }
            _ = ticker.tick() => {
                run_redb_cleanup_iteration(&storage, &config).await;
            }
        }
    }
}

/// Run a single cleanup iteration for Redb storage.
async fn run_redb_cleanup_iteration(storage: &SharedRedbStorage, config: &TtlCleanupConfig) {
    let mut total_deleted: u64 = 0;
    let mut batches_run: u32 = 0;

    // Keep deleting batches until no more expired keys or max batches reached
    loop {
        if batches_run >= config.max_batches_per_run {
            debug!(
                total_deleted,
                batches_run,
                max_batches = config.max_batches_per_run,
                "Redb TTL cleanup reached max batches limit"
            );
            break;
        }

        match storage.delete_expired_keys(config.batch_size) {
            Ok(deleted) => {
                total_deleted += deleted as u64;
                batches_run += 1;

                if deleted == 0 {
                    // No more expired keys
                    break;
                }

                if deleted < config.batch_size {
                    // Deleted fewer than batch size, so we're done
                    break;
                }
            }
            Err(e) => {
                warn!(error = %e, "Redb TTL cleanup batch failed");
                break;
            }
        }
    }

    if total_deleted > 0 {
        // Log metrics for monitoring
        let remaining = storage.count_expired_keys().unwrap_or(0);
        let with_ttl = storage.count_keys_with_ttl().unwrap_or(0);

        info!(
            total_deleted,
            batches_run,
            remaining_expired = remaining,
            keys_with_ttl = with_ttl,
            "Redb TTL cleanup iteration completed"
        );
    } else {
        debug!("Redb TTL cleanup: no expired keys to delete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TtlCleanupConfig::default();
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_batches_per_run, 100);
    }

    #[test]
    fn test_config_custom_values() {
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(30),
            batch_size: 500,
            max_batches_per_run: 50,
        };
        assert_eq!(config.cleanup_interval, Duration::from_secs(30));
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.max_batches_per_run, 50);
    }

    #[test]
    fn test_config_clone() {
        let config = TtlCleanupConfig::default();
        let cloned = config.clone();
        assert_eq!(config.cleanup_interval, cloned.cleanup_interval);
        assert_eq!(config.batch_size, cloned.batch_size);
        assert_eq!(config.max_batches_per_run, cloned.max_batches_per_run);
    }

    #[test]
    fn test_config_debug_format() {
        let config = TtlCleanupConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("TtlCleanupConfig"));
        assert!(debug_str.contains("cleanup_interval"));
        assert!(debug_str.contains("batch_size"));
        assert!(debug_str.contains("max_batches_per_run"));
    }

    #[test]
    fn test_cleanup_capacity_calculation() {
        // Verify the default config can handle reasonable workloads
        let config = TtlCleanupConfig::default();
        let keys_per_run = config.batch_size as u64 * config.max_batches_per_run as u64;
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
            batch_size: 5000,
            max_batches_per_run: 200,
        };
        let keys_per_run = config.batch_size as u64 * config.max_batches_per_run as u64;
        // 5000 * 200 = 1,000,000 keys per run
        assert_eq!(keys_per_run, 1_000_000);
    }

    #[test]
    fn test_config_minimal_cleanup() {
        // Config for low-throughput scenarios
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            batch_size: 100,
            max_batches_per_run: 10,
        };
        let keys_per_run = config.batch_size as u64 * config.max_batches_per_run as u64;
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
            batch_size: 0,
            max_batches_per_run: 100,
        };
        let keys_per_run = config.batch_size as u64 * config.max_batches_per_run as u64;
        assert_eq!(keys_per_run, 0);
    }

    #[test]
    fn test_max_batches_zero_handling() {
        // Zero max_batches means no cleanup happens per run
        let config = TtlCleanupConfig {
            cleanup_interval: Duration::from_secs(60),
            batch_size: 1000,
            max_batches_per_run: 0,
        };
        // This is a valid but useless config - cleanup will do nothing
        assert_eq!(config.max_batches_per_run, 0);
    }
}
