//! Deploy stage monitoring for pipelines with deploy jobs.
//!
//! When a pipeline has deploy stages (stages where all jobs are `JobType::Deploy`),
//! a background monitor task is spawned after the pipeline run starts. The monitor:
//!
//! 1. Polls until the workflow portion (non-deploy stages) completes or fails
//! 2. Runs deploy stages in order via `DeployExecutor`
//! 3. Sets the final pipeline status via `complete_deploy_stages()`
//!
//! This module lives in `aspen-ci` so both trigger paths (direct RPC and
//! auto-trigger via gossip) get deploy monitoring automatically.

use std::sync::Arc;
use std::time::Duration;

use aspen_core::KeyValueStore;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::types::JobType;
use crate::config::types::PipelineConfig;
use crate::orchestrator::DeployDispatcher;
use crate::orchestrator::DeployExecutor;
use crate::orchestrator::DeployJobParams;
use crate::orchestrator::DeployJobResult;
use crate::orchestrator::PipelineOrchestrator;
use crate::orchestrator::PipelineStatus;

/// Poll interval for checking workflow completion before running deploys.
const DEPLOY_MONITOR_POLL_SECS: u64 = 5;

/// Maximum time to wait for workflow completion (2 hours).
const DEPLOY_MONITOR_MAX_WAIT_SECS: u64 = 7200;

// ============================================================================
// Deploy stage info types
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
    /// Binary to validate inside a Nix store path.
    pub expected_binary: Option<String>,
    /// Whether to track deployment lifecycle state in Raft KV.
    /// `None` defaults to `true` (stateful) for backwards compatibility.
    pub stateful: Option<bool>,
}

// ============================================================================
// Extract deploy stages from config
// ============================================================================

/// Extract deploy stage info from a pipeline config.
///
/// Returns deploy stages in pipeline order. Only stages where ALL jobs
/// are `JobType::Deploy` are included. Stages with a `when` guard that
/// doesn't match `ref_name` are excluded.
pub fn extract_deploy_stages(config: &PipelineConfig) -> Vec<DeployStageInfo> {
    extract_deploy_stages_for_ref(config, None)
}

/// Extract deploy stages that should run for a given ref.
///
/// When `ref_name` is `Some`, stages guarded by `when` are filtered by
/// `should_run()`. When `None`, all deploy-only stages are returned
/// (backwards-compatible with the original behavior).
pub fn extract_deploy_stages_for_ref(config: &PipelineConfig, ref_name: Option<&str>) -> Vec<DeployStageInfo> {
    config
        .stages_in_order()
        .iter()
        .filter(|s| {
            PipelineOrchestrator::<dyn KeyValueStore>::is_deploy_only_stage(s)
                && ref_name.is_none_or(|r| s.should_run(r))
        })
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
                    expected_binary: j.expected_binary.clone(),
                    stateful: j.stateful,
                })
                .collect(),
        })
        .collect()
}

// ============================================================================
// Deploy monitor task
// ============================================================================

/// Spawn a background task that monitors workflow completion and runs
/// deploy stages inline via `DeployExecutor`.
///
/// This task:
/// 1. Polls the pipeline run until the workflow reaches "done" (non-deploy stages all succeeded) or
///    a terminal failure state.
/// 2. For each deploy stage (in order), runs all deploy jobs via `DeployExecutor::execute()`.
/// 3. Updates deploy stage/job status in the pipeline run.
/// 4. Sets the final pipeline status via `complete_deploy_stages()`.
pub fn spawn_deploy_monitor<S: KeyValueStore + ?Sized + 'static>(
    orchestrator: Arc<PipelineOrchestrator<S>>,
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
async fn wait_for_workflow<S: KeyValueStore + ?Sized + 'static>(
    orchestrator: &PipelineOrchestrator<S>,
    run_id: &str,
) -> Option<bool> {
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
async fn run_all_deploy_stages<S: KeyValueStore + ?Sized + 'static>(
    orchestrator: &PipelineOrchestrator<S>,
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
                expected_binary: job.expected_binary.as_deref(),
                stateful: job.stateful,
                pipeline_run: &pipeline_run,
                dispatcher,
            };

            let executor = DeployExecutor::new(orchestrator.kv_store());

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

/// Update a deploy stage's status in the pipeline run.
async fn update_deploy_stage_status<S: KeyValueStore + ?Sized + 'static>(
    orchestrator: &PipelineOrchestrator<S>,
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
async fn update_deploy_job_status<S: KeyValueStore + ?Sized + 'static>(
    orchestrator: &PipelineOrchestrator<S>,
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
    use super::*;
    use crate::config::types::JobConfig;
    use crate::config::types::StageConfig;

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

    #[test]
    fn test_extract_deploy_stages_when_guard_skips_non_matching_ref() {
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
                    ..test_job_config()
                }],
                depends_on: vec!["build".to_string()],
                when: Some("refs/tags/release/*".to_string()),
                ..test_stage()
            },
        ]);

        // Non-matching ref: deploy stage should be excluded
        let stages = extract_deploy_stages_for_ref(&config, Some("refs/heads/main"));
        assert!(stages.is_empty(), "deploy stage with when guard should be skipped for non-matching ref");

        // Matching ref: deploy stage should be included
        let stages = extract_deploy_stages_for_ref(&config, Some("refs/tags/release/v1.0"));
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].stage_name, "deploy");

        // No ref (backwards compat): all deploy stages included
        let stages = extract_deploy_stages_for_ref(&config, None);
        assert_eq!(stages.len(), 1);
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
            expected_binary: None,
            stateful: None,
        }
    }
}
