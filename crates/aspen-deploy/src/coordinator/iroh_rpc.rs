//! IrohNodeRpcClient — real inter-node RPC over iroh QUIC.
//!
//! The coordinator (running on the Raft leader) uses this to send
//! NodeUpgrade, NodeRollback, and health check RPCs to follower nodes
//! via the CLIENT_ALPN protocol handler.
//!
//! Gated behind the `iroh` feature to avoid pulling iroh deps for
//! users who only need the coordinator with mock clients.

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::info;
use tracing::warn;

use super::rpc::NodeRpcClient;
use super::rpc::RpcError;

/// RPC timeout for inter-node deploy operations.
const DEPLOY_RPC_TIMEOUT_SECS: u64 = 30;

/// Maximum response size for inter-node RPCs (same as MAX_CLIENT_MESSAGE_SIZE).
const MAX_DEPLOY_RPC_RESPONSE: usize = 16 * 1024 * 1024;

/// NodeRpcClient implementation that sends real RPCs via iroh QUIC.
///
/// Node address resolution uses `ClusterController::current_state()` which
/// returns `ClusterState` with `ClusterNode` entries containing the iroh
/// `EndpointAddr` for each node.
pub struct IrohNodeRpcClient {
    endpoint: iroh::Endpoint,
    controller: Arc<dyn aspen_core::ClusterController>,
    source_node_id: u64,
}

impl IrohNodeRpcClient {
    /// Create a new iroh-based RPC client.
    pub fn new(
        endpoint: iroh::Endpoint,
        controller: Arc<dyn aspen_core::ClusterController>,
        source_node_id: u64,
    ) -> Self {
        Self {
            endpoint,
            controller,
            source_node_id,
        }
    }

    /// Get a reference to the controller (used for health check log gap verification).
    pub fn controller(&self) -> &Arc<dyn aspen_core::ClusterController> {
        &self.controller
    }

    /// Resolve a node_id to its iroh EndpointAddr via cluster state.
    async fn resolve_node_addr(&self, node_id: u64) -> std::result::Result<iroh::EndpointAddr, RpcError> {
        let state = self
            .controller
            .current_state()
            .await
            .map_err(|e| RpcError::new(format!("failed to get cluster state for node {node_id}: {e}")))?;

        for node in &state.nodes {
            if node.id == node_id {
                if let Some(addr) = node.iroh_addr() {
                    return Ok(addr.clone());
                }
                return Err(RpcError::new(format!(
                    "node {node_id} found in cluster state but has no iroh endpoint address"
                )));
            }
        }

        Err(RpcError::new(format!("node {node_id} not found in cluster state")))
    }

    /// Send a ClientRpcRequest to a specific node and return the response.
    async fn send_rpc(
        &self,
        node_id: u64,
        request: ClientRpcRequest,
    ) -> std::result::Result<ClientRpcResponse, RpcError> {
        let target_addr = self.resolve_node_addr(node_id).await?;
        let timeout_duration = Duration::from_secs(DEPLOY_RPC_TIMEOUT_SECS);

        // Connect via CLIENT_ALPN
        let connection = tokio::time::timeout(timeout_duration, async {
            self.endpoint
                .connect(target_addr.clone(), aspen_client_api::CLIENT_ALPN)
                .await
                .map_err(|e| RpcError::new(format!("connection to node {node_id} failed: {e}")))
        })
        .await
        .map_err(|_| RpcError::new(format!("connection to node {node_id} timed out")))??;

        // Open bidirectional stream
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| RpcError::new(format!("failed to open stream to node {node_id}: {e}")))?;

        // Wrap as unauthenticated request (inter-node, same cluster)
        let authenticated_request = aspen_client_api::AuthenticatedRequest::unauthenticated(request);

        // Serialize and send
        let request_bytes = postcard::to_stdvec(&authenticated_request)
            .map_err(|e| RpcError::new(format!("failed to serialize request for node {node_id}: {e}")))?;

