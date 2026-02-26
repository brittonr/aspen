//! Background lease cleanup task for expired lease garbage collection.
//!
//! This module provides a background task that periodically deletes expired leases
//! and their attached keys from the Redb state machine. Similar to etcd's lease
//! expiration, when a lease expires, all keys attached to it are automatically deleted.
//!
//! # Architecture
//!
//! The cleanup task:
//! 1. Runs on a configurable interval (default: 10 seconds)
//! 2. Deletes expired leases and their keys in batches (default: 100 per run)
//! 3. Continues until all expired leases are deleted or max iterations reached
//! 4. Tracks metrics for monitoring (expired leases deleted, remaining, etc.)
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

/// Configuration for the lease cleanup task.
#[derive(Debug, Clone)]
pub struct LeaseCleanupConfig {
    /// Interval between cleanup runs (default: 10 seconds).
    /// Leases typically have shorter TTLs so we check more frequently.
    pub cleanup_interval: Duration,
    /// Maximum leases to delete per batch (default: 100).
    pub batch_size_leases: u32,
    /// Maximum batches per cleanup run (default: 10).
    /// Prevents the cleanup from running indefinitely if there are many expired leases.
    pub max_batches_per_run: u32,
}

impl Default for LeaseCleanupConfig {
    fn default() -> Self {
        Self {
            cleanup_interval: Duration::from_secs(10),
            batch_size_leases: 100,
            max_batches_per_run: 10,
        }
    }
}

/// Background lease cleanup task handle for Redb storage.
///
/// Returns a CancellationToken that can be used to stop the task.
pub fn spawn_redb_lease_cleanup_task(storage: Arc<SharedRedbStorage>, config: LeaseCleanupConfig) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_redb_lease_cleanup_loop(storage, config, cancel_clone).await;
    });

    cancel
}

/// Main cleanup loop for Redb storage.
async fn run_redb_lease_cleanup_loop(
    storage: Arc<SharedRedbStorage>,
    config: LeaseCleanupConfig,
    cancel: CancellationToken,
) {
    let mut ticker = interval(config.cleanup_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_secs = config.cleanup_interval.as_secs(),
        batch_size_leases = config.batch_size_leases,
        max_batches = config.max_batches_per_run,
        "Redb lease cleanup task started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Redb lease cleanup task shutting down");
                break;
            }
            _ = ticker.tick() => {
                run_redb_cleanup_iteration(&storage, &config).await;
            }
        }
    }
}

