//! NodeRpcClient trait for sending upgrade/rollback/health RPCs to nodes.
//!
//! The coordinator uses this trait so it's testable with mock implementations.
//! The real implementation sends RPCs via iroh QUIC (CLIENT_ALPN) in the
//! handler layer.

use std::fmt;

/// Trait for sending RPCs to individual cluster nodes.
///
/// The coordinator calls these methods during deployment. The real implementation
/// sends `NodeUpgrade`, `NodeRollback`, and `GetHealth` RPCs via iroh QUIC.
/// Tests provide a mock that records calls and returns configurable results.
#[async_trait::async_trait]
pub trait NodeRpcClient: Send + Sync {
    /// Send a NodeUpgrade RPC to the target node.
    ///
    /// The `deploy_id` identifies the deployment this upgrade belongs to.
    /// The `artifact_ref` is either a Nix store path or blob hash string.
    /// Returns Ok(()) if the node accepted the upgrade.
    async fn send_upgrade(
        &self,
        node_id: u64,
        deploy_id: &str,
        artifact_ref: &str,
        expected_binary: Option<&str>,
    ) -> std::result::Result<(), RpcError>;

    /// Send a NodeRollback RPC to the target node.
    ///
    /// The `deploy_id` identifies the deployment being rolled back.
    /// Returns Ok(()) if the node accepted the rollback.
    async fn send_rollback(&self, node_id: u64, deploy_id: &str) -> std::result::Result<(), RpcError>;

    /// Check if a node is healthy (passed all health checks).
    ///
    /// Should verify:
    /// 1. Node responds to health check
    /// 2. Node is in Raft membership
    /// 3. Node's Raft log gap is acceptable
    ///
    /// Returns Ok(true) if healthy, Ok(false) if not yet healthy,
    /// Err if the node is unreachable.
    async fn check_health(&self, node_id: u64) -> std::result::Result<bool, RpcError>;
}

/// Error from node RPC calls.
#[derive(Debug)]
pub struct RpcError {
    pub message: String,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RpcError {}

impl RpcError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
