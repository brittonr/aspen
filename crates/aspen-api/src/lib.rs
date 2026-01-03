//! Core API traits and types for Aspen cluster operations.
//!
//! This module re-exports types from `aspen-core` and provides openraft-specific
//! conversions that cannot be in the core crate (to avoid openraft dependency).
//!
//! # Key Traits
//!
//! - `ClusterController`: Cluster membership management (init, add learner, change membership)
//! - `KeyValueStore`: Distributed key-value operations (read, write, delete, scan)
//!
//! # Tiger Style
//!
//! - Fixed limits on scan results (MAX_SCAN_RESULTS = 10,000)
//! - Explicit error types with actionable context
//! - Size validation on keys and values (prevents memory exhaustion)
//! - Pagination support for bounded memory usage

// Re-export everything from aspen-core
pub use aspen_core::*;

// ============================================================================
// Local type definitions to avoid circular dependencies
// ============================================================================

// Import the actual types from aspen-raft-types
// Note: NodeId is already re-exported from aspen_core via pub use aspen_core::*
use aspen_raft_types::{AppRequest, AppResponse, RaftMemberInfo};

// Define the minimum types needed for OpenRaft conversions
openraft::declare_raft_types!(
    /// Local AppTypeConfig for OpenRaft conversions in aspen-api
    pub LocalAppTypeConfig:
    D = AppRequest,
    R = AppResponse,
    NodeId = NodeId,
    Node = RaftMemberInfo,
);

// ============================================================================
// OpenRaft Conversions (cannot be in aspen-core due to openraft dependency)
// ============================================================================

/// Convert openraft::ServerState to NodeState.
pub fn node_state_from_openraft(state: openraft::ServerState) -> NodeState {
    match state {
        openraft::ServerState::Learner => NodeState::Learner,
        openraft::ServerState::Follower => NodeState::Follower,
        openraft::ServerState::Candidate => NodeState::Candidate,
        openraft::ServerState::Leader => NodeState::Leader,
        openraft::ServerState::Shutdown => NodeState::Shutdown,
    }
}

/// Convert NodeState to openraft::ServerState.
pub fn node_state_to_openraft(state: NodeState) -> openraft::ServerState {
    match state {
        NodeState::Learner => openraft::ServerState::Learner,
        NodeState::Follower => openraft::ServerState::Follower,
        NodeState::Candidate => openraft::ServerState::Candidate,
        NodeState::Leader => openraft::ServerState::Leader,
        NodeState::Shutdown => openraft::ServerState::Shutdown,
    }
}

/// Create ClusterMetrics from openraft RaftMetrics.
///
/// Note: This function uses a generic type parameter to avoid circular dependencies.
pub fn cluster_metrics_from_openraft<C: openraft::RaftTypeConfig>(
    metrics: &openraft::metrics::RaftMetrics<C>,
) -> ClusterMetrics
where
    C::NodeId: Into<u64> + Copy,
    C::Term: Into<u64> + Copy,
{
    let membership = metrics.membership_config.membership();
    ClusterMetrics {
        id: metrics.id.into(),
        state: node_state_from_openraft(metrics.state),
        current_leader: metrics.current_leader.map(|id| id.into()),
        current_term: metrics.current_term.into(),
        last_log_index: metrics.last_log_index,
        last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
        snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
        replication: metrics.replication.as_ref().map(|repl_map| {
            repl_map
                .iter()
                .map(|(node_id, matched)| ((*node_id).into(), matched.as_ref().map(|log_id| log_id.index)))
                .collect()
        }),
        voters: membership.voter_ids().map(|id| id.into()).collect(),
        learners: membership.learner_ids().map(|id| id.into()).collect(),
    }
}

/// Create SnapshotLogId from openraft LogId.
pub fn snapshot_log_id_from_openraft<C: openraft::RaftTypeConfig>(log_id: &openraft::LogId<C>) -> SnapshotLogId
where
    C::NodeId: Into<u64> + Copy,
    C::Term: Into<u64> + Copy,
    <C::LeaderId as openraft::vote::RaftLeaderId<C>>::Committed: Copy + std::ops::Deref<Target = C::Term>,
{
    SnapshotLogId {
        term: (*log_id.leader_id).into(),
        index: log_id.index,
    }
}
