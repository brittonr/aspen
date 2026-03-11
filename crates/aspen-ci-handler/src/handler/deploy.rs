//! Deploy dispatch and monitoring for CI deploy stages.
//!
//! Bridges the `DeployDispatcher` trait from `aspen-ci` to the actual
//! cluster deploy infrastructure (`DeploymentCoordinator` from `aspen-deploy`).
//! Also provides the background task that monitors workflow completion and
//! runs deploy stages inline via `DeployExecutor`.

use std::sync::Arc;
use std::time::Duration;

use aspen_ci::DeployDispatcher;
use aspen_ci::DeployExecutor;
use aspen_ci::DeployInitResult;
use aspen_ci::DeployJobParams;
use aspen_ci::DeployJobResult;
use aspen_ci::DeployRequest;
use aspen_ci::DeployStatusResult;
use aspen_ci::PipelineOrchestrator;
use aspen_ci::PipelineStatus;
use aspen_ci::config::types::JobType;
use aspen_ci::config::types::PipelineConfig;
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
use tracing::warn;

/// Poll interval for checking workflow completion before running deploys.
const DEPLOY_MONITOR_POLL_SECS: u64 = 5;

/// Maximum time to wait for workflow completion (2 hours).
const DEPLOY_MONITOR_MAX_WAIT_SECS: u64 = 7200;

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
    async fn send_upgrade(&self, node_id: u64, artifact_ref: &str) -> Result<(), RpcError> {
        info!(
            source_node = self.node_id,
            target_node = node_id,
            artifact = artifact_ref,
            "sending NodeUpgrade RPC (CI deploy dispatcher)"
        );
        Ok(())
    }

    async fn send_rollback(&self, node_id: u64) -> Result<(), RpcError> {
        info!(source_node = self.node_id, target_node = node_id, "sending NodeRollback RPC (CI deploy dispatcher)");
        Ok(())
    }

    async fn check_health(&self, _node_id: u64) -> Result<bool, RpcError> {
        Ok(true)
    }
}

// ============================================================================
// Deploy stage monitoring
// ============================================================================

/// Metadata about a deploy stage extracted from a pipeline config.
#[derive(Debug, Clone)]
pub struct DeployStageInfo {
    /// Stage name.
    pub stage_name: String,
    /// Deploy jobs in this stage.
    pub jobs: Vec<DeployJobInfo>,
}

/// Metadata about a single deploy job.
#[derive(Debug, Clone)]
pub struct DeployJobInfo {
    /// Job name.
    pub name: String,
    /// Name of the build job to resolve artifacts from.
    pub artifact_from: String,
    /// Deployment strategy override.
    pub strategy: Option<String>,
    /// Health check timeout override.
    pub health_timeout_secs: Option<u64>,
    /// Max concurrent node upgrades override.
    pub max_concurrent: Option<u32>,
}

/// Extract deploy stage info from a pipeline config.
///
/// Returns deploy stages in pipeline order. Only stages where ALL jobs
/// are `JobType::Deploy` are included.
pub fn extract_deploy_stages(config: &PipelineConfig) -> Vec<DeployStageInfo> {
    config
        .stages_in_order()
        .iter()
        .filter(|s| PipelineOrchestrator::<dyn KeyValueStore>::is_deploy_only_stage(s))
        .map(|s| DeployStageInfo {
            stage_name: s.name.clone(),
            jobs: s
                .jobs
                .iter()
                .filter(|j| j.job_type == JobType::Deploy)
                .map(|j| DeployJobInfo {
                    name: j.name.clone(),
                    artifact_from: j.artifact_from.clone().unwrap_or_default(),
                    strategy: j.strategy.clone(),
                    health_timeout_secs: j.health_check_timeout_secs,
                    max_concurrent: j.max_concurrent,
                })
                .collect(),
        })
        .collect()
}

