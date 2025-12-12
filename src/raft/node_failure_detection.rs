//! Node-level failure detection for distinguishing actor crashes from node crashes.
//!
//! This module provides classification of failures to enable appropriate responses:
//! - ActorCrash: Raft heartbeat fails but Iroh connection succeeds → auto-restart
//! - NodeCrash: Both Raft and Iroh fail → alert operator for promotion
//!
//! Tiger Style: Fixed timeouts, bounded tracking (max 1000 nodes), explicit types.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::raft::constants::MAX_UNREACHABLE_NODES;
use crate::raft::types::NodeId;

/// Classification of node failures based on connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureType {
    /// Node is healthy and responsive.
    Healthy,
    /// Raft actor crashed but node is reachable (Iroh connected).
    ///
    /// Indicates local process problem, suitable for auto-restart.
    ActorCrash,
    /// Both Raft and Iroh connections failed.
    ///
    /// Indicates node-level failure, requires operator intervention.
    NodeCrash,
}

/// Connection status for Raft heartbeat or Iroh transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connection is active and healthy.
    Connected,
    /// Connection is broken or unreachable.
    Disconnected,
}

/// Internal tracking for unreachable nodes.
#[derive(Debug, Clone)]
struct UnreachableInfo {
    /// When the node first became unreachable.
    first_failed_at: Instant,
    /// Current Raft heartbeat connection status.
    raft_heartbeat_status: ConnectionStatus,
    /// Current Iroh P2P connection status.
    iroh_connection_status: ConnectionStatus,
    /// Classified failure type based on both statuses.
    failure_type: FailureType,
}

/// Detects and classifies node-level failures vs actor-level crashes.
///
/// Uses two independent signals:
/// 1. Raft heartbeat status (from openraft RPC failures)
/// 2. Iroh connection status (from P2P transport layer)
///
/// Classification logic:
/// - Raft OK → Healthy (regardless of Iroh)
/// - Raft fail + Iroh OK → ActorCrash
/// - Raft fail + Iroh fail → NodeCrash
///
/// Tiger Style: Fixed thresholds (60s), bounded tracking (1000 nodes).
pub struct NodeFailureDetector {
    /// Map of currently unreachable nodes with failure details.
    unreachable_nodes: HashMap<NodeId, UnreachableInfo>,
    /// Timeout before considering a node failed (60 seconds).
    _failure_timeout: Duration,
    /// Threshold for alerting operator (60 seconds).
    alert_threshold: Duration,
}

impl NodeFailureDetector {
    /// Create a new failure detector with the given timeout.
    ///
    /// Tiger Style: Explicit timeout parameter, defaults to 60 seconds for both
    /// failure timeout and alert threshold.
    pub fn new(failure_timeout: Duration) -> Self {
        Self {
            unreachable_nodes: HashMap::new(),
            _failure_timeout: failure_timeout,
            alert_threshold: failure_timeout,
        }
    }

    /// Create a failure detector with default 60-second timeout.
    pub fn default_timeout() -> Self {
        Self::new(Duration::from_secs(60))
    }

    /// Update node status based on Raft heartbeat and Iroh connection.
    ///
    /// This should be called whenever connection status changes are detected:
    /// - After Raft RPC success/failure
    /// - When Iroh connection events occur
    ///
    /// Tiger Style: Bounded map size, fail fast if limit exceeded.
    pub fn update_node_status(
        &mut self,
        node_id: impl Into<NodeId>,
        raft_heartbeat: ConnectionStatus,
        iroh_connection: ConnectionStatus,
    ) {
        let node_id = node_id.into();
        let failure_type = self.classify_failure(raft_heartbeat, iroh_connection);

        match failure_type {
            FailureType::Healthy => {
                // Node recovered, remove from unreachable map
                if let Some(info) = self.unreachable_nodes.remove(&node_id) {
                    tracing::info!(
                        node_id = %node_id,
                        unreachable_duration_ms = info.first_failed_at.elapsed().as_millis(),
                        previous_failure_type = ?info.failure_type,
                        "node recovered"
                    );
                }
            }
            FailureType::ActorCrash | FailureType::NodeCrash => {
                // Tiger Style: Enforce bounded map size before insertion
                // Check if we need to evict before the entry() call to avoid borrow checker issues
                if !self.unreachable_nodes.contains_key(&node_id)
                    && self.unreachable_nodes.len() >= MAX_UNREACHABLE_NODES as usize
                {
                    tracing::error!(
                        max_unreachable_nodes = MAX_UNREACHABLE_NODES,
                        "unreachable nodes map at capacity, oldest entries will be evicted"
                    );
                    // Evict oldest entry by elapsed time
                    if let Some(oldest_node_id) = self.find_oldest_unreachable_node() {
                        self.unreachable_nodes.remove(&oldest_node_id);
                    }
                }

                // Node is failing, track it or update existing entry
                self.unreachable_nodes
                    .entry(node_id)
                    .and_modify(|info| {
                        // Update statuses but preserve first_failed_at timestamp
                        info.raft_heartbeat_status = raft_heartbeat;
                        info.iroh_connection_status = iroh_connection;
                        info.failure_type = failure_type;
                    })
                    .or_insert_with(|| {
                        tracing::warn!(
                            node_id = %node_id,
                            failure_type = ?failure_type,
                            raft_heartbeat = ?raft_heartbeat,
                            iroh_connection = ?iroh_connection,
                            "node now unreachable"
                        );

                        UnreachableInfo {
                            first_failed_at: Instant::now(),
                            raft_heartbeat_status: raft_heartbeat,
                            iroh_connection_status: iroh_connection,
                            failure_type,
                        }
                    });
            }
        }
    }

