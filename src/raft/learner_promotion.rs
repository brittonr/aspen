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
//! Success: `{"success": true, "learner_id": 4, "previous_voters": [1,2,3], "new_voters": [1,2,3,4]}`
//!
//! Error: `HTTP 400 Bad Request` with error message
//!
//! Tiger Style: Fixed timeouts, bounded membership (max 100 voters), explicit types.

use std::sync::Arc;
use std::time::{Duration, Instant};

use snafu::Snafu;
use tokio::sync::RwLock;

use crate::api::{ChangeMembershipRequest, ClusterController};
use crate::raft::node_failure_detection::{FailureType, NodeFailureDetector};
use crate::raft::types::NodeId;

/// Maximum number of voters allowed in the cluster.
///
/// Tiger Style: Bounded resource to prevent unbounded membership growth.
const MAX_VOTERS: u32 = 100;

/// Lag threshold for learner promotion (entries behind leader).
///
/// Tiger Style: Fixed threshold for determining if learner is caught up.
const LEARNER_LAG_THRESHOLD: u64 = 100;

/// Cooldown period between membership changes.
///
/// Tiger Style: Fixed 5-minute cooldown to prevent rapid membership churn.
const MEMBERSHIP_COOLDOWN: Duration = Duration::from_secs(300);

/// Errors that can occur during learner promotion.
#[derive(Debug, Snafu)]
pub enum PromotionError {
    #[snafu(display("Learner {} is not healthy or reachable", learner_id))]
    LearnerUnhealthy { learner_id: NodeId },

    #[snafu(display("Learner {} is too far behind (lag: {} entries)", learner_id, lag))]
    LearnerLagging { learner_id: NodeId, lag: u64 },

    #[snafu(display("Cluster lacks quorum without failed node {:?}", failed_node_id))]
    NoQuorumWithoutFailedNode { failed_node_id: Option<NodeId> },

    #[snafu(display("Membership was changed recently (wait {:?})", remaining))]
    MembershipChangeTooRecent { remaining: Duration },

    #[snafu(display("Maximum voters ({}) would be exceeded", MAX_VOTERS))]
    MaxVotersExceeded,

    #[snafu(display("Learner {} not found in cluster", learner_id))]
    LearnerNotFound { learner_id: NodeId },

    #[snafu(display("Node {} is not a learner (already a voter)", node_id))]
    NotALearner { node_id: NodeId },

