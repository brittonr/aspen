//! Deployment RPC message types.
//!
//! Types for the rolling deployment protocol. These are used by both
//! the `ClusterDeploy*` / `NodeUpgrade*` / `NodeRollback*` RPC variants
//! and the `aspen-deploy` crate's internal state machine.

use serde::Deserialize;
use serde::Serialize;

/// Result of initiating a cluster-wide deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDeployResultResponse {
    /// Whether the deployment was accepted.
    pub is_accepted: bool,
    /// Unique deployment ID assigned by the coordinator.
    pub deploy_id: Option<String>,
    /// Error message if not accepted.
    pub error: Option<String>,
}

/// Current deployment status with per-node breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDeployStatusResultResponse {
    /// Whether a deployment was found.
    pub is_found: bool,
    /// Unique deployment ID.
    pub deploy_id: Option<String>,
    /// Overall status: "pending", "deploying", "completed", "failed", "rolling_back",
    /// "rolled_back".
    pub status: Option<String>,
    /// Target artifact reference (store path or blob hash).
    pub artifact: Option<String>,
    /// Per-node status entries.
    pub nodes: Vec<NodeDeployStatusEntry>,
    /// Deployment start time in milliseconds since epoch.
    pub started_at_ms: Option<u64>,
    /// Elapsed time in milliseconds since deployment started.
    pub elapsed_ms: Option<u64>,
    /// Error message if applicable.
    pub error: Option<String>,
}

/// Per-node deployment status for status reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDeployStatusEntry {
    /// Node ID.
    pub node_id: u64,
    /// Node status: "pending", "draining", "upgrading", "restarting", "healthy", "failed".
    pub status: String,
    /// Error message if the node failed.
    pub error: Option<String>,
}

/// Result of a cluster rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterRollbackResultResponse {
    /// Whether the rollback was initiated.
    pub is_accepted: bool,
    /// Deployment ID being rolled back.
    pub deploy_id: Option<String>,
    /// Error message if not accepted.
    pub error: Option<String>,
}

/// Result of a single-node upgrade operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeUpgradeResultResponse {
    /// Whether the upgrade was accepted.
    pub is_accepted: bool,
    /// Error message if the upgrade failed.
    pub error: Option<String>,
}

/// Result of a single-node rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRollbackResultResponse {
    /// Whether the rollback succeeded.
    pub is_success: bool,
    /// Error message if the rollback failed.
    pub error: Option<String>,
}