/// Run a single cleanup iteration for Redb storage.
async fn run_redb_cleanup_iteration(storage: &SharedRedbStorage, config: &LeaseCleanupConfig) {
    let mut total_deleted: u64 = 0;
    let mut batches_run: u32 = 0;

    // Keep deleting batches until no more expired leases or max batches reached
    loop {
        if batches_run >= config.max_batches_per_run {
            debug!(
                total_deleted,
                batches_run,
                max_batches = config.max_batches_per_run,
                "Redb lease cleanup reached max batches limit"
            );
            break;
        }

        match storage.delete_expired_leases(config.batch_size_leases) {
            Ok(deleted) => {
                total_deleted += deleted as u64;
                batches_run += 1;

                if deleted == 0 {
                    // No more expired leases
                    break;
                }

                if deleted < config.batch_size_leases {
                    // Deleted fewer than batch size, so we're done
                    break;
                }
            }
            Err(e) => {
                warn!(error = %e, "Redb lease cleanup batch failed");
                break;
            }
        }
    }

    if total_deleted > 0 {
        // Log metrics for monitoring
        let remaining = storage.count_expired_leases().unwrap_or(0);
        let active = storage.count_active_leases().unwrap_or(0);

        info!(
            total_deleted,
            batches_run,
            remaining_expired = remaining,
            active_leases = active,
            "Redb lease cleanup iteration completed"
        );
    } else {
        debug!("Redb lease cleanup: no expired leases to delete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LeaseCleanupConfig::default();
        assert_eq!(config.cleanup_interval, Duration::from_secs(10));
        assert_eq!(config.batch_size_leases, 100);
        assert_eq!(config.max_batches_per_run, 10);
    }

    #[test]
    fn test_config_custom_values() {
        let config = LeaseCleanupConfig {
            cleanup_interval: Duration::from_secs(5),
            batch_size_leases: 50,
            max_batches_per_run: 20,
        };
        assert_eq!(config.cleanup_interval, Duration::from_secs(5));
        assert_eq!(config.batch_size_leases, 50);
        assert_eq!(config.max_batches_per_run, 20);
    }

    #[test]
    fn test_config_clone() {
        let config = LeaseCleanupConfig::default();
        let cloned = config.clone();
        assert_eq!(config.cleanup_interval, cloned.cleanup_interval);
        assert_eq!(config.batch_size_leases, cloned.batch_size_leases);
        assert_eq!(config.max_batches_per_run, cloned.max_batches_per_run);
    }

    #[test]
    fn test_config_debug_format() {
        let config = LeaseCleanupConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("LeaseCleanupConfig"));
        assert!(debug_str.contains("cleanup_interval"));
        assert!(debug_str.contains("batch_size_leases"));
        assert!(debug_str.contains("max_batches_per_run"));
    }

    #[test]
    fn test_cleanup_capacity_calculation() {
        // Verify the default config can handle reasonable workloads
        let config = LeaseCleanupConfig::default();
        let leases_per_run = config.batch_size_leases as u64 * config.max_batches_per_run as u64;
        // Default: 100 * 10 = 1,000 leases per run
        assert_eq!(leases_per_run, 1_000);

        // At 10 second intervals, that's 100 leases/second capacity
        let leases_per_second = leases_per_run / config.cleanup_interval.as_secs();
        assert!(leases_per_second >= 100, "cleanup capacity should be at least 100 leases/sec");
    }

    #[test]
    fn test_config_aggressive_cleanup() {
        // Config for high-throughput lease workloads
        let config = LeaseCleanupConfig {
            cleanup_interval: Duration::from_secs(1),
            batch_size_leases: 500,
            max_batches_per_run: 100,
        };
        let leases_per_run = config.batch_size_leases as u64 * config.max_batches_per_run as u64;
        // 500 * 100 = 50,000 leases per run
        assert_eq!(leases_per_run, 50_000);
    }

    #[test]
    fn test_config_minimal_cleanup() {
        // Config for low-throughput scenarios
        let config = LeaseCleanupConfig {
            cleanup_interval: Duration::from_secs(60),
            batch_size_leases: 10,
            max_batches_per_run: 5,
        };
        let leases_per_run = config.batch_size_leases as u64 * config.max_batches_per_run as u64;
        // 10 * 5 = 50 leases per run
        assert_eq!(leases_per_run, 50);
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
        let config = LeaseCleanupConfig {
            cleanup_interval: Duration::from_secs(10),
            batch_size_leases: 0,
            max_batches_per_run: 10,
        };
        let leases_per_run = config.batch_size_leases as u64 * config.max_batches_per_run as u64;
        assert_eq!(leases_per_run, 0);
    }

    #[test]
    fn test_max_batches_zero_handling() {
        // Zero max_batches means no cleanup happens per run
        let config = LeaseCleanupConfig {
            cleanup_interval: Duration::from_secs(10),
            batch_size_leases: 100,
            max_batches_per_run: 0,
        };
        // This is a valid but useless config - cleanup will do nothing
        assert_eq!(config.max_batches_per_run, 0);
    }

    #[test]
    fn test_shorter_interval_than_ttl() {
        // Lease cleanup should run more frequently than TTL to catch expirations promptly
        // Default: 10 second interval catches leases expiring shortly after their TTL
        let config = LeaseCleanupConfig::default();
        // Verify interval is reasonable (not longer than a minute)
        assert!(config.cleanup_interval <= Duration::from_secs(60));
    }
}
