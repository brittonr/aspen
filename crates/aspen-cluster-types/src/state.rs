use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;

use crate::ClusterNode;
use crate::ControlPlaneError;

/// Reflects the state of the cluster from the perspective of the control plane.
///
/// Provides a snapshot of cluster topology including all known nodes,
/// current voting members, and learner nodes.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterState {
    /// All nodes known to the cluster (both voters and learners).
    pub nodes: Vec<ClusterNode>,
    /// Node IDs of current voting members participating in Raft consensus.
    pub members: Vec<u64>,
    /// Non-voting learner nodes that replicate data but don't vote.
    pub learners: Vec<ClusterNode>,
}

/// Request to initialize a new Raft cluster.
///
/// Used with [`ClusterController::init()`] to bootstrap a cluster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitRequest {
    /// The founding voting members of the cluster.
    pub initial_members: Vec<ClusterNode>,
}

impl InitRequest {
    /// Validate the initialization request.
    ///
    /// Returns an error if:
    /// - `initial_members` is empty
    /// - Any node ID is zero (reserved in Raft)
    /// - Any two nodes have the same ID
    pub fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.initial_members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "initial_members must not be empty".into(),
            });
        }

        // Check for node ID zero (reserved)
        for node in &self.initial_members {
            if node.id == 0 {
                return Err(ControlPlaneError::InvalidRequest {
                    reason: "node ID 0 is reserved and cannot be used".into(),
                });
            }
        }

        // Check for duplicate node IDs
        let mut seen_ids = HashSet::new();
        for node in &self.initial_members {
            if !seen_ids.insert(node.id) {
                return Err(ControlPlaneError::InvalidRequest {
                    reason: format!("duplicate node ID {} in initial_members", node.id),
                });
            }
        }

        Ok(())
    }
}

/// Request to add a non-voting learner to the cluster.
///
/// Used with [`ClusterController::add_learner()`] to add nodes that
/// replicate data without participating in consensus voting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddLearnerRequest {
    /// The learner node to add to the cluster.
    pub learner: ClusterNode,
}

/// Request to change the voting membership of the cluster.
///
/// Used with [`ClusterController::change_membership()`] to reconfigure
/// which nodes participate in Raft consensus voting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeMembershipRequest {
    /// The complete set of node IDs that should be voting members.
    pub members: Vec<u64>,
}
