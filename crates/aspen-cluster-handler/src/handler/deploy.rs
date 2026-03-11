//! Deploy operation handlers for cluster-wide rolling deployments.
//!
//! Handles: ClusterDeploy, ClusterDeployStatus, ClusterRollback, NodeUpgrade, NodeRollback.
//!
//! The cluster-level operations (ClusterDeploy/Status/Rollback) delegate to
//! `DeploymentCoordinator` which persists state in KV.
//!
//! The node-level operations (NodeUpgrade/NodeRollback) delegate to
//! `NodeUpgradeExecutor` which handles the local drain → binary swap → restart cycle.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::messages::deploy::*;
use aspen_deploy::DeployArtifact;
use aspen_deploy::DeployStrategy;
use aspen_deploy::DeploymentCoordinator;
use aspen_deploy::NodeDeployStatus;
use aspen_deploy::coordinator::rpc::NodeRpcClient;
use aspen_deploy::coordinator::rpc::RpcError;
use aspen_rpc_core::ClientProtocolContext;
use aspen_traits::KeyValueStore;
use tracing::error;
use tracing::info;
use tracing::warn;

/// RPC timeout for inter-node deploy operations.
const DEPLOY_RPC_TIMEOUT_SECS: u64 = 30;

/// Maximum response size for inter-node RPCs (same as MAX_CLIENT_MESSAGE_SIZE).
const MAX_DEPLOY_RPC_RESPONSE: usize = 16 * 1024 * 1024;

// ============================================================================
// Cluster-level handlers
// ============================================================================