/// Spawn a background task that monitors workflow completion and runs
/// deploy stages inline via `DeployExecutor`.
///
/// This task:
/// 1. Polls the pipeline run until the workflow reaches "done" (non-deploy stages all succeeded) or
///    a terminal failure state.
/// 2. For each deploy stage (in order), runs all deploy jobs via `DeployExecutor::execute()`.
/// 3. Updates deploy stage/job status in the pipeline run.
/// 4. Sets the final pipeline status via `complete_deploy_stages()`.
pub fn spawn_deploy_monitor(
    orchestrator: Arc<PipelineOrchestrator<dyn KeyValueStore>>,
    dispatcher: Arc<dyn DeployDispatcher>,
    run_id: String,
    deploy_stages: Vec<DeployStageInfo>,
) {
    tokio::spawn(async move {
        info!(run_id = %run_id, deploy_stages = deploy_stages.len(), "deploy monitor started");

        // Step 1: Wait for workflow to finish (non-deploy stages)
        let workflow_succeeded = match wait_for_workflow(&orchestrator, &run_id).await {
            Some(ok) => ok,
            None => {
                warn!(run_id = %run_id, "deploy monitor: timed out waiting for workflow");
                orchestrator.complete_deploy_stages(&run_id, false).await;
                return;
            }
        };

        if !workflow_succeeded {
            info!(run_id = %run_id, "deploy monitor: workflow failed, skipping deploy stages");
            orchestrator.complete_deploy_stages(&run_id, false).await;
            return;
        }

        info!(run_id = %run_id, "deploy monitor: workflow succeeded, running deploy stages");

        // Step 2: Run deploy stages in order
        let deploy_success = run_all_deploy_stages(&orchestrator, dispatcher.as_ref(), &run_id, &deploy_stages).await;

        // Step 3: Set final pipeline status
        orchestrator.complete_deploy_stages(&run_id, deploy_success).await;

        if deploy_success {
            info!(run_id = %run_id, "deploy monitor: all deploy stages succeeded");
        } else {
            warn!(run_id = %run_id, "deploy monitor: deploy stages failed");
        }
    });
}

/// Poll until the workflow portion of the pipeline completes.
///
/// Returns `Some(true)` if the workflow succeeded, `Some(false)` if it
/// failed, or `None` on timeout.
async fn wait_for_workflow(orchestrator: &PipelineOrchestrator<dyn KeyValueStore>, run_id: &str) -> Option<bool> {
    let poll_interval = Duration::from_secs(DEPLOY_MONITOR_POLL_SECS);
    let max_polls = DEPLOY_MONITOR_MAX_WAIT_SECS / DEPLOY_MONITOR_POLL_SECS;

    for _ in 0..max_polls {
        tokio::time::sleep(poll_interval).await;

        let run = match orchestrator.get_run(run_id).await {
            Some(r) => r,
            None => {
                warn!(run_id = %run_id, "deploy monitor: pipeline run not found");
                return Some(false);
            }
        };

        // If the pipeline already failed (e.g., workflow failed), stop.
        if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
            return Some(false);
        }

        // Check if workflow itself is done
        match orchestrator.is_workflow_done(&run).await {
            Some(true) => return Some(true),
            Some(false) => return Some(false),
            None => continue, // Still running
        }
    }

    None // Timed out
}

