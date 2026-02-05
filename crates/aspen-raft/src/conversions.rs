//! OpenRaft type conversions for aspen-core types.
//!
//! This module provides conversion functions between aspen-core types
//! and OpenRaft types. These conversions cannot be in aspen-core directly
//! because it would create a circular dependency with openraft.

use aspen_core::ClusterMetrics;
use aspen_core::NodeState;
use aspen_core::SnapshotLogId;

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