    /// Classify failure based on connection statuses.
    ///
    /// Classification truth table:
    /// | Raft      | Iroh          | Result      |
    /// |-----------|---------------|-------------|
    /// | Connected | *             | Healthy     |
    /// | Disconnected | Connected  | ActorCrash  |
    /// | Disconnected | Disconnected | NodeCrash |
    pub fn classify_failure(
        &self,
        raft_heartbeat: ConnectionStatus,
        iroh_connection: ConnectionStatus,
    ) -> FailureType {
        match (raft_heartbeat, iroh_connection) {
            (ConnectionStatus::Connected, _) => FailureType::Healthy,
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected) => {
                FailureType::ActorCrash
            }
            (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected) => {
                FailureType::NodeCrash
            }
        }
    }

    /// Get nodes that need operator intervention (unreachable > alert_threshold).
    ///
    /// Returns list of (node_id, failure_type, unreachable_duration) tuples
    /// for nodes that have been unreachable beyond the alert threshold.
    pub fn get_nodes_needing_attention(&self) -> Vec<(NodeId, FailureType, Duration)> {
        self.unreachable_nodes
            .iter()
            .filter(|(_, info)| info.first_failed_at.elapsed() >= self.alert_threshold)
            .map(|(node_id, info)| (*node_id, info.failure_type, info.first_failed_at.elapsed()))
            .collect()
    }

    /// Get current failure type for a node.
    ///
    /// Returns Healthy if the node is not in the unreachable map.
    pub fn get_failure_type(&self, node_id: impl Into<NodeId>) -> FailureType {
        let node_id = node_id.into();
        self.unreachable_nodes
            .get(&node_id)
            .map(|info| info.failure_type)
            .unwrap_or(FailureType::Healthy)
    }

    /// Get unreachable duration for a node.
    ///
    /// Returns None if the node is not currently unreachable.
    pub fn get_unreachable_duration(&self, node_id: impl Into<NodeId>) -> Option<Duration> {
        let node_id = node_id.into();
        self.unreachable_nodes
            .get(&node_id)
            .map(|info| info.first_failed_at.elapsed())
    }

    /// Get count of currently unreachable nodes.
    pub fn unreachable_count(&self) -> u32 {
        self.unreachable_nodes.len() as u32
    }

    /// Find the oldest unreachable node by elapsed time.
    ///
    /// Used for eviction when MAX_UNREACHABLE_NODES is reached.
    fn find_oldest_unreachable_node(&self) -> Option<NodeId> {
        self.unreachable_nodes
            .iter()
            .max_by_key(|(_, info)| info.first_failed_at.elapsed())
            .map(|(node_id, _)| *node_id)
    }

    /// Clear all unreachable node tracking (used for testing).
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.unreachable_nodes.clear();
    }
}

/// Alert manager for notifying operators about node failures.
///
/// Tracks which nodes have been alerted to avoid spam. When a node
/// needs attention (unreachable > threshold), it fires an alert once.
/// Alerts are cleared when nodes recover.
///
/// Tiger Style: Bounded alert tracking (max 1000 nodes).
pub struct AlertManager {
    /// Set of node IDs that have already been alerted.
    alerted_nodes: std::collections::HashSet<NodeId>,
}

