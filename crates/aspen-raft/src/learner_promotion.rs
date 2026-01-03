//! Manual learner promotion API for cluster membership changes.
//!
//! Provides safe, operator-controlled promotion of learner nodes to voters
//! when failures occur. This is Phase 5 of the Actor Supervision system.
//!
//! # Safety Mechanisms
//!
//! - Membership cooldown (300s between changes)
//! - Learner health verification (via NodeFailureDetector)
//! - Log catchup verification (<100 entries behind leader)
//! - Quorum verification (cluster maintains quorum after change)
//! - Maximum voters limit (100 nodes)
//!
//! # HTTP API Usage
//!
//! ## Basic Promotion
//!
//! ```bash
//! curl -X POST http://localhost:8081/admin/promote-learner \
//!   -H "Content-Type: application/json" \
//!   -d '{"learner_id": 4}'
//! ```
//!
//! ## Replace Failed Voter
//!
//! ```bash
//! curl -X POST http://localhost:8081/admin/promote-learner \
//!   -H "Content-Type: application/json" \
//!   -d '{"learner_id": 5, "replace_node": 2}'
//! ```
//!
//! ## Force Promotion (skip safety checks)
//!
//! ```bash
//! curl -X POST http://localhost:8081/admin/promote-learner \
//!   -H "Content-Type: application/json" \
//!   -d '{"learner_id": 4, "force": true}'
//! ```
//!
//! # Response Format
//!
//! Success: `{"success": true, "learner_id": 4, "previous_voters": [1,2,3], "new_voters":
//! [1,2,3,4]}`
//!
//! Error: `HTTP 400 Bad Request` with error message
//!
//! Tiger Style: Fixed timeouts, bounded membership (max 100 voters), explicit types.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use snafu::Snafu;
use tokio::sync::RwLock;

use crate::constants::LEARNER_LAG_THRESHOLD;
use crate::constants::MAX_VOTERS;
use crate::constants::MEMBERSHIP_COOLDOWN;
use crate::node_failure_detection::FailureType;
use crate::node_failure_detection::NodeFailureDetector;
use crate::types::NodeId;
use aspen_core::ChangeMembershipRequest;
use aspen_core::ClusterController;

/// Errors that can occur during learner promotion.
#[derive(Debug, Snafu)]
pub enum PromotionError {
    /// Learner is not healthy or reachable according to failure detector.
    #[snafu(display("Learner {} is not healthy or reachable", learner_id))]
    LearnerUnhealthy {
        /// ID of the unhealthy learner.
        learner_id: NodeId,
    },

    /// Learner's log is too far behind the leader to safely promote.
    #[snafu(display("Learner {} is too far behind (lag: {} entries)", learner_id, lag))]
    LearnerLagging {
        /// ID of the lagging learner.
        learner_id: NodeId,
        /// Number of log entries the learner is behind.
        lag: u64,
    },

    /// Cluster would lose quorum if the failed node is removed.
    #[snafu(display("Cluster lacks quorum without failed node {:?}", failed_node_id))]
    NoQuorumWithoutFailedNode {
        /// ID of the failed node being replaced.
        failed_node_id: Option<NodeId>,
    },

    /// Membership change attempted during cooldown period.
    #[snafu(display("Membership was changed recently (wait {:?})", remaining))]
    MembershipChangeTooRecent {
        /// Time remaining in the cooldown period.
        remaining: Duration,
    },

    /// Promoting this learner would exceed the maximum voter limit.
    #[snafu(display("Maximum voters ({}) would be exceeded", MAX_VOTERS))]
    MaxVotersExceeded,

    /// Specified learner does not exist in the cluster.
    #[snafu(display("Learner {} not found in cluster", learner_id))]
    LearnerNotFound {
        /// ID of the learner that was not found.
        learner_id: NodeId,
    },

    /// Specified node is already a voter, not a learner.
    #[snafu(display("Node {} is not a learner (already a voter)", node_id))]
    NotALearner {
        /// ID of the node that is already a voter.
        node_id: NodeId,
    },