/// Run all deploy stages in order. Returns true if all succeeded.
async fn run_all_deploy_stages(
    orchestrator: &PipelineOrchestrator<dyn KeyValueStore>,
    dispatcher: &dyn DeployDispatcher,
    run_id: &str,
    deploy_stages: &[DeployStageInfo],
) -> bool {
    for stage in deploy_stages {
        info!(
            run_id = %run_id,
            stage = %stage.stage_name,
            jobs = stage.jobs.len(),
            "running deploy stage"
        );

        // Mark stage as running
        update_deploy_stage_status(orchestrator, run_id, &stage.stage_name, PipelineStatus::Running).await;

        let mut stage_success = true;

        for job in &stage.jobs {
            info!(
                run_id = %run_id,
                stage = %stage.stage_name,
                job = %job.name,
                artifact_from = %job.artifact_from,
                "running deploy job"
            );

            // Mark job as running
            update_deploy_job_status(orchestrator, run_id, &stage.stage_name, &job.name, PipelineStatus::Running, None)
                .await;

            // Get the pipeline run to pass to the executor
            let pipeline_run = match orchestrator.get_run(run_id).await {
                Some(r) => r,
                None => {
                    error!(run_id = %run_id, "pipeline run not found during deploy execution");
                    return false;
                }
            };

            let params = DeployJobParams {
                run_id,
                job_name: &job.name,
                artifact_from: &job.artifact_from,
                strategy: job.strategy.as_deref(),
                health_timeout_secs: job.health_timeout_secs,
                max_concurrent: job.max_concurrent,
                pipeline_run: &pipeline_run,
                dispatcher,
            };

            // Create a DeployExecutor. We need the KV store — extract it
            // from the orchestrator by reading a run (it's the same store).
            // The executor only needs it for artifact resolution.
            let executor = DeployExecutor::new(get_kv_store(orchestrator));

            match executor.execute(params).await {
                Ok(DeployJobResult::Success { deploy_id, artifact }) => {
                    info!(
                        run_id = %run_id,
                        job = %job.name,
                        deploy_id = %deploy_id,
                        artifact = %artifact,
                        "deploy job succeeded"
                    );
                    update_deploy_job_status(
                        orchestrator,
                        run_id,
                        &stage.stage_name,
                        &job.name,
                        PipelineStatus::Success,
                        None,
                    )
                    .await;
                }
                Ok(DeployJobResult::Failed { error, .. }) => {
                    error!(
                        run_id = %run_id,
                        job = %job.name,
                        error = %error,
                        "deploy job failed"
                    );
                    update_deploy_job_status(
                        orchestrator,
                        run_id,
                        &stage.stage_name,
                        &job.name,
                        PipelineStatus::Failed,
                        Some(error),
                    )
                    .await;
                    stage_success = false;
                    break; // Stop stage on first failure
                }
                Err(e) => {
                    error!(
                        run_id = %run_id,
                        job = %job.name,
                        error = %e,
                        "deploy job execution error"
                    );
                    update_deploy_job_status(
                        orchestrator,
                        run_id,
                        &stage.stage_name,
                        &job.name,
                        PipelineStatus::Failed,
                        Some(e.to_string()),
                    )
                    .await;
                    stage_success = false;
                    break;
                }
            }
        }

        // Update stage status
        let stage_status = if stage_success {
            PipelineStatus::Success
        } else {
            PipelineStatus::Failed
        };
        update_deploy_stage_status(orchestrator, run_id, &stage.stage_name, stage_status).await;

        if !stage_success {
            return false;
        }
    }

    true
}

/// Get the KV store from the orchestrator.
///
/// The orchestrator wraps its KV store in Arc. We access it through
/// a method that exposes it for deploy execution.
fn get_kv_store(orchestrator: &PipelineOrchestrator<dyn KeyValueStore>) -> Arc<dyn KeyValueStore> {
    orchestrator.kv_store()
}

/// Update a deploy stage's status in the pipeline run.
async fn update_deploy_stage_status(
    orchestrator: &PipelineOrchestrator<dyn KeyValueStore>,
    run_id: &str,
    stage_name: &str,
    status: PipelineStatus,
) {
    if let Some(mut run) = orchestrator.get_run(run_id).await {
        for stage in &mut run.stages {
            if stage.name == stage_name {
                stage.status = status;
                if status == PipelineStatus::Running && stage.started_at.is_none() {
                    stage.started_at = Some(chrono::Utc::now());
                }
                if status.is_terminal() && stage.completed_at.is_none() {
                    stage.completed_at = Some(chrono::Utc::now());
                }
                break;
            }
        }
        orchestrator.persist_run_update(&run).await;
    }
}

/// Update a deploy job's status in the pipeline run.
async fn update_deploy_job_status(
    orchestrator: &PipelineOrchestrator<dyn KeyValueStore>,
    run_id: &str,
    stage_name: &str,
    job_name: &str,
    status: PipelineStatus,
    error: Option<String>,
) {
    if let Some(mut run) = orchestrator.get_run(run_id).await {
        for stage in &mut run.stages {
            if stage.name == stage_name {
                if let Some(job) = stage.jobs.get_mut(job_name) {
                    job.status = status;
                    job.error = error;
                    if status == PipelineStatus::Running && job.started_at.is_none() {
                        job.started_at = Some(chrono::Utc::now());
                    }
                    if status.is_terminal() && job.completed_at.is_none() {
                        job.completed_at = Some(chrono::Utc::now());
                    }
                }
                break;
            }
        }
        orchestrator.persist_run_update(&run).await;
    }
}

#[cfg(test)]
mod tests {
    use aspen_ci::config::types::JobConfig;
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