        send.write_all(&request_bytes)
            .await
            .map_err(|e| RpcError::new(format!("failed to send request to node {node_id}: {e}")))?;

        send.finish()
            .map_err(|e| RpcError::new(format!("failed to finish send stream to node {node_id}: {e}")))?;

        // Read response with timeout
        let response_bytes = tokio::time::timeout(timeout_duration, async {
            recv.read_to_end(MAX_DEPLOY_RPC_RESPONSE)
                .await
                .map_err(|e| RpcError::new(format!("failed to read response from node {node_id}: {e}")))
        })
        .await
        .map_err(|_| RpcError::new(format!("response from node {node_id} timed out")))??;

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes)
            .map_err(|e| RpcError::new(format!("failed to deserialize response from node {node_id}: {e}")))?;

        // Close connection gracefully
        connection.close(iroh::endpoint::VarInt::from_u32(0), b"done");

        Ok(response)
    }
}

#[async_trait::async_trait]
impl NodeRpcClient for IrohNodeRpcClient {
    async fn send_upgrade(
        &self,
        node_id: u64,
        deploy_id: &str,
        artifact_ref: &str,
    ) -> std::result::Result<(), RpcError> {
        info!(
            source_node = self.source_node_id,
            target_node = node_id,
            deploy_id = deploy_id,
            artifact = artifact_ref,
            "sending NodeUpgrade RPC via iroh"
        );

        let request = ClientRpcRequest::NodeUpgrade {
            deploy_id: deploy_id.to_string(),
            artifact: artifact_ref.to_string(),
        };

        let response = self.send_rpc(node_id, request).await?;

        match response {
            ClientRpcResponse::NodeUpgradeResult(result) => {
                if result.is_accepted {
                    info!(target_node = node_id, "NodeUpgrade accepted");
                    Ok(())
                } else {
                    let msg = result.error.unwrap_or_else(|| "upgrade rejected".to_string());
                    Err(RpcError::new(format!("node {node_id} rejected upgrade: {msg}")))
                }
            }
            ClientRpcResponse::Error(e) => {
                Err(RpcError::new(format!("node {node_id} returned error: {} - {}", e.code, e.message)))
            }
            other => Err(RpcError::new(format!("node {node_id} returned unexpected response: {other:?}"))),
        }
    }

    async fn send_rollback(&self, node_id: u64, deploy_id: &str) -> std::result::Result<(), RpcError> {
        info!(
            source_node = self.source_node_id,
            target_node = node_id,
            deploy_id = deploy_id,
            "sending NodeRollback RPC via iroh"
        );

        let request = ClientRpcRequest::NodeRollback {
            deploy_id: deploy_id.to_string(),
        };

        let response = self.send_rpc(node_id, request).await?;

        match response {
            ClientRpcResponse::NodeRollbackResult(result) => {
                if result.is_success {
                    info!(target_node = node_id, "NodeRollback accepted");
                    Ok(())
                } else {
                    let msg = result.error.unwrap_or_else(|| "rollback rejected".to_string());
                    Err(RpcError::new(format!("node {node_id} rejected rollback: {msg}")))
                }
            }
            ClientRpcResponse::Error(e) => {
                Err(RpcError::new(format!("node {node_id} returned error: {} - {}", e.code, e.message)))
            }
            other => Err(RpcError::new(format!("node {node_id} returned unexpected response: {other:?}"))),
        }
    }

