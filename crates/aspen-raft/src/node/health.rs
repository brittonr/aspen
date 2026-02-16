//! Health monitoring for RaftNode.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use aspen_cluster_types::NodeState;
use tokio::time::Duration;
use tokio::time::interval;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::RaftNode;
use super::conversions::node_state_from_openraft;

/// Health check result with detailed status.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the node is considered healthy overall.
    pub is_healthy: bool,
    /// Current Raft state (Leader, Follower, Candidate, Learner, Shutdown).
    pub state: NodeState,
    /// Current leader ID, if known.
    pub leader: Option<u64>,
    /// Number of consecutive health check failures.
    pub consecutive_failures: u32,
    /// Whether the node is in shutdown state.
    pub is_shutdown: bool,
    /// Whether the node has a committed membership.
    pub has_membership: bool,
}

/// Health monitor for RaftNode.
///
/// Provides periodic health checks without actor overhead.
/// Can be connected to a supervisor for automatic recovery actions.
pub struct RaftNodeHealth {
    node: Arc<RaftNode>,
    /// Consecutive failed health checks
    consecutive_failures: AtomicU32,
    /// Threshold before triggering recovery actions
    failure_threshold: u32,
}

impl RaftNodeHealth {
    /// Create a new health monitor.
    pub fn new(node: Arc<RaftNode>) -> Self {
        Self {
            node,
            consecutive_failures: AtomicU32::new(0),
            failure_threshold: 3, // 3 consecutive failures triggers alert
        }
    }

    /// Create a health monitor with custom failure threshold.
    pub fn with_threshold(node: Arc<RaftNode>, threshold: u32) -> Self {
        Self {
            node,
            consecutive_failures: AtomicU32::new(0),
            failure_threshold: threshold,
        }
    }

    /// Check if the node is healthy.
    pub async fn is_healthy(&self) -> bool {
        // Simple health check: can we get metrics and is there a state?
        let metrics = self.node.raft().metrics();
        let borrowed = metrics.borrow();
        // Check if the node is in any valid state (not just created)
        !matches!(&borrowed.state, openraft::ServerState::Shutdown)
    }

    /// Get detailed health status.
    pub async fn status(&self) -> HealthStatus {
        let metrics = self.node.raft().metrics();
        let borrowed = metrics.borrow();

        let state: NodeState = node_state_from_openraft(borrowed.state);
        let is_shutdown = !state.is_healthy();
        let leader = borrowed.current_leader.map(|id| id.0);
        let has_membership = borrowed.membership_config.membership().voter_ids().next().is_some();

        let consecutive_failures = self.consecutive_failures.load(Ordering::Relaxed);

        HealthStatus {
            is_healthy: !is_shutdown && (has_membership || state == NodeState::Learner),
            state,
            leader,
            consecutive_failures,
            is_shutdown,
            has_membership,
        }
    }

    /// Reset the failure counter (call after recovery).
    pub fn reset_failures(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Run periodic health monitoring with optional supervisor callback.
    ///
    /// When the failure threshold is exceeded, the callback is invoked to
    /// allow the supervisor to take action (e.g., restart services).
    pub async fn monitor_with_callback<F>(&self, interval_secs: u64, mut on_failure: F)
    where F: FnMut(HealthStatus) + Send {
        let mut interval = interval(Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            let status = self.status().await;

            if !status.is_healthy {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

                warn!(
                    node_id = %self.node.node_id(),
                    consecutive_failures = failures,
                    state = ?status.state,
                    "node health check failed"
                );

                if failures >= self.failure_threshold {
                    error!(
                        node_id = %self.node.node_id(),
                        failures = failures,
                        threshold = self.failure_threshold,
                        "health failure threshold exceeded, triggering callback"
                    );
                    on_failure(status);
                }
            } else {
                // Reset failure counter on successful check
                let prev_failures = self.consecutive_failures.swap(0, Ordering::Relaxed);
                if prev_failures > 0 {
                    info!(
                        node_id = %self.node.node_id(),
                        previous_failures = prev_failures,
                        "node recovered, resetting failure count"
                    );
                }
            }
        }
    }

    /// Run periodic health monitoring (simple version without callback).
    pub async fn monitor(&self, interval_secs: u64) {
        self.monitor_with_callback(interval_secs, |_| {
            // No-op callback - just log the failure
        })
        .await
    }
}
