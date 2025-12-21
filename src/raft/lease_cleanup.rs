//! Background lease cleanup task for expired lease garbage collection.
//!
//! This module provides a background task that periodically deletes expired leases
//! and their attached keys from the SQLite state machine. Similar to etcd's lease
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
use tracing::{debug, info, warn};

use crate::raft::storage_sqlite::SqliteStateMachine;

/// Configuration for the lease cleanup task.
#[derive(Debug, Clone)]
pub struct LeaseCleanupConfig {
    /// Interval between cleanup runs (default: 10 seconds).
    /// Leases typically have shorter TTLs so we check more frequently.
    pub cleanup_interval: Duration,
    /// Maximum leases to delete per batch (default: 100).
    pub batch_size: u32,
    /// Maximum batches per cleanup run (default: 10).
    /// Prevents the cleanup from running indefinitely if there are many expired leases.
    pub max_batches_per_run: u32,
}

impl Default for LeaseCleanupConfig {
    fn default() -> Self {
        Self {
            cleanup_interval: Duration::from_secs(10),
            batch_size: 100,
            max_batches_per_run: 10,
        }
    }
}

/// Background lease cleanup task handle.
///
/// Returns a CancellationToken that can be used to stop the task.
pub fn spawn_lease_cleanup_task(
    state_machine: Arc<SqliteStateMachine>,
    config: LeaseCleanupConfig,
) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_lease_cleanup_loop(state_machine, config, cancel_clone).await;
    });

    cancel
}

/// Main cleanup loop.
async fn run_lease_cleanup_loop(
    state_machine: Arc<SqliteStateMachine>,
    config: LeaseCleanupConfig,
    cancel: CancellationToken,
) {
    let mut ticker = interval(config.cleanup_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_secs = config.cleanup_interval.as_secs(),
        batch_size = config.batch_size,
        max_batches = config.max_batches_per_run,
        "Lease cleanup task started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Lease cleanup task shutting down");
                break;
            }
            _ = ticker.tick() => {
                run_cleanup_iteration(&state_machine, &config).await;
            }
        }
    }
}

/// Run a single cleanup iteration.
async fn run_cleanup_iteration(state_machine: &SqliteStateMachine, config: &LeaseCleanupConfig) {
    let mut total_deleted: u64 = 0;
    let mut batches_run: u32 = 0;

    // Keep deleting batches until no more expired leases or max batches reached
    loop {
        if batches_run >= config.max_batches_per_run {
            debug!(
                total_deleted,
                batches_run,
                max_batches = config.max_batches_per_run,
                "Lease cleanup reached max batches limit"
            );
            break;
        }

        match state_machine.delete_expired_leases(config.batch_size) {
            Ok(deleted) => {
                total_deleted += deleted as u64;
                batches_run += 1;

                if deleted == 0 {
                    // No more expired leases
                    break;
                }

                if deleted < config.batch_size {
                    // Deleted fewer than batch size, so we're done
                    break;
                }
            }
            Err(e) => {
                warn!(error = %e, "Lease cleanup batch failed");
                break;
            }
        }
    }

    if total_deleted > 0 {
        // Log metrics for monitoring
        let remaining = state_machine.count_expired_leases().unwrap_or(0);
        let active = state_machine.count_active_leases().unwrap_or(0);

        info!(
            total_deleted,
            batches_run,
            remaining_expired = remaining,
            active_leases = active,
            "Lease cleanup iteration completed"
        );
    } else {
        debug!("Lease cleanup: no expired leases to delete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LeaseCleanupConfig::default();
        assert_eq!(config.cleanup_interval, Duration::from_secs(10));
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_batches_per_run, 10);
    }
}
