//! Deployment coordinator error types.

use snafu::Snafu;

/// Errors that can occur during deployment coordination.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DeployError {
    /// A deployment is already in progress.
    #[snafu(display("deployment already in progress: {deploy_id}"))]
    DeploymentInProgress { deploy_id: String },

    /// No active or recent deployment found.
    #[snafu(display("no deployment found"))]
    NoDeploymentFound,

    /// The deployment is not in a rollback-eligible state.
    #[snafu(display("deployment {deploy_id} has status {status}, cannot rollback"))]
    NotRollbackEligible { deploy_id: String, status: String },

    /// The deployment cannot be resumed (terminal state).
    #[snafu(display("deployment {deploy_id} has terminal status {status}"))]
    DeploymentTerminal { deploy_id: String, status: String },

    /// KV store operation failed.
    #[snafu(display("kv error: {reason}"))]
    KvError { reason: String },

    /// CAS write conflict (concurrent modification).
    #[snafu(display("concurrent modification on key {key}"))]
    ConcurrentModification { key: String },

    /// Node upgrade RPC failed.
    #[snafu(display("node {node_id} upgrade failed: {reason}"))]
    NodeUpgradeFailed { node_id: u64, reason: String },

    /// Node rollback RPC failed.
    #[snafu(display("node {node_id} rollback failed: {reason}"))]
    NodeRollbackFailed { node_id: u64, reason: String },

    /// Health check timed out for a node.
    #[snafu(display("node {node_id} health check timed out after {timeout_secs}s"))]
    HealthCheckTimeout { node_id: u64, timeout_secs: u64 },

    /// Quorum safety violation — cannot upgrade more nodes.
    #[snafu(display("quorum safety: {reason}"))]
    QuorumSafetyViolation { reason: String },

    /// Serialization/deserialization failed.
    #[snafu(display("serde error: {reason}"))]
    SerdeError { reason: String },
}

pub type Result<T> = std::result::Result<T, DeployError>;
