//! Cluster management types.
//!
//! Types for managing cluster membership and topology.

use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;

use crate::error::ControlPlaneError;
use crate::types::NodeAddress;

/// Describes a node participating in the control-plane cluster.
///
/// Contains both the node's identifier and its P2P endpoint address,
/// which is stored in Raft membership state for persistent discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    /// Unique identifier for this node within the cluster.
    pub id: u64,
    /// Display address for logging and human-readable output.
    pub addr: String,
    /// Optional legacy Raft address (host:port) for backwards compatibility.
    pub raft_addr: Option<String>,
    /// P2P endpoint address for connecting to this node.
    pub node_addr: Option<NodeAddress>,
}

impl ClusterNode {
    /// Create a new ClusterNode with a simple string address (legacy).
    pub fn new(id: u64, addr: impl Into<String>, raft_addr: Option<String>) -> Self {
        Self {
            id,
            addr: addr.into(),
            raft_addr,
            node_addr: None,
        }
    }

    /// Create a new ClusterNode with a P2P endpoint address.
    pub fn with_node_addr(id: u64, node_addr: NodeAddress) -> Self {
        Self {
            id,
            addr: node_addr.id(),
            raft_addr: None,
            node_addr: Some(node_addr),
        }
    }

    /// Create a ClusterNode with an iroh EndpointAddr.
    pub fn with_iroh_addr(id: u64, iroh_addr: iroh::EndpointAddr) -> Self {
        Self::with_node_addr(id, NodeAddress::new(iroh_addr))
    }

    /// Get the node address as an iroh EndpointAddr, if available.
    pub fn iroh_addr(&self) -> Option<&iroh::EndpointAddr> {
        self.node_addr.as_ref().map(|addr| addr.inner())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // ClusterNode tests
    // ============================================================================

    #[test]
    fn cluster_node_new_creates_with_legacy_address() {
        let node = ClusterNode::new(1, "127.0.0.1:5000", Some("127.0.0.1:5001".to_string()));
        assert_eq!(node.id, 1);
        assert_eq!(node.addr, "127.0.0.1:5000");
        assert_eq!(node.raft_addr, Some("127.0.0.1:5001".to_string()));
        assert!(node.node_addr.is_none());
    }

    #[test]
    fn cluster_node_new_without_raft_addr() {
        let node = ClusterNode::new(42, "localhost", None);
        assert_eq!(node.id, 42);
        assert_eq!(node.addr, "localhost");
        assert!(node.raft_addr.is_none());
        assert!(node.node_addr.is_none());
    }

    #[test]
    fn cluster_node_iroh_addr_returns_none_when_not_set() {
        let node = ClusterNode::new(1, "test", None);
        assert!(node.iroh_addr().is_none());
    }

    #[test]
    fn cluster_node_equality() {
        let node1 = ClusterNode::new(1, "addr1", None);
        let node2 = ClusterNode::new(1, "addr1", None);
        let node3 = ClusterNode::new(2, "addr1", None);
        let node4 = ClusterNode::new(1, "addr2", None);

        assert_eq!(node1, node2);
        assert_ne!(node1, node3); // Different ID
        assert_ne!(node1, node4); // Different addr
    }

    #[test]
    fn cluster_node_clone() {
        let node = ClusterNode::new(5, "cloned", Some("raft".to_string()));
        let cloned = node.clone();
        assert_eq!(node, cloned);
    }

    #[test]
    fn cluster_node_debug_format() {
        let node = ClusterNode::new(1, "test", None);
        let debug = format!("{:?}", node);
        assert!(debug.contains("id: 1"));
        assert!(debug.contains("addr: \"test\""));
    }

    #[test]
    fn cluster_node_serialization_roundtrip() {
        let node = ClusterNode::new(99, "serialize-me", Some("raft-addr".to_string()));
        let json = serde_json::to_string(&node).expect("serialize");
        let deserialized: ClusterNode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(node, deserialized);
    }

    // ============================================================================
    // ClusterState tests
    // ============================================================================

    #[test]
    fn cluster_state_default_is_empty() {
        let state = ClusterState::default();
        assert!(state.nodes.is_empty());
        assert!(state.members.is_empty());
        assert!(state.learners.is_empty());
    }

    #[test]
    fn cluster_state_with_nodes() {
        let state = ClusterState {
            nodes: vec![ClusterNode::new(1, "node1", None), ClusterNode::new(2, "node2", None)],
            members: vec![1, 2],
            learners: vec![],
        };
        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.members, vec![1, 2]);
    }

    #[test]
    fn cluster_state_clone() {
        let state = ClusterState {
            nodes: vec![ClusterNode::new(1, "n1", None)],
            members: vec![1],
            learners: vec![ClusterNode::new(2, "learner", None)],
        };
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn cluster_state_serialization_roundtrip() {
        let state = ClusterState {
            nodes: vec![ClusterNode::new(1, "node", None)],
            members: vec![1],
            learners: vec![],
        };
        let json = serde_json::to_string(&state).expect("serialize");
        let deserialized: ClusterState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, deserialized);
    }

    // ============================================================================
    // InitRequest validation tests
    // ============================================================================

    #[test]
    fn init_request_validate_empty_members_fails() {
        let request = InitRequest {
            initial_members: vec![],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("must not be empty"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_node_id_zero_fails() {
        let request = InitRequest {
            initial_members: vec![ClusterNode::new(0, "zero-node", None)],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("node ID 0 is reserved"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_node_id_zero_in_middle_fails() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(0, "zero-node", None),
                ClusterNode::new(2, "node2", None),
            ],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("node ID 0 is reserved"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_duplicate_node_ids_fails() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(2, "node2", None),
                ClusterNode::new(1, "node1-dup", None), // Duplicate ID
            ],
        };
        let result = request.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("duplicate node ID 1"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn init_request_validate_single_valid_node_succeeds() {
        let request = InitRequest {
            initial_members: vec![ClusterNode::new(1, "single-node", None)],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn init_request_validate_multiple_valid_nodes_succeeds() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(2, "node2", None),
                ClusterNode::new(3, "node3", None),
            ],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn init_request_validate_large_node_ids_succeeds() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(u64::MAX, "max-id", None),
                ClusterNode::new(u64::MAX - 1, "almost-max", None),
            ],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn init_request_serialization_roundtrip() {
        let request = InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", Some("raft1".to_string())),
                ClusterNode::new(2, "node2", None),
            ],
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: InitRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }

    // ============================================================================
    // AddLearnerRequest tests
    // ============================================================================

    #[test]
    fn add_learner_request_creation() {
        let request = AddLearnerRequest {
            learner: ClusterNode::new(5, "learner-node", None),
        };
        assert_eq!(request.learner.id, 5);
        assert_eq!(request.learner.addr, "learner-node");
    }

    #[test]
    fn add_learner_request_serialization_roundtrip() {
        let request = AddLearnerRequest {
            learner: ClusterNode::new(10, "learner", Some("raft".to_string())),
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: AddLearnerRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }

    // ============================================================================
    // ChangeMembershipRequest tests
    // ============================================================================

    #[test]
    fn change_membership_request_creation() {
        let request = ChangeMembershipRequest { members: vec![1, 2, 3] };
        assert_eq!(request.members, vec![1, 2, 3]);
    }

    #[test]
    fn change_membership_request_empty_members() {
        let request = ChangeMembershipRequest { members: vec![] };
        assert!(request.members.is_empty());
    }

    #[test]
    fn change_membership_request_serialization_roundtrip() {
        let request = ChangeMembershipRequest {
            members: vec![1, 2, 3, 4, 5],
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: ChangeMembershipRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }
}