impl AlertManager {
    /// Create a new alert manager.
    pub fn new() -> Self {
        Self {
            alerted_nodes: std::collections::HashSet::new(),
        }
    }

    /// Check for nodes needing attention and fire alerts if not already alerted.
    ///
    /// This should be called periodically (e.g., every 10-30 seconds) to
    /// monitor for new node failures requiring operator intervention.
    ///
    /// Tiger Style: Bounded alert set (enforced by detector's MAX_UNREACHABLE_NODES).
    pub fn check_and_alert(&mut self, detector: &NodeFailureDetector) {
        for (node_id, failure_type, duration) in detector.get_nodes_needing_attention() {
            if !self.alerted_nodes.contains(&node_id) {
                // Fire alert (log for now, could integrate with PagerDuty/Slack/etc)
                tracing::error!(
                    node_id = %node_id,
                    failure_type = ?failure_type,
                    unreachable_duration_s = duration.as_secs(),
                    "ALERT: Node has been unreachable beyond threshold. Operator intervention required."
                );

                // Mark as alerted to avoid spam
                self.alerted_nodes.insert(node_id);
            }
        }

        // Clear alerts for recovered nodes
        self.alerted_nodes
            .retain(|node_id| detector.get_failure_type(*node_id) != FailureType::Healthy);
    }

    /// Get count of currently active alerts.
    pub fn active_alert_count(&self) -> u32 {
        self.alerted_nodes.len() as u32
    }

    /// Clear all alerts (used for testing).
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.alerted_nodes.clear();
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_healthy_node() {
        let detector = NodeFailureDetector::default_timeout();

        // Both connected → Healthy
        assert_eq!(
            detector.classify_failure(ConnectionStatus::Connected, ConnectionStatus::Connected),
            FailureType::Healthy
        );

        // Raft connected, Iroh disconnected → Healthy (Raft takes precedence)
        assert_eq!(
            detector.classify_failure(ConnectionStatus::Connected, ConnectionStatus::Disconnected),
            FailureType::Healthy
        );
    }

    #[test]
    fn test_classify_actor_crash() {
        let detector = NodeFailureDetector::default_timeout();

        // Raft disconnected, Iroh connected → ActorCrash
        assert_eq!(
            detector.classify_failure(ConnectionStatus::Disconnected, ConnectionStatus::Connected),
            FailureType::ActorCrash
        );
    }

    #[test]
    fn test_classify_node_crash() {
        let detector = NodeFailureDetector::default_timeout();

        // Both disconnected → NodeCrash
        assert_eq!(
            detector.classify_failure(
                ConnectionStatus::Disconnected,
                ConnectionStatus::Disconnected
            ),
            FailureType::NodeCrash
        );
    }

    #[test]
    fn test_detect_actor_crash_scenario() {
        let mut detector = NodeFailureDetector::default_timeout();
        let node_id = 42;

        // Initial state: healthy
        assert_eq!(detector.get_failure_type(node_id), FailureType::Healthy);
        assert_eq!(detector.unreachable_count(), 0);

        // Simulate actor crash: Raft fails, Iroh OK
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Connected,
        );

        // Should be classified as ActorCrash
        assert_eq!(detector.get_failure_type(node_id), FailureType::ActorCrash);
        assert_eq!(detector.unreachable_count(), 1);