/// Handle ClusterDeploy: start a new rolling deployment.
pub async fn handle_cluster_deploy(
    ctx: &ClientProtocolContext,
    artifact: String,
    strategy: String,
    max_concurrent: u32,
    health_timeout_secs: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let parsed_artifact = DeployArtifact::parse(&artifact);
    let deploy_strategy = match strategy.as_str() {
        "rolling" | "" => DeployStrategy::rolling(max_concurrent),
        other => {
            return Ok(ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
                is_accepted: false,
                deploy_id: None,
                error: Some(format!("unknown strategy: {other}")),
            }));
        }
    };

    // Get cluster members for node list
    let metrics = match ctx.controller.get_metrics().await {
        Ok(m) => m,
        Err(e) => {
            return Ok(ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
                is_accepted: false,
                deploy_id: None,
                error: Some(format!("failed to get cluster metrics: {e}")),
            }));
        }
    };
    let node_ids: Vec<u64> = metrics.voters.clone();

    if node_ids.is_empty() {
        return Ok(ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
            is_accepted: false,
            deploy_id: None,
            error: Some("no voters in cluster".to_string()),
        }));
    }

    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let deploy_id = format!("deploy-{now_ms}");

    let rpc_client =
        Arc::new(IrohNodeRpcClient::new(ctx.endpoint_manager.endpoint().clone(), ctx.controller.clone(), ctx.node_id));
    let coordinator = DeploymentCoordinator::with_timeouts(
        ctx.kv_store.clone(),
        rpc_client.clone(),
        ctx.node_id,
        health_timeout_secs,
        aspen_constants::api::DEPLOY_STATUS_POLL_INTERVAL_SECS,
    );

    match coordinator
        .start_deployment(deploy_id.clone(), parsed_artifact, deploy_strategy, &node_ids, now_ms)
        .await
    {
        Ok(record) => {
            info!(deploy_id = %record.deploy_id, "deployment accepted, spawning background task");

            // Spawn background task to run the deployment
            let deploy_id_clone = record.deploy_id.clone();
            let kv = ctx.kv_store.clone();
            let node_id = ctx.node_id;
            let ht = health_timeout_secs;
            let pi = aspen_constants::api::DEPLOY_STATUS_POLL_INTERVAL_SECS;
            tokio::spawn(async move {
                let coord = DeploymentCoordinator::with_timeouts(kv, rpc_client, node_id, ht, pi);
                match coord.run_deployment(&deploy_id_clone).await {
                    Ok(r) => info!(deploy_id = %r.deploy_id, status = ?r.status, "deployment finished"),
                    Err(e) => error!(deploy_id = %deploy_id_clone, error = %e, "deployment failed"),
                }
            });

            Ok(ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
                is_accepted: true,
                deploy_id: Some(record.deploy_id),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
            is_accepted: false,
            deploy_id: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle ClusterDeployStatus: get current deployment status.
pub async fn handle_cluster_deploy_status(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let rpc_client =
        Arc::new(IrohNodeRpcClient::new(ctx.endpoint_manager.endpoint().clone(), ctx.controller.clone(), ctx.node_id));
    let coordinator = DeploymentCoordinator::new(ctx.kv_store.clone(), rpc_client, ctx.node_id);

    match coordinator.get_status().await {
        Ok(record) => {
            let now_ms =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                    as u64;

            let nodes: Vec<NodeDeployStatusEntry> = record
                .nodes
                .iter()
                .map(|n| {
                    let (status_str, error) = match &n.status {
                        NodeDeployStatus::Failed(reason) => ("failed".to_string(), Some(reason.clone())),
                        other => (other.as_status_str().to_string(), None),
                    };
                    NodeDeployStatusEntry {
                        node_id: n.node_id,
                        status: status_str,
                        error,
                    }
                })
                .collect();

            Ok(ClientRpcResponse::ClusterDeployStatusResult(ClusterDeployStatusResultResponse {
                is_found: true,
                deploy_id: Some(record.deploy_id),
                status: Some(format!("{:?}", record.status).to_lowercase()),
                artifact: Some(record.artifact.display_ref().to_string()),
                nodes,
                started_at_ms: Some(record.created_at_ms),
                elapsed_ms: Some(now_ms.saturating_sub(record.created_at_ms)),
                error: record.error,
            }))
        }
        Err(_) => Ok(ClientRpcResponse::ClusterDeployStatusResult(ClusterDeployStatusResultResponse {
            is_found: false,
            deploy_id: None,
            status: None,
            artifact: None,
            nodes: vec![],
            started_at_ms: None,
            elapsed_ms: None,
            error: None,
        })),
    }
}

/// Handle ClusterRollback: initiate rollback of current/last deployment.
pub async fn handle_cluster_rollback(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let rpc_client =
        Arc::new(IrohNodeRpcClient::new(ctx.endpoint_manager.endpoint().clone(), ctx.controller.clone(), ctx.node_id));
    let coordinator = DeploymentCoordinator::new(ctx.kv_store.clone(), rpc_client, ctx.node_id);

    match coordinator.rollback_deployment().await {
        Ok(record) => Ok(ClientRpcResponse::ClusterRollbackResult(ClusterRollbackResultResponse {
            is_accepted: true,
            deploy_id: Some(record.deploy_id),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ClusterRollbackResult(ClusterRollbackResultResponse {
            is_accepted: false,
            deploy_id: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Node-level handlers
// ============================================================================

/// Handle NodeUpgrade: upgrade this node's binary.
///
/// Spawns the upgrade executor in a background task. The executor handles:
/// 1. Draining in-flight client RPCs
/// 2. Replacing the binary (Nix profile switch or blob download)
/// 3. Restarting the process (systemd or execve)
/// 4. Reporting status transitions to cluster KV
pub async fn handle_node_upgrade(
    ctx: &ClientProtocolContext,
    deploy_id: String,
    artifact: String,
) -> anyhow::Result<ClientRpcResponse> {
    info!(
        node_id = ctx.node_id,
        deploy_id = %deploy_id,
        artifact = %artifact,
        "received NodeUpgrade request, delegating to executor"
    );

    let parsed_artifact = DeployArtifact::parse(&artifact);
    let config = resolve_upgrade_config(ctx.node_id, &parsed_artifact);
    let kv = ctx.kv_store.clone();
    let node_id = ctx.node_id;

    // Spawn the executor in a background task so the RPC returns immediately.
    // The coordinator polls health to track progress.
    tokio::spawn(async move {
        let executor = aspen_cluster::upgrade::NodeUpgradeExecutor::new(config);
        let status_writer = KvStatusWriter { kv };

        match executor.execute(&parsed_artifact, Some(&status_writer)).await {
            Ok(()) => info!(node_id, deploy_id = %deploy_id, "node upgrade completed"),
            Err(e) => error!(node_id, deploy_id = %deploy_id, error = %e, "node upgrade failed"),
        }
    });

    Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
        is_accepted: true,
        error: None,
    }))
}

/// Handle NodeRollback: rollback this node's binary.
///
/// Calls the rollback logic from aspen-cluster which restores the previous
/// binary (Nix rollback or .bak restore) and restarts the process.
pub async fn handle_node_rollback(ctx: &ClientProtocolContext, deploy_id: String) -> anyhow::Result<ClientRpcResponse> {
    info!(
        node_id = ctx.node_id,
        deploy_id = %deploy_id,
        "received NodeRollback request, delegating to rollback executor"
    );

    // Read the current deployment record to determine the artifact type.
    // Fall back to Nix profile rollback if we can't determine the method.
    let upgrade_method = resolve_upgrade_method_for_rollback(ctx.node_id);
    let restart_method = resolve_restart_method();
    let node_id = ctx.node_id;

    tokio::spawn(async move {
        match aspen_cluster::upgrade::rollback::execute_rollback(&upgrade_method, &restart_method).await {
            Ok(()) => info!(node_id, deploy_id = %deploy_id, "node rollback completed"),
            Err(e) => error!(node_id, deploy_id = %deploy_id, error = %e, "node rollback failed"),
        }
    });

    Ok(ClientRpcResponse::NodeRollbackResult(NodeRollbackResultResponse {
        is_success: true,
        error: None,
    }))
}

// ============================================================================
// IrohNodeRpcClient — real inter-node RPC over iroh QUIC
// ============================================================================

/// NodeRpcClient implementation that sends real RPCs via iroh QUIC.
///
/// The coordinator (running on the Raft leader) uses this to send
/// NodeUpgrade, NodeRollback, and health check RPCs to follower nodes
/// via the CLIENT_ALPN protocol handler.
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
                // Node is healthy if it reports "healthy" status
                let is_healthy = health.status == "healthy";
                if !is_healthy {
                    info!(
                        target_node = node_id,
                        status = %health.status,
                        "node reports unhealthy"
                    );
                }
                Ok(is_healthy)
            }
            ClientRpcResponse::Error(e) => {
                warn!(target_node = node_id, code = %e.code, "health check returned error");
                Ok(false)
            }
            _ => {
                // Unexpected response type — treat as unhealthy but not an error
                warn!(target_node = node_id, "health check returned unexpected response type");
                Ok(false)
            }
        }
    }
}

// ============================================================================
// KvStatusWriter — reports upgrade status to cluster KV
// ============================================================================

/// StatusWriter that persists per-node deployment status to cluster KV.
///
/// Writes to `_sys:deploy:node:{node_id}` so the coordinator can track
/// upgrade progress on each node.
struct KvStatusWriter {
    kv: Arc<dyn KeyValueStore>,
}

#[async_trait::async_trait]
impl aspen_cluster::upgrade::executor::StatusWriter for KvStatusWriter {
    async fn write_status(
        &self,
        node_id: u64,
        status: &NodeDeployStatus,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = aspen_cluster::upgrade::status::node_status_key(node_id);
        let value = aspen_cluster::upgrade::status::serialize_status(status);

        self.kv
            .write(aspen_kv_types::WriteRequest::set(&key, value))
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

        info!(node_id, status = status.as_status_str(), "wrote deploy status to KV");
        Ok(())
    }
}

// ============================================================================
// Config resolution helpers
// ============================================================================

/// Resolve the NodeUpgradeConfig from the artifact type and environment.
///
/// Uses well-known defaults for NixOS deployments:
/// - Nix artifacts → nix-env --profile /nix/var/nix/profiles/aspen-node
/// - Blob artifacts → binary at /usr/local/bin/aspen-node
/// - Restart via systemd (unit: aspen-node)
///
/// Environment overrides:
/// - ASPEN_PROFILE_PATH: Nix profile path
/// - ASPEN_BINARY_PATH: Binary path for blob upgrades
/// - ASPEN_STAGING_DIR: Staging directory for blob downloads
/// - ASPEN_SYSTEMD_UNIT: Systemd unit name
/// - ASPEN_RESTART_METHOD: "systemd" (default) or "execve"
fn resolve_upgrade_config(node_id: u64, artifact: &DeployArtifact) -> aspen_cluster::upgrade::NodeUpgradeConfig {
    let upgrade_method = match artifact {
        DeployArtifact::NixStorePath(_) => {
            let profile_path = std::env::var("ASPEN_PROFILE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/nix/var/nix/profiles/aspen-node"));
            aspen_cluster::upgrade::UpgradeMethod::Nix { profile_path }
        }
        DeployArtifact::BlobHash(_) => {
            let binary_path = std::env::var("ASPEN_BINARY_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/aspen-node"));
            let staging_dir = std::env::var("ASPEN_STAGING_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp/aspen-staging"));
            aspen_cluster::upgrade::UpgradeMethod::Blob {
                binary_path,
                staging_dir,
            }
        }
    };

    let restart_method = resolve_restart_method();

    aspen_cluster::upgrade::NodeUpgradeConfig {
        node_id,
        upgrade_method,
        restart_method,
        drain_timeout_secs: aspen_constants::api::DRAIN_TIMEOUT_SECS,
    }
}

/// Resolve the upgrade method for rollback (uses Nix by default).
fn resolve_upgrade_method_for_rollback(_node_id: u64) -> aspen_cluster::upgrade::UpgradeMethod {
    let profile_path = std::env::var("ASPEN_PROFILE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/nix/var/nix/profiles/aspen-node"));
    aspen_cluster::upgrade::UpgradeMethod::Nix { profile_path }
}

/// Resolve the restart method from environment.
fn resolve_restart_method() -> aspen_cluster::upgrade::RestartMethod {
    match std::env::var("ASPEN_RESTART_METHOD").as_deref() {
        Ok("execve") => aspen_cluster::upgrade::RestartMethod::Execve,
        _ => {
            let unit_name = std::env::var("ASPEN_SYSTEMD_UNIT").unwrap_or_else(|_| "aspen-node".to_string());
            aspen_cluster::upgrade::RestartMethod::Systemd { unit_name }
        }
    }
}
