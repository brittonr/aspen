//! Simulation metrics collected during test execution.

use serde::Deserialize;
use serde::Serialize;

use super::liveness::LivenessMetrics;

/// Structured metrics captured during simulation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimulationMetrics {
    /// Number of Raft RPC calls made.
    pub rpc_count: u64,
    /// Maximum log size across all nodes.
    pub max_log_size: u64,
    /// Number of nodes in the cluster.
    pub node_count: u32,
    /// Total simulation duration in milliseconds.
    pub duration_ms: u64,
    /// Number of leader elections observed.
    pub elections: u32,
    /// Number of node crashes.
    pub node_crashes: u32,
    /// Number of node restarts.
    pub node_restarts: u32,
    /// Number of network partitions created.
    pub network_partitions: u32,
    /// Number of Byzantine message corruptions.
    pub byzantine_corruptions: u64,
    /// Number of membership changes (add learner, change membership).
    pub membership_changes: u32,
    /// Number of BUGGIFY faults triggered.
    pub buggify_triggers: u64,
    /// Liveness metrics (populated when liveness mode is used).
    #[serde(default)]
    pub liveness: LivenessMetrics,
}