    #[snafu(display("Failed to get cluster metrics: {}", source))]
    MetricsError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("Failed to change membership: {}", source))]
    MembershipChangeError {
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
    pub async fn promote_learner(
        &self,
        request: PromotionRequest,
    ) -> Result<PromotionResult, PromotionError> {
        // Safety check 1: Membership cooldown
        if !request.force {
            self.check_membership_cooldown().await?;
        }

        // Get current membership state
        let metrics = self
            .cluster_controller
            .get_metrics()
            .await
            .map_err(|e| PromotionError::MetricsError {
                source: Box::new(e),
            })?;

        let current_voters: Vec<NodeId> = metrics.membership_config.membership().voter_ids().collect();
        let current_learners: Vec<NodeId> = metrics.membership_config.membership().learner_ids().collect();

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
            self.verify_learner_caught_up(request.learner_id, &metrics)
                .await?;
        }

        // Safety check 4: Verify quorum will be maintained
        if !request.force {
            self.verify_quorum(&current_voters, request.replace_node)
                .await?;
        }

        // Build new membership set
        let new_voters = self.build_new_membership(
            &current_voters,
            request.learner_id,
            request.replace_node,
        )?;

        // Execute membership change
        let change_request = ChangeMembershipRequest {
            members: new_voters.clone(),
        };

        self.cluster_controller
            .change_membership(change_request)
            .await
            .map_err(|e| PromotionError::MembershipChangeError {
                source: Box::new(e),
            })?;

        // Update last change timestamp
        *self.last_membership_change.write().await = Some(Instant::now());

        tracing::info!(
            learner_id = request.learner_id,
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
    async fn verify_learner_healthy(&self, learner_id: NodeId) -> Result<(), PromotionError> {
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
    async fn verify_learner_caught_up(
        &self,
        learner_id: NodeId,
        metrics: &openraft::metrics::RaftMetrics<crate::raft::types::AppTypeConfig>,
    ) -> Result<(), PromotionError> {
        // Only the leader has replication metrics
        if let Some(ref replication) = metrics.replication {
            if let Some(matched_log_id) = replication.get(&learner_id) {
                if let (Some(leader_last_log), Some(matched)) =
                    (metrics.last_log_index, matched_log_id)
                {
                    let lag = leader_last_log.saturating_sub(matched.index);
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
    use crate::api::inmemory::DeterministicClusterController;

    #[tokio::test]
    async fn test_build_new_membership_promotes_learner() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        let current = vec![1, 2, 3];
        let new = coordinator
            .build_new_membership(&current, 4, None)
            .unwrap();

        assert_eq!(new, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_build_new_membership_replaces_node() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        let current = vec![1, 2, 3];
        let new = coordinator
            .build_new_membership(&current, 4, Some(2))
            .unwrap();

        assert_eq!(new, vec![1, 3, 4]);
    }

    #[tokio::test]
    async fn test_build_new_membership_no_duplicate() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Learner is already in voter set (edge case)
        let current = vec![1, 2, 3, 4];
        let new = coordinator
            .build_new_membership(&current, 4, None)
            .unwrap();

        // Should not add duplicate
        assert_eq!(new, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_membership_cooldown_enforced() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Simulate a recent membership change
        *coordinator.last_membership_change.write().await = Some(Instant::now());

        // Try to promote immediately (should fail)
        let result = coordinator.check_membership_cooldown().await;
        assert!(matches!(
            result,
            Err(PromotionError::MembershipChangeTooRecent { .. })
        ));
    }

    #[tokio::test]
    async fn test_membership_cooldown_elapsed() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Simulate a change that happened long ago
        *coordinator.last_membership_change.write().await =
            Some(Instant::now() - Duration::from_secs(400));

        // Should succeed (cooldown elapsed)
        let result = coordinator.check_membership_cooldown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_max_voters_enforced() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // Create membership at max capacity
        let current: Vec<NodeId> = (1..=MAX_VOTERS as u64).collect();

        // Try to add one more voter
        let result = coordinator.build_new_membership(&current, 999, None);

        assert!(matches!(result, Err(PromotionError::MaxVotersExceeded)));
    }

    #[tokio::test]
    async fn test_verify_quorum_with_replacement() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // 5-node cluster (quorum = 3)
        let voters = vec![1, 2, 3, 4, 5];

        // Replacing one node should still maintain quorum
        let result = coordinator.verify_quorum(&voters, Some(5)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_quorum_fails_without_enough_nodes() {
        let controller = Arc::new(DeterministicClusterController::new());
        let coordinator = LearnerPromotionCoordinator::new(controller);

        // 3-node cluster (quorum = 2)
        let voters = vec![1, 2, 3];

        // Replacing one node leaves 2 nodes, which is exactly quorum
        // This should pass since we still have quorum
        let result = coordinator.verify_quorum(&voters, Some(3)).await;
        assert!(result.is_ok());

        // 2-node cluster (quorum = 2)
        let voters = vec![1, 2];

        // Replacing one node leaves 1 node, which is less than quorum (2)
        // This should fail
        let result = coordinator.verify_quorum(&voters, Some(2)).await;
        assert!(matches!(
            result,
            Err(PromotionError::NoQuorumWithoutFailedNode { .. })
        ));
    }

    #[tokio::test]
    async fn test_learner_health_check_with_detector() {
        use crate::raft::node_failure_detection::{ConnectionStatus, NodeFailureDetector};

        let controller = Arc::new(DeterministicClusterController::new());
        let detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));
        let coordinator =
            LearnerPromotionCoordinator::with_failure_detector(controller, detector.clone());

        let learner_id = 42;

        // Learner is healthy - should pass
        {
            let mut detector_guard = detector.write().await;
            detector_guard.update_node_status(
                learner_id,
                ConnectionStatus::Connected,
                ConnectionStatus::Connected,
            );
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
