//! Worker and health monitoring initialization for cluster nodes.
//!
//! This module handles spawning health monitoring background tasks.

use std::sync::Arc;

use aspen_raft::node::RaftNodeHealth;
use aspen_raft::supervisor::Supervisor;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::warn;

use crate::config::NodeConfig;

/// Spawn health monitoring background task.
///
/// Returns the shutdown token for the health monitor.
pub(super) fn spawn_health_monitoring(
    config: &NodeConfig,
    health_monitor: Arc<RaftNodeHealth>,
    supervisor: Arc<Supervisor>,
) -> CancellationToken {
    let health_clone = health_monitor.clone();
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    let supervisor_for_health = supervisor.clone();
    let node_id_for_health = config.node_id;

    // Tiger Style: Outer task runs until shutdown_clone is cancelled. The JoinHandle
    // is not tracked because shutdown is controlled via CancellationToken.
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
        let dropped_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dropped_count_for_callback = Arc::clone(&dropped_count);

        let supervisor_clone = supervisor_for_health.clone();
        let node_id_for_processor = node_id_for_health;
        // Tiger Style: Inner processor task terminates when tx is dropped (channel closes).
        // Its lifetime is bounded by the outer task's lifetime.
        tokio::spawn(async move {
            while let Some(reason) = rx.recv().await {
                supervisor_clone.record_health_failure(&reason).await;

                if !supervisor_clone.should_attempt_recovery().await {
                    error!(
                        node_id = node_id_for_processor,
                        "too many health failures, supervisor circuit breaker triggered"
                    );
                    supervisor_clone.stop();
                    break;
                }
            }
            warn!(node_id = node_id_for_processor, "health failure processor channel closed");
        });

        tokio::select! {
            _ = health_clone.monitor_with_callback(5, move |status| {
                let reason = format!(
                    "health check failed: state={:?}, failures={}",
                    status.state, status.consecutive_failures
                );
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) = tx.try_send(reason) {
                    let count = dropped_count_for_callback
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if count % 10 == 1 {
                        tracing::warn!(
                            dropped_count = count,
                            "health failure channel full, messages being dropped \
                             (health processor may be blocked or overwhelmed)"
                        );
                    }
                }
            }) => {}
            _ = shutdown_clone.cancelled() => {
                let final_drops = dropped_count.load(std::sync::atomic::Ordering::Relaxed);
                if final_drops > 0 {
                    warn!(
                        node_id = node_id_for_health,
                        dropped_messages = final_drops,
                        "health monitor shutdown with dropped messages"
                    );
                }
            }
        }
    });

    shutdown
}
