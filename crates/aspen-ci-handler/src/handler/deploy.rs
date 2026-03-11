//! Deploy dispatch and monitoring for CI deploy stages.
//!
//! Bridges the `DeployDispatcher` trait from `aspen-ci` to the actual
//! cluster deploy infrastructure (`DeploymentCoordinator` from `aspen-deploy`).
//!
//! Deploy monitoring logic has been moved to `aspen_ci::orchestrator::deploy_monitor`
//! so the orchestrator can spawn monitors automatically for both trigger paths.
//! This module provides `RpcDeployDispatcher` which implements `DeployDispatcher`
//! via the cluster's `DeploymentCoordinator`, and re-exports the monitoring types.

use std::sync::Arc;

use aspen_ci::DeployDispatcher;
use aspen_ci::DeployInitResult;
use aspen_ci::DeployRequest;
use aspen_ci::DeployStatusResult;
use aspen_core::ClusterController;
use aspen_core::KeyValueStore;
use aspen_deploy::DeployArtifact;
use aspen_deploy::DeployStrategy;
use aspen_deploy::DeploymentCoordinator;
use aspen_deploy::NodeDeployStatus;
use aspen_deploy::coordinator::rpc::NodeRpcClient;
use aspen_deploy::coordinator::rpc::RpcError;
use tracing::error;
use tracing::info;

// ============================================================================
// RpcDeployDispatcher — implements DeployDispatcher via DeploymentCoordinator
// ============================================================================

/// Bridges `DeployDispatcher` to the cluster's `DeploymentCoordinator`.
///
/// Uses the same approach as `handle_cluster_deploy` in the cluster handler:
/// creates a `DeploymentCoordinator` with the node's KV store and a
/// placeholder `NodeRpcClient`, then dispatches deploy and status queries.
pub struct RpcDeployDispatcher {
    kv_store: Arc<dyn KeyValueStore>,
    controller: Arc<dyn ClusterController>,
    node_id: u64,
}

impl RpcDeployDispatcher {
    /// Create a new deploy dispatcher.
    pub fn new(kv_store: Arc<dyn KeyValueStore>, controller: Arc<dyn ClusterController>, node_id: u64) -> Self {
        Self {
            kv_store,
            controller,
            node_id,
        }
    }
}

#[async_trait::async_trait]
impl DeployDispatcher for RpcDeployDispatcher {
    async fn deploy(&self, request: DeployRequest) -> Result<DeployInitResult, String> {
        let parsed_artifact = DeployArtifact::parse(&request.artifact);
        let strategy = match request.strategy.as_str() {
            "rolling" | "" => DeployStrategy::rolling(request.max_concurrent),
            other => {
                return Ok(DeployInitResult {
                    is_accepted: false,
                    deploy_id: None,
                    error: Some(format!("unknown strategy: {other}")),
                });
            }
        };

        // Get cluster members for node list
        let metrics = self.controller.get_metrics().await.map_err(|e| format!("failed to get cluster metrics: {e}"))?;
        let node_ids: Vec<u64> = metrics.voters.clone();

        if node_ids.is_empty() {
            return Ok(DeployInitResult {
                is_accepted: false,
                deploy_id: None,
                error: Some("no voters in cluster".to_string()),
            });
        }

        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let deploy_id = format!("deploy-{now_ms}");

        let rpc_client = Arc::new(PlaceholderNodeRpcClient { node_id: self.node_id });
        let coordinator = DeploymentCoordinator::with_timeouts(
            self.kv_store.clone(),
            rpc_client.clone(),
            self.node_id,
            request.health_timeout_secs,
            aspen_constants::api::DEPLOY_STATUS_POLL_INTERVAL_SECS,
        );

        match coordinator.start_deployment(deploy_id.clone(), parsed_artifact, strategy, &node_ids, now_ms).await {
            Ok(record) => {
                info!(deploy_id = %record.deploy_id, "deployment accepted via CI deploy dispatcher");

                // Spawn background task to run the deployment
                let deploy_id_bg = record.deploy_id.clone();
                let kv = self.kv_store.clone();
                let nid = self.node_id;
                let ht = request.health_timeout_secs;
                let pi = aspen_constants::api::DEPLOY_STATUS_POLL_INTERVAL_SECS;
                tokio::spawn(async move {
                    let coord = DeploymentCoordinator::with_timeouts(kv, rpc_client, nid, ht, pi);
                    match coord.run_deployment(&deploy_id_bg).await {
                        Ok(r) => info!(deploy_id = %r.deploy_id, status = ?r.status, "deployment finished"),
                        Err(e) => error!(deploy_id = %deploy_id_bg, error = %e, "deployment failed"),
                    }
                });

                Ok(DeployInitResult {
                    is_accepted: true,
                    deploy_id: Some(record.deploy_id),
                    error: None,
                })
            }
            Err(e) => Ok(DeployInitResult {
                is_accepted: false,
                deploy_id: None,
                error: Some(e.to_string()),
            }),
        }
    }

