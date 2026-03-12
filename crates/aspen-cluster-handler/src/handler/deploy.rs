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

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::messages::deploy::*;
use aspen_deploy::DeployArtifact;
use aspen_deploy::DeployStrategy;
use aspen_deploy::DeploymentCoordinator;
use aspen_deploy::IrohNodeRpcClient;
use aspen_deploy::NodeDeployStatus;
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

    let rpc_client =
        Arc::new(IrohNodeRpcClient::new(ctx.endpoint_manager.endpoint().clone(), ctx.controller.clone(), ctx.node_id));
    let coordinator = DeploymentCoordinator::with_cluster_controller_and_timeouts(
        ctx.kv_store.clone(),
        rpc_client.clone(),
        ctx.controller.clone(),
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
            let cc = ctx.controller.clone();
            let node_id = ctx.node_id;
            let ht = health_timeout_secs;
            let pi = aspen_constants::api::DEPLOY_STATUS_POLL_INTERVAL_SECS;
            tokio::spawn(async move {
                let coord =
                    DeploymentCoordinator::with_cluster_controller_and_timeouts(kv, rpc_client, cc, node_id, ht, pi);
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
    let coordinator: DeploymentCoordinator<dyn KeyValueStore, _, dyn aspen_traits::ClusterController> =
        DeploymentCoordinator::new(ctx.kv_store.clone(), rpc_client, ctx.node_id);

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
    let coordinator: DeploymentCoordinator<dyn KeyValueStore, _, dyn aspen_traits::ClusterController> =
        DeploymentCoordinator::new(ctx.kv_store.clone(), rpc_client, ctx.node_id);

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

    // For blob artifacts, validate blob store availability and stage the binary.
    // The executor expects the staged binary to already exist on disk.
    if let DeployArtifact::BlobHash(ref blob_hash) = parsed_artifact {
        #[cfg(feature = "blob")]
        {
            let blob_store = match &ctx.blob_store {
                Some(store) => store.clone(),
                None => {
                    info!(node_id = ctx.node_id, "blob artifact rejected: blob store not configured");
                    return Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
                        is_accepted: false,
                        error: Some(
                            "blob upgrades require the blob feature to be enabled and a blob store configured"
                                .to_string(),
                        ),
                    }));
                }
            };

            let staging_dir = std::env::var("ASPEN_STAGING_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/aspen-staging"));
            if let Err(e) = tokio::fs::create_dir_all(&staging_dir).await {
                return Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
                    is_accepted: false,
                    error: Some(format!("failed to create staging directory: {e}")),
                }));
            }

            let staging_path = staging_dir.join(format!("aspen-node-{}", blob_hash));
            let timeout = std::time::Duration::from_secs(aspen_constants::api::DEPLOY_HEALTH_TIMEOUT_SECS);

            info!(
                node_id = ctx.node_id,
                blob_hash = %blob_hash,
                staging_path = %staging_path.display(),
                "downloading blob artifact to staging directory"
            );

            // Parse the blob hash and fetch from the local blob store via BlobRead trait.
            let download_result = tokio::time::timeout(timeout, async {
                use aspen_blob::BlobRead;

                let hash: iroh_blobs::Hash =
                    blob_hash.parse().map_err(|e| format!("invalid blob hash '{blob_hash}': {e}"))?;

                // Read blob content via BlobRead::get_bytes
                let data = blob_store
                    .get_bytes(&hash)
                    .await
                    .map_err(|e| format!("blob store error: {e}"))?
                    .ok_or_else(|| format!("blob hash {} not found in local store", hash))?;

                // Write to staging file
                tokio::fs::write(&staging_path, &data)
                    .await
                    .map_err(|e| format!("failed to write staged blob: {e}"))?;

                // Set executable permission
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o755);
                    tokio::fs::set_permissions(&staging_path, perms)
                        .await
                        .map_err(|e| format!("failed to set permissions: {e}"))?;
                }

                Ok::<(), String>(())
            })
            .await;

            match download_result {
                Ok(Ok(())) => {
                    info!(node_id = ctx.node_id, blob_hash = %blob_hash, "blob artifact staged");
                }
                Ok(Err(e)) => {
                    return Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
                        is_accepted: false,
                        error: Some(format!("blob download failed: {e}")),
                    }));
                }
                Err(_) => {
                    return Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
                        is_accepted: false,
                        error: Some("blob download timed out".to_string()),
                    }));
                }
            }
        }

        #[cfg(not(feature = "blob"))]
        {
            let _ = blob_hash;
            return Ok(ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
                is_accepted: false,
                error: Some("blob upgrades require the blob feature to be enabled".to_string()),
            }));
        }
    }

    let config = resolve_upgrade_config(ctx.node_id, &parsed_artifact);
    let kv = ctx.kv_store.clone();
    let node_id = ctx.node_id;

    // Use the shared drain state from the RPC context if available,
    // so the executor's drain phase waits for real in-flight client RPCs.
    #[cfg(feature = "deploy")]
    let drain_state = ctx.drain_state.clone();

    // Spawn the executor in a background task so the RPC returns immediately.
    // The coordinator polls health to track progress.
    tokio::spawn(async move {
        #[cfg(feature = "deploy")]
        let executor = match drain_state {
            Some(ds) => aspen_cluster::upgrade::NodeUpgradeExecutor::with_drain_state(config, ds),
            None => aspen_cluster::upgrade::NodeUpgradeExecutor::new(config),
        };
        #[cfg(not(feature = "deploy"))]
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