    async fn check_health(&self, node_id: u64) -> std::result::Result<bool, RpcError> {
        let request = ClientRpcRequest::GetHealth;

        let response = self.send_rpc(node_id, request).await?;

        match response {
            ClientRpcResponse::Health(health) => {
                let is_healthy = health.status == "healthy";
                if !is_healthy {
                    info!(
                        target_node = node_id,
                        status = %health.status,
                        "node reports unhealthy"
                    );
                }

                // Verify Raft replication log gap if node reports healthy
                if is_healthy {
                    match self.controller.get_metrics().await {
                        Ok(metrics) => {
                            if let Some(ref replication) = metrics.replication {
                                if let Some(matched) = replication.get(&node_id) {
                                    if let (Some(matched_idx), Some(last_idx)) = (matched, metrics.last_log_index) {
                                        let gap = last_idx.saturating_sub(*matched_idx);
                                        if gap > aspen_constants::api::DEPLOY_LOG_GAP_THRESHOLD {
                                            info!(
                                                target_node = node_id,
                                                matched_log_index = matched_idx,
                                                last_log_index = last_idx,
                                                gap = gap,
                                                threshold = aspen_constants::api::DEPLOY_LOG_GAP_THRESHOLD,
                                                "node healthy but Raft log gap exceeds threshold"
                                            );
                                            return Ok(false);
                                        }
                                    }
                                } else {
                                    // Node not in replication map — not yet healthy
                                    info!(target_node = node_id, "node not found in Raft replication map");
                                    return Ok(false);
                                }
                            }
                            // No replication data (single-node or metrics not available) — trust
                            // GetHealth
                        }
                        Err(e) => {
                            warn!(
                                target_node = node_id,
                                error = %e,
                                "failed to get Raft metrics for log gap check, trusting GetHealth"
                            );
                        }
                    }
                }

                Ok(is_healthy)
            }
            ClientRpcResponse::Error(e) => {
                warn!(target_node = node_id, code = %e.code, "health check returned error");
                Ok(false)
            }
            _ => {
                warn!(target_node = node_id, "health check returned unexpected response type");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// Helper to verify log gap logic: returns true if gap is acceptable.
    fn is_log_gap_acceptable(
        replication: &Option<BTreeMap<u64, Option<u64>>>,
        target_node_id: u64,
        last_log_index: Option<u64>,
    ) -> bool {
        if let Some(repl) = replication {
            if let Some(matched) = repl.get(&target_node_id) {
                if let (Some(matched_idx), Some(last_idx)) = (matched, last_log_index) {
                    let gap = last_idx.saturating_sub(*matched_idx);
                    return gap <= aspen_constants::api::DEPLOY_LOG_GAP_THRESHOLD;
                }
            } else {
                // Not in replication map → not healthy
                return false;
            }
        }
        // No replication data → trust GetHealth
        true
    }

    /// Test: node healthy with acceptable log gap.
    #[test]
    fn test_log_gap_acceptable() {
        let mut repl = BTreeMap::new();
        repl.insert(2u64, Some(990u64));
        assert!(is_log_gap_acceptable(&Some(repl), 2, Some(1000)));
    }

    /// Test: node with large log gap exceeds threshold.
    #[test]
    fn test_log_gap_exceeds_threshold() {
        let mut repl = BTreeMap::new();
        repl.insert(2u64, Some(800u64));
        assert!(!is_log_gap_acceptable(&Some(repl), 2, Some(1000)));
    }

    /// Test: node not in replication map is treated as not healthy.
    #[test]
    fn test_node_not_in_replication_map() {
        let repl = BTreeMap::new(); // empty
        assert!(!is_log_gap_acceptable(&Some(repl), 2, Some(1000)));
    }

    /// Test: no replication data trusts GetHealth.
    #[test]
    fn test_no_replication_data_trusts_get_health() {
        assert!(is_log_gap_acceptable(&None, 2, Some(1000)));
    }

    /// Test: gap exactly at threshold is acceptable.
    #[test]
    fn test_log_gap_at_threshold() {
        let mut repl = BTreeMap::new();
        repl.insert(2u64, Some(900u64));
        // gap = 1000 - 900 = 100 = threshold → acceptable (<=)
        assert!(is_log_gap_acceptable(&Some(repl), 2, Some(1000)));
    }

    /// Test: gap one over threshold is not acceptable.
    #[test]
    fn test_log_gap_one_over_threshold() {
        let mut repl = BTreeMap::new();
        repl.insert(2u64, Some(899u64));
        // gap = 1000 - 899 = 101 > 100 threshold
        assert!(!is_log_gap_acceptable(&Some(repl), 2, Some(1000)));
    }
}
