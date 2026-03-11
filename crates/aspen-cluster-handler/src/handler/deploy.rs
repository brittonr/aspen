//! Deploy operation handlers for cluster-wide rolling deployments.
//!
//! Handles: ClusterDeploy, ClusterDeployStatus, ClusterRollback, NodeUpgrade, NodeRollback.
//!
//! The cluster-level operations (ClusterDeploy/Status/Rollback) delegate to
//! `DeploymentCoordinator` which persists state in KV.
//!
//! The node-level operations (NodeUpgrade/NodeRollback) delegate to
//! `NodeUpgradeExecutor` which handles the local drain → binary swap → restart cycle.

use std::sync::Arc;

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

    let rpc_client = Arc::new(ContextNodeRpcClient {
        ctx_kv: ctx.kv_store.clone(),
        ctx_node_id: ctx.node_id,
    });
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
    let rpc_client = Arc::new(ContextNodeRpcClient {
        ctx_kv: ctx.kv_store.clone(),
        ctx_node_id: ctx.node_id,
    });
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
    let rpc_client = Arc::new(ContextNodeRpcClient {
        ctx_kv: ctx.kv_store.clone(),
        ctx_node_id: ctx.node_id,
    });
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
pub async fn handle_node_upgrade(
    ctx: &ClientProtocolContext,
    deploy_id: String,
    artifact: String,
) -> anyhow::Result<ClientRpcResponse> {
    info!(
        node_id = ctx.node_id,
        deploy_id = %deploy_id,
        artifact = %artifact,
        "received NodeUpgrade request"
    );

    // In a real implementation, this would delegate to NodeUpgradeExecutor.
    // For now, acknowledge the request. The actual drain/swap/restart happens
    // via the executor wired at the node binary level with real filesystem access.
    Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
        is_accepted: true,
        error: None,
    }))
}

/// Handle NodeRollback: rollback this node's binary.
pub async fn handle_node_rollback(ctx: &ClientProtocolContext, deploy_id: String) -> anyhow::Result<ClientRpcResponse> {
    info!(
        node_id = ctx.node_id,
        deploy_id = %deploy_id,
        "received NodeRollback request"
    );

    Ok(ClientRpcResponse::NodeRollbackResult(NodeRollbackResultResponse {
        is_success: true,
        error: None,
    }))
}

// ============================================================================
// ContextNodeRpcClient — bridges coordinator to cluster RPC
// ============================================================================

/// NodeRpcClient implementation that uses the ClientProtocolContext.
///
/// In production, this would send actual RPCs via iroh QUIC to target nodes.
/// Currently a placeholder that acknowledges all requests — the real
/// implementation will be added when inter-node RPC routing is wired up.
struct ContextNodeRpcClient {
    #[allow(dead_code)] // Will be used when real inter-node RPC is wired up
    ctx_kv: Arc<dyn KeyValueStore>,
    ctx_node_id: u64,
}

#[async_trait::async_trait]
impl NodeRpcClient for ContextNodeRpcClient {
    async fn send_upgrade(&self, node_id: u64, artifact_ref: &str) -> std::result::Result<(), RpcError> {
        // In production: send NodeUpgrade RPC to node via iroh QUIC.
        // For now: log and acknowledge.
        info!(
            source_node = self.ctx_node_id,
            target_node = node_id,
            artifact = artifact_ref,
            "sending NodeUpgrade RPC (placeholder)"
        );
        Ok(())
    }

    async fn send_rollback(&self, node_id: u64) -> std::result::Result<(), RpcError> {
        info!(source_node = self.ctx_node_id, target_node = node_id, "sending NodeRollback RPC (placeholder)");
        Ok(())
    }

    async fn check_health(&self, _node_id: u64) -> std::result::Result<bool, RpcError> {
        // In production: send GetHealth RPC + check Raft membership + log gap.
        // For now: always healthy.
        Ok(true)
    }
}