    /// Failed to retrieve cluster metrics from Raft.
    #[snafu(display("Failed to get cluster metrics: {}", source))]
    MetricsError {
        /// Underlying error from the metrics operation.
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to execute the membership change operation.
    #[snafu(display("Failed to change membership: {}", source))]
    MembershipChangeError {
        /// Underlying error from the membership change operation.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Request to promote a learner to voter.
#[derive(Debug, Clone)]
pub struct PromotionRequest {
    /// ID of the learner to promote.
    pub learner_id: NodeId,
    /// Optional voter to replace (if promoting to replace failed node).
    pub replace_node: Option<NodeId>,
    /// Skip safety checks (use with extreme caution).
    pub force: bool,
}

/// Result of a successful learner promotion.
#[derive(Debug, Clone)]
pub struct PromotionResult {
    /// ID of the promoted learner.
    pub learner_id: NodeId,
    /// Previous voter set before promotion.
    pub previous_voters: Vec<NodeId>,
    /// New voter set after promotion.
    pub new_voters: Vec<NodeId>,
    /// Timestamp when promotion completed.
    pub timestamp: Instant,
}

/// Coordinator for manual learner promotion operations.
///
/// Enforces safety checks and cooldown periods to prevent unsafe membership
/// changes. Integrates with NodeFailureDetector to verify learner health.
pub struct LearnerPromotionCoordinator<C>
where
    C: ClusterController,
{
    /// Cluster controller for membership operations.
    cluster_controller: Arc<C>,
    /// Optional failure detector for health checks.
    failure_detector: Option<Arc<RwLock<NodeFailureDetector>>>,
    /// Timestamp of last membership change.
    last_membership_change: Arc<RwLock<Option<Instant>>>,
    /// Cooldown period between membership changes.
    membership_cooldown: Duration,
}

impl<C> LearnerPromotionCoordinator<C>
where
    C: ClusterController + 'static,
{
    /// Create a new promotion coordinator with the given cluster controller.
    pub fn new(cluster_controller: Arc<C>) -> Self {
        Self {
            cluster_controller,
            failure_detector: None,
            last_membership_change: Arc::new(RwLock::new(None)),
            membership_cooldown: MEMBERSHIP_COOLDOWN,
        }
    }

    /// Create a coordinator with a failure detector for health checks.
    pub fn with_failure_detector(
        cluster_controller: Arc<C>,
        failure_detector: Arc<RwLock<NodeFailureDetector>>,
    ) -> Self {
        Self {
            cluster_controller,
            failure_detector: Some(failure_detector),
            last_membership_change: Arc::new(RwLock::new(None)),
            membership_cooldown: MEMBERSHIP_COOLDOWN,
        }
    }

    /// Promote a learner to voter with safety validation.
    ///
    /// Safety checks performed (unless force=true):
    /// 1. Membership cooldown elapsed (300s since last change)
    /// 2. Learner is healthy and reachable
    /// 3. Learner is caught up on log (<100 entries behind)
    /// 4. Cluster will maintain quorum after change
    /// 5. Maximum voters limit not exceeded
    ///
    /// Tiger Style: Fail-fast on validation errors, explicit error types.
    pub async fn promote_learner(&self, request: PromotionRequest) -> Result<PromotionResult, PromotionError> {
        // Safety check 1: Membership cooldown
        if !request.force {
            self.check_membership_cooldown().await?;
        }

        // Get current membership state
        let metrics = self
            .cluster_controller
            .get_metrics()
            .await
            .map_err(|e| PromotionError::MetricsError { source: Box::new(e) })?;

        let current_voters: Vec<NodeId> = metrics.voters.iter().map(|id| NodeId(*id)).collect();
        let current_learners: Vec<NodeId> = metrics.learners.iter().map(|id| NodeId(*id)).collect();

        // Verify learner exists and is actually a learner
        if current_voters.contains(&request.learner_id) {
            return Err(PromotionError::NotALearner {
                node_id: request.learner_id,
            });
        }
        if !current_learners.contains(&request.learner_id) {
            return Err(PromotionError::LearnerNotFound {
                learner_id: request.learner_id,
            });
        }

        // Safety check 2: Verify learner is healthy (unless forced)
        if !request.force {
            self.verify_learner_healthy(request.learner_id).await?;
        }

        // Safety check 3: Verify learner is caught up on log (unless forced)
        if !request.force {
            self.verify_learner_caught_up(request.learner_id, &metrics).await?;
        }

        // Safety check 4: Verify quorum will be maintained
        if !request.force {
            self.verify_quorum(&current_voters, request.replace_node).await?;
        }

        // Build new membership set
        let new_voters = self.build_new_membership(&current_voters, request.learner_id, request.replace_node)?;

        // Execute membership change (convert NodeId to u64 for API boundary)
        let change_request = ChangeMembershipRequest {
            members: new_voters.iter().map(|id| (*id).into()).collect(),
        };

        self.cluster_controller
            .change_membership(change_request)
            .await
            .map_err(|e| PromotionError::MembershipChangeError { source: Box::new(e) })?;

        // Update last change timestamp
        *self.last_membership_change.write().await = Some(Instant::now());

        tracing::info!(
            learner_id = %request.learner_id,
            replaced_node = ?request.replace_node,
            previous_voters = ?current_voters,
            new_voters = ?new_voters,
            "successfully promoted learner to voter"
        );

        Ok(PromotionResult {
            learner_id: request.learner_id,
            previous_voters: current_voters,
            new_voters,
            timestamp: Instant::now(),
        })
    }

    /// Check if membership cooldown has elapsed.
    ///
    /// Tiger Style: Explicit Duration type, fixed 300s cooldown.
    async fn check_membership_cooldown(&self) -> Result<(), PromotionError> {
        if let Some(last_change) = *self.last_membership_change.read().await {
            let elapsed = last_change.elapsed();
            if elapsed < self.membership_cooldown {
                let remaining = self.membership_cooldown - elapsed;
                return Err(PromotionError::MembershipChangeTooRecent { remaining });
            }
        }
        Ok(())
    }

    /// Verify learner is healthy and reachable.
    ///
    /// Queries NodeFailureDetector if available, otherwise succeeds.
    ///
    /// # Parameters
    ///
    /// * `learner_id` - ID of the learner node to verify.
    async fn verify_learner_healthy(&self, learner_id: impl Into<NodeId>) -> Result<(), PromotionError> {
        let learner_id = learner_id.into();
        if let Some(ref detector) = self.failure_detector {
            let detector_guard = detector.read().await;
            let failure_type = detector_guard.get_failure_type(learner_id);

            if failure_type != FailureType::Healthy {
                return Err(PromotionError::LearnerUnhealthy { learner_id });
            }
        }

        // If no failure detector, assume healthy (permissive default)
        Ok(())
    }

    /// Verify learner is caught up on the log.
    ///
    /// Checks replication lag from Raft metrics. Learner must be within
    /// LEARNER_LAG_THRESHOLD (100 entries) of the leader's log.
    ///
    /// # Parameters
    ///
    /// * `learner_id` - ID of the learner node to verify.
    /// * `metrics` - Current cluster metrics containing replication state.
    async fn verify_learner_caught_up(
        &self,
        learner_id: NodeId,
        metrics: &aspen_core::ClusterMetrics,
    ) -> Result<(), PromotionError> {
        // Only the leader has replication metrics
        if let Some(ref replication) = metrics.replication {
            // ClusterMetrics uses u64 keys instead of NodeId
            if let Some(matched_index) = replication.get(&learner_id.0) {
                if let (Some(leader_last_log), Some(matched)) = (metrics.last_log_index, *matched_index) {
                    let lag = leader_last_log.saturating_sub(matched);
                    if lag > LEARNER_LAG_THRESHOLD {
                        return Err(PromotionError::LearnerLagging { learner_id, lag });
                    }
                }
            } else {
                // Learner has no replication tracking - likely not syncing yet
                return Err(PromotionError::LearnerLagging {
                    learner_id,
                    lag: u64::MAX,
                });
            }
        }

        // If not the leader or no replication data, assume caught up (permissive)
        Ok(())
    }

    /// Verify cluster will maintain quorum after membership change.
    ///
    /// Quorum = (voters / 2) + 1. When replacing a node, verify the cluster
    /// has quorum without the replaced node.
    ///
    /// # Parameters
    ///
    /// * `current_voters` - Current set of voting nodes in the cluster.
    /// * `replace_node` - Optional node being replaced (if any).
    async fn verify_quorum(
        &self,
        current_voters: &[NodeId],
        replace_node: Option<NodeId>,
    ) -> Result<(), PromotionError> {
        if let Some(failed_node) = replace_node {
            // Check that we have quorum without the failed node
            let remaining_voters = current_voters.len() - 1;
            // Quorum is based on ORIGINAL cluster size, not remaining nodes
            let quorum_size = (current_voters.len() / 2) + 1;

            // Need at least quorum_size nodes to be reachable
            // For now, we assume all remaining nodes are healthy
            // (proper implementation would query failure detector for each)
            if remaining_voters < quorum_size {
                return Err(PromotionError::NoQuorumWithoutFailedNode {
                    failed_node_id: Some(failed_node),
                });
            }
        }

        Ok(())
    }

    /// Build new membership set by promoting learner and optionally replacing a voter.
    ///
    /// Tiger Style: Bounded membership (max 100 voters), deterministic ordering.
    ///
    /// # Parameters
    ///
    /// * `current` - Current set of voting nodes.
    /// * `promote` - Learner node to promote to voter.
    /// * `replace` - Optional voter to remove (if replacing a failed node).
    fn build_new_membership(
        &self,
        current: &[NodeId],
        promote: NodeId,
        replace: Option<NodeId>,
    ) -> Result<Vec<NodeId>, PromotionError> {
        let mut new_members: Vec<NodeId> = current
            .iter()
            .filter(|&&id| Some(id) != replace) // Remove replaced node
            .copied()
            .collect();

        // Add promoted learner if not already in voter set
        if !new_members.contains(&promote) {
            new_members.push(promote);
        }

        // Tiger Style: Enforce maximum voters limit
        if new_members.len() > MAX_VOTERS as usize {
            return Err(PromotionError::MaxVotersExceeded);
        }

        // Deterministic ordering for reproducibility
        new_members.sort();

        Ok(new_members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspen_core::inmemory::DeterministicClusterController;

    #[tokio::test]
    async fn test_build_new_membership_promotes_learner() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        let current = vec![1.into(), 2.into(), 3.into()];
        let new = coordinator.build_new_membership(&current, 4.into(), None).unwrap();

        assert_eq!(new, vec![1.into(), 2.into(), 3.into(), 4.into()]);
    }

    #[tokio::test]
    async fn test_build_new_membership_replaces_node() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        let current = vec![1.into(), 2.into(), 3.into()];
        let new = coordinator.build_new_membership(&current, 4.into(), Some(2.into())).unwrap();

        assert_eq!(new, vec![1.into(), 3.into(), 4.into()]);
    }

    #[tokio::test]
    async fn test_build_new_membership_no_duplicate() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Learner is already in voter set (edge case)
        let current: Vec<NodeId> = vec![1.into(), 2.into(), 3.into(), 4.into()];
        let new = coordinator.build_new_membership(&current, 4.into(), None).unwrap();

        // Should not add duplicate
        let expected: Vec<NodeId> = vec![1.into(), 2.into(), 3.into(), 4.into()];
        assert_eq!(new, expected);
    }

    #[tokio::test]
    async fn test_membership_cooldown_enforced() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Simulate a recent membership change
        *coordinator.last_membership_change.write().await = Some(Instant::now());

        // Try to promote immediately (should fail)
        let result = coordinator.check_membership_cooldown().await;
        assert!(matches!(result, Err(PromotionError::MembershipChangeTooRecent { .. })));
    }

    #[tokio::test]
    async fn test_membership_cooldown_elapsed() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Simulate a change that happened long ago
        *coordinator.last_membership_change.write().await = Some(Instant::now() - Duration::from_secs(400));

        // Should succeed (cooldown elapsed)
        let result = coordinator.check_membership_cooldown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_max_voters_enforced() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Create membership at max capacity
        let current: Vec<NodeId> = (1..=MAX_VOTERS as u64).map(NodeId::from).collect();

        // Try to add one more voter
        let result = coordinator.build_new_membership(&current, 999.into(), None);

        assert!(matches!(result, Err(PromotionError::MaxVotersExceeded)));
    }