    async fn deploy_status(&self) -> Result<DeployStatusResult, String> {
        let rpc_client = Arc::new(PlaceholderNodeRpcClient { node_id: self.node_id });
        let coordinator = DeploymentCoordinator::new(self.kv_store.clone(), rpc_client, self.node_id);

        match coordinator.get_status().await {
            Ok(record) => {
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;

                let nodes: Vec<aspen_ci::DeployNodeStatus> = record
                    .nodes
                    .iter()
                    .map(|n| {
                        let (status_str, err) = match &n.status {
                            NodeDeployStatus::Failed(reason) => ("failed".to_string(), Some(reason.clone())),
                            other => (other.as_status_str().to_string(), None),
                        };
                        aspen_ci::DeployNodeStatus {
                            node_id: n.node_id,
                            status: status_str,
                            error: err,
                        }
                    })
                    .collect();

                Ok(DeployStatusResult {
                    is_found: true,
                    overall_status: Some(format!("{:?}", record.status).to_lowercase()),
                    nodes,
                    elapsed_ms: Some(now_ms.saturating_sub(record.created_at_ms)),
                    error: record.error,
                })
            }
            Err(_) => Ok(DeployStatusResult {
                is_found: false,
                overall_status: None,
                nodes: vec![],
                elapsed_ms: None,
                error: None,
            }),
        }
    }
}

// ============================================================================
// PlaceholderNodeRpcClient — bridges coordinator to cluster RPC
// ============================================================================

/// Placeholder NodeRpcClient that logs operations.
///
/// In production, this sends actual RPCs via iroh QUIC to target nodes.
/// Currently acknowledges all requests — the real inter-node RPC routing
/// will be wired up when per-node upgrade execution is implemented.
struct PlaceholderNodeRpcClient {
    node_id: u64,
}

#[async_trait::async_trait]
impl NodeRpcClient for PlaceholderNodeRpcClient {
    async fn send_upgrade(&self, node_id: u64, _deploy_id: &str, artifact_ref: &str) -> Result<(), RpcError> {
        info!(
            source_node = self.node_id,
            target_node = node_id,
            artifact = artifact_ref,
            "sending NodeUpgrade RPC (CI deploy dispatcher)"
        );
        Ok(())
    }

    async fn send_rollback(&self, node_id: u64, _deploy_id: &str) -> Result<(), RpcError> {
        info!(source_node = self.node_id, target_node = node_id, "sending NodeRollback RPC (CI deploy dispatcher)");
        Ok(())
    }

    async fn check_health(&self, _node_id: u64) -> Result<bool, RpcError> {
        Ok(true)
    }
}

// ============================================================================
// Deploy stage monitoring — delegated to aspen-ci orchestrator
// ============================================================================
//
// Deploy stage types and monitoring have been moved to
// `aspen_ci::orchestrator::deploy_monitor`. The orchestrator now
// spawns deploy monitors automatically when a deploy dispatcher is
// configured, covering both the direct RPC trigger and the
// auto-trigger (gossip) paths.
//
// Re-export for backward compatibility.

