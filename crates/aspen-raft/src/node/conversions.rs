//! OpenRaft type conversion utilities.

use aspen_cluster_types::ClusterMetrics;
use aspen_cluster_types::NodeState;
use aspen_cluster_types::SnapshotLogId;
use openraft::metrics::RaftMetrics;

use crate::types::AppTypeConfig;

/// Convert openraft::ServerState to NodeState.
pub(crate) fn node_state_from_openraft(state: openraft::ServerState) -> NodeState {
    match state {
        openraft::ServerState::Learner => NodeState::Learner,
        openraft::ServerState::Follower => NodeState::Follower,
        openraft::ServerState::Candidate => NodeState::Candidate,
        openraft::ServerState::Leader => NodeState::Leader,
        openraft::ServerState::Shutdown => NodeState::Shutdown,
    }
}

/// Create ClusterMetrics from openraft RaftMetrics.
pub(crate) fn cluster_metrics_from_openraft(metrics: &RaftMetrics<AppTypeConfig>) -> ClusterMetrics {
    let membership = metrics.membership_config.membership();
    ClusterMetrics {
        id: metrics.id.0,
        state: node_state_from_openraft(metrics.state),
        current_leader: metrics.current_leader.map(|id| id.0),
        current_term: metrics.current_term,
        last_log_index: metrics.last_log_index,
        last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
        snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
        replication: metrics.replication.as_ref().map(|repl_map| {
            repl_map
                .iter()
                .map(|(node_id, matched)| (node_id.0, matched.as_ref().map(|log_id| log_id.index)))
                .collect()
        }),
        voters: membership.voter_ids().map(|id| id.0).collect(),
        learners: membership.learner_ids().map(|id| id.0).collect(),
    }
}

/// Create SnapshotLogId from openraft LogId.
pub(crate) fn snapshot_log_id_from_openraft(log_id: &openraft::LogId<AppTypeConfig>) -> SnapshotLogId {
    SnapshotLogId {
        term: log_id.leader_id.term,
        index: log_id.index,
    }
}
