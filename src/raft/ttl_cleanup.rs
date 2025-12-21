//! Background TTL cleanup task for expired key garbage collection.
//!
//! This module provides a background task that periodically deletes expired keys
//! from the SQLite state machine. It follows the "active expiration" pattern
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
use tracing::{debug, info, warn};

use crate::raft::storage_sqlite::SqliteStateMachine;

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

/// Background TTL cleanup task handle.
///
/// Returns a CancellationToken that can be used to stop the task.
pub fn spawn_ttl_cleanup_task(
    state_machine: Arc<SqliteStateMachine>,
    config: TtlCleanupConfig,
) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_ttl_cleanup_loop(state_machine, config, cancel_clone).await;
    });

    cancel
}

/// Main cleanup loop.
async fn run_ttl_cleanup_loop(
    state_machine: Arc<SqliteStateMachine>,
    config: TtlCleanupConfig,
    cancel: CancellationToken,
) {
    let mut ticker = interval(config.cleanup_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_secs = config.cleanup_interval.as_secs(),
        batch_size = config.batch_size,
        max_batches = config.max_batches_per_run,
        "TTL cleanup task started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("TTL cleanup task shutting down");
                break;
            }
            _ = ticker.tick() => {
                run_cleanup_iteration(&state_machine, &config).await;
            }
        }
    }
}

/// Run a single cleanup iteration.
async fn run_cleanup_iteration(state_machine: &SqliteStateMachine, config: &TtlCleanupConfig) {
    let mut total_deleted: u64 = 0;
    let mut batches_run: u32 = 0;

    // Keep deleting batches until no more expired keys or max batches reached
    loop {
        if batches_run >= config.max_batches_per_run {
            debug!(
                total_deleted,
                batches_run,
                max_batches = config.max_batches_per_run,
                "TTL cleanup reached max batches limit"
            );
            break;
        }

        match state_machine.delete_expired_keys(config.batch_size) {
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
                warn!(error = %e, "TTL cleanup batch failed");
                break;
            }
        }
    }

    if total_deleted > 0 {
        // Log metrics for monitoring
        let remaining = state_machine.count_expired_keys().unwrap_or(0);
        let with_ttl = state_machine.count_keys_with_ttl().unwrap_or(0);

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
}