        // Should have unreachable duration
        assert!(detector.get_unreachable_duration(node_id).is_some());
    }

    #[test]
    fn test_detect_node_crash_scenario() {
        let mut detector = NodeFailureDetector::default_timeout();
        let node_id = 99;

        // Simulate node crash: both fail
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );

        // Should be classified as NodeCrash
        assert_eq!(detector.get_failure_type(node_id), FailureType::NodeCrash);
        assert_eq!(detector.unreachable_count(), 1);
    }

    #[test]
    fn test_recovery_clears_unreachable_state() {
        let mut detector = NodeFailureDetector::default_timeout();
        let node_id = 7;

        // Node fails
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );
        assert_eq!(detector.unreachable_count(), 1);

        // Node recovers
        detector.update_node_status(
            node_id,
            ConnectionStatus::Connected,
            ConnectionStatus::Connected,
        );

        // Should be removed from tracking
        assert_eq!(detector.get_failure_type(node_id), FailureType::Healthy);
        assert_eq!(detector.unreachable_count(), 0);
        assert!(detector.get_unreachable_duration(node_id).is_none());
    }

    #[test]
    fn test_alert_threshold() {
        let mut detector = NodeFailureDetector::new(Duration::from_millis(100));
        let node_id: NodeId = 5.into();

        // Node fails
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );

        // Initially no alerts (just failed)
        assert_eq!(detector.get_nodes_needing_attention().len(), 0);

        // Wait for alert threshold
        std::thread::sleep(Duration::from_millis(150));

        // Now should need attention
        let alerts = detector.get_nodes_needing_attention();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].0, node_id);
        assert_eq!(alerts[0].1, FailureType::NodeCrash);
        assert!(alerts[0].2 >= Duration::from_millis(100));
    }

    #[test]
    fn test_max_unreachable_nodes_bounded() {
        let mut detector = NodeFailureDetector::default_timeout();

        // Fill beyond max capacity
        for node_id in 0..MAX_UNREACHABLE_NODES + 10 {
            detector.update_node_status(
                node_id as u64,
                ConnectionStatus::Disconnected,
                ConnectionStatus::Disconnected,
            );
        }

        // Should be capped at MAX_UNREACHABLE_NODES
        assert_eq!(detector.unreachable_count(), MAX_UNREACHABLE_NODES);
    }

    #[test]
    fn test_status_update_preserves_timestamp() {
        let mut detector = NodeFailureDetector::default_timeout();
        let node_id = 123;

        // Initial failure
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );

        let initial_duration = detector.get_unreachable_duration(node_id).unwrap();

        // Wait a bit
        std::thread::sleep(Duration::from_millis(50));

        // Update status (from NodeCrash to ActorCrash)
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Connected,
        );

        // Duration should have increased, not reset
        let updated_duration = detector.get_unreachable_duration(node_id).unwrap();
        assert!(updated_duration > initial_duration);

        // Failure type should have changed
        assert_eq!(detector.get_failure_type(node_id), FailureType::ActorCrash);
    }

    #[test]
    fn test_alert_manager_fires_once() {
        let mut detector = NodeFailureDetector::new(Duration::from_millis(50));
        let mut alert_mgr = AlertManager::new();
        let node_id = 10;

        // Node fails
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );

        // Wait for alert threshold
        std::thread::sleep(Duration::from_millis(100));

        // First check should fire alert
        assert_eq!(alert_mgr.active_alert_count(), 0);
        alert_mgr.check_and_alert(&detector);
        assert_eq!(alert_mgr.active_alert_count(), 1);

        // Second check should not fire again (already alerted)
        alert_mgr.check_and_alert(&detector);
        assert_eq!(alert_mgr.active_alert_count(), 1);
    }

    #[test]
    fn test_alert_manager_clears_on_recovery() {
        let mut detector = NodeFailureDetector::new(Duration::from_millis(50));
        let mut alert_mgr = AlertManager::new();
        let node_id = 20;

        // Node fails
        detector.update_node_status(
            node_id,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected,
        );

        // Wait and fire alert
        std::thread::sleep(Duration::from_millis(100));
        alert_mgr.check_and_alert(&detector);
        assert_eq!(alert_mgr.active_alert_count(), 1);

        // Node recovers
        detector.update_node_status(
            node_id,
            ConnectionStatus::Connected,
            ConnectionStatus::Connected,
        );

        // Alert should be cleared
        alert_mgr.check_and_alert(&detector);
        assert_eq!(alert_mgr.active_alert_count(), 0);
    }

    #[test]
    fn test_alert_manager_multiple_nodes() {
        let mut detector = NodeFailureDetector::new(Duration::from_millis(50));
        let mut alert_mgr = AlertManager::new();

        // Three nodes fail
        for node_id in [30, 31, 32] {
            detector.update_node_status(
                node_id,
                ConnectionStatus::Disconnected,
                ConnectionStatus::Disconnected,
            );
        }

        // Wait for alert threshold
        std::thread::sleep(Duration::from_millis(100));

        // All three should be alerted
        alert_mgr.check_and_alert(&detector);
        assert_eq!(alert_mgr.active_alert_count(), 3);

        // One node recovers
        detector.update_node_status(30, ConnectionStatus::Connected, ConnectionStatus::Connected);

        // Should have 2 active alerts
        alert_mgr.check_and_alert(&detector);
        assert_eq!(alert_mgr.active_alert_count(), 2);
    }
}