pub use aspen_ci::DeployJobInfo;
pub use aspen_ci::DeployStageInfo;
pub use aspen_ci::orchestrator::deploy_monitor::extract_deploy_stages;
pub use aspen_ci::orchestrator::deploy_monitor::spawn_deploy_monitor;

#[cfg(test)]
mod tests {
    use aspen_ci::config::types::JobConfig;
    use aspen_ci::config::types::JobType;
    use aspen_ci::config::types::PipelineConfig;
    use aspen_ci::config::types::StageConfig;

    use super::*;

    #[test]
    fn test_extract_deploy_stages_empty() {
        let config = test_pipeline(vec![]);
        let stages = extract_deploy_stages(&config);
        assert!(stages.is_empty());
    }

    #[test]
    fn test_extract_deploy_stages_no_deploys() {
        let config = test_pipeline(vec![StageConfig {
            name: "build".to_string(),
            jobs: vec![test_job("build-node", JobType::Shell)],
            ..test_stage()
        }]);
        let stages = extract_deploy_stages(&config);
        assert!(stages.is_empty());
    }

    #[test]
    fn test_extract_deploy_stages_with_deploy() {
        let config = test_pipeline(vec![
            StageConfig {
                name: "build".to_string(),
                jobs: vec![test_job("build-node", JobType::Nix)],
                ..test_stage()
            },
            StageConfig {
                name: "deploy".to_string(),
                jobs: vec![JobConfig {
                    name: "deploy-node".to_string(),
                    job_type: JobType::Deploy,
                    artifact_from: Some("build-node".to_string()),
                    strategy: Some("rolling".to_string()),
                    ..test_job_config()
                }],
                depends_on: vec!["build".to_string()],
                parallel: false,
                ..test_stage()
            },
        ]);
        let stages = extract_deploy_stages(&config);
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].stage_name, "deploy");
        assert_eq!(stages[0].jobs.len(), 1);
        assert_eq!(stages[0].jobs[0].name, "deploy-node");
        assert_eq!(stages[0].jobs[0].artifact_from, "build-node");
    }

    #[test]
    fn test_extract_deploy_stages_mixed_stage_excluded() {
        // A stage with both shell and deploy jobs is NOT deploy-only,
        // so it's not extracted as a deploy stage.
        let config = test_pipeline(vec![StageConfig {
            name: "mixed".to_string(),
            jobs: vec![test_job("build-step", JobType::Shell), JobConfig {
                name: "deploy-step".to_string(),
                job_type: JobType::Deploy,
                artifact_from: Some("build-step".to_string()),
                ..test_job_config()
            }],
            ..test_stage()
        }]);
        let stages = extract_deploy_stages(&config);
        assert!(stages.is_empty(), "mixed stages should not be extracted as deploy stages");
    }

    fn test_pipeline(stages: Vec<StageConfig>) -> PipelineConfig {
        PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: Default::default(),
            stages,
            artifacts: Default::default(),
            env: Default::default(),
            timeout_secs: 3600,
            priority: Default::default(),
        }
    }

    fn test_stage() -> StageConfig {
        StageConfig {
            name: String::new(),
            jobs: vec![],
            parallel: true,
            depends_on: vec![],
            when: None,
        }
    }

    fn test_job(name: &str, job_type: JobType) -> JobConfig {
        JobConfig {
            name: name.to_string(),
            job_type,
            ..test_job_config()
        }
    }

    fn test_job_config() -> JobConfig {
        JobConfig {
            name: String::new(),
            job_type: JobType::Shell,
            command: None,
            args: vec![],
            env: Default::default(),
            working_dir: None,
            flake_url: None,
            flake_attr: None,
            binary_hash: None,
            timeout_secs: 3600,
            isolation: Default::default(),
            cache_key: None,
            artifacts: vec![],
            depends_on: vec![],
            retry_count: 0,
            allow_failure: false,
            tags: vec![],
            should_upload_result: false,
            publish_to_cache: false,
            artifact_from: None,
            strategy: None,
            health_check_timeout_secs: None,
            max_concurrent: None,
        }
    }
}