    #[tokio::test]
    async fn test_verify_quorum_with_replacement() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // 5-node cluster (quorum = 3)
        let voters: Vec<NodeId> = vec![1.into(), 2.into(), 3.into(), 4.into(), 5.into()];

        // Replacing one node should still maintain quorum
        let result = coordinator.verify_quorum(&voters, Some(5.into())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_quorum_fails_without_enough_nodes() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // 3-node cluster (quorum = 2)
        let voters: Vec<NodeId> = vec![1.into(), 2.into(), 3.into()];

        // Replacing one node leaves 2 nodes, which is exactly quorum
        // This should pass since we still have quorum
        let result = coordinator.verify_quorum(&voters, Some(3.into())).await;
        assert!(result.is_ok());

        // 2-node cluster (quorum = 2)
        let voters: Vec<NodeId> = vec![1.into(), 2.into()];

        // Replacing one node leaves 1 node, which is less than quorum (2)
        // This should fail
        let result = coordinator.verify_quorum(&voters, Some(2.into())).await;
        assert!(matches!(result, Err(PromotionError::NoQuorumWithoutFailedNode { .. })));
    }

    #[tokio::test]
    async fn test_learner_health_check_with_detector() {
        use crate::node_failure_detection::ConnectionStatus;
        use crate::node_failure_detection::NodeFailureDetector;

        let controller = Arc::new(DeterministicClusterController::new());
        let detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));
        let coordinator = LearnerPromotionCoordinator::with_failure_detector(controller, detector.clone());

        let learner_id = 42;

        // Learner is healthy - should pass
        {
            let mut detector_guard = detector.write().await;
            detector_guard.update_node_status(learner_id, ConnectionStatus::Connected, ConnectionStatus::Connected);
        }

        let result = coordinator.verify_learner_healthy(learner_id).await;
        assert!(result.is_ok());

        // Learner fails - should fail
        {
            let mut detector_guard = detector.write().await;
            detector_guard.update_node_status(
                learner_id,
                ConnectionStatus::Disconnected,
                ConnectionStatus::Disconnected,
            );
        }

        let result = coordinator.verify_learner_healthy(learner_id).await;
        assert!(matches!(result, Err(PromotionError::LearnerUnhealthy { .. })));
    }
}
