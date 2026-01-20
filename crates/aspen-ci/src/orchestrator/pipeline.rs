//! Pipeline orchestrator for CI/CD execution.
//!
//! This module converts CI pipeline configurations into aspen-jobs workflows
//! and manages pipeline run lifecycle.
//!
//! # Architecture
//!
//! ```text
//! PipelineConfig        PipelineOrchestrator         aspen-jobs
//! ┌────────────┐       ┌────────────────────┐       ┌─────────────┐
//! │ stages[]   │──────►│ Convert to         │──────►│ Workflow    │
//! │ jobs[]     │       │ WorkflowDefinition │       │ Manager     │
//! │ depends_on │       └────────────────────┘       └─────────────┘
//! └────────────┘                │                          │
//!                               │                          │
//!                    ┌──────────▼──────────┐               │
//!                    │    PipelineRun      │◄──────────────┘
//!                    │ - status            │   (status updates)
//!                    │ - stage_statuses    │
//!                    │ - job_results       │
//!                    └─────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::TransitionCondition;
use aspen_jobs::WorkflowDefinition;
use aspen_jobs::WorkflowManager;
use aspen_jobs::WorkflowStep;
use aspen_jobs::WorkflowTransition;

use crate::config::types::JobConfig;
use crate::config::types::JobType;
use crate::config::types::PipelineConfig;
use crate::error::CiError;
use crate::error::Result;
use crate::workers::NixBuildPayload;

// Tiger Style: Bounded resources
/// Maximum concurrent pipeline runs per repository.
const MAX_CONCURRENT_RUNS_PER_REPO: usize = 5;
/// Maximum total concurrent pipeline runs.
const MAX_TOTAL_CONCURRENT_RUNS: usize = 50;
/// Maximum jobs per pipeline.
const MAX_JOBS_PER_PIPELINE: usize = 100;
/// Default step timeout (30 minutes).
const DEFAULT_STEP_TIMEOUT_SECS: u64 = 1800;

/// Configuration for the PipelineOrchestrator.
#[derive(Debug, Clone)]
pub struct PipelineOrchestratorConfig {
    /// Maximum concurrent runs per repository.
    pub max_runs_per_repo: usize,
    /// Maximum total concurrent runs.
    pub max_total_runs: usize,
    /// Default step timeout.
    pub default_step_timeout: Duration,
}

impl Default for PipelineOrchestratorConfig {
    fn default() -> Self {
        Self {
            max_runs_per_repo: MAX_CONCURRENT_RUNS_PER_REPO,
            max_total_runs: MAX_TOTAL_CONCURRENT_RUNS,
            default_step_timeout: Duration::from_secs(DEFAULT_STEP_TIMEOUT_SECS),
        }
    }
}

/// Context for a pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    /// Repository ID.
    pub repo_id: RepoId,
    /// Commit hash being built.
    pub commit_hash: [u8; 32],
    /// Ref name (e.g., "refs/heads/main").
    pub ref_name: String,
    /// Who triggered the build.
    pub triggered_by: String,
    /// Environment variables to inject.
    pub env: HashMap<String, String>,
}

impl PipelineContext {
    /// Get the short commit hash (first 8 hex chars).
    pub fn short_hash(&self) -> String {
        hex::encode(&self.commit_hash[..4])
    }
}

/// Status of a pipeline run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStatus {
    /// Pipeline is pending execution.
    Pending,
    /// Pipeline is currently running.
    Running,
    /// Pipeline completed successfully.
    Success,
    /// Pipeline failed.
    Failed,
    /// Pipeline was cancelled.
    Cancelled,
}

impl PipelineStatus {
    /// Check if the status is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Failed | Self::Cancelled)
    }
}

/// Status of a stage within a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageStatus {
    /// Stage name.
    pub name: String,
    /// Stage status.
    pub status: PipelineStatus,
    /// When the stage started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the stage completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Job statuses within this stage.
    pub jobs: HashMap<String, JobStatus>,
}

/// Status of a job within a stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    /// Job ID in aspen-jobs.
    pub job_id: Option<JobId>,
    /// Job status.
    pub status: PipelineStatus,
    /// When the job started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Job output if completed.
    pub output: Option<serde_json::Value>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// A pipeline run instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    /// Unique run ID.
    pub id: String,
    /// Pipeline name.
    pub pipeline_name: String,
    /// Execution context.
    pub context: PipelineContext,
    /// Overall status.
    pub status: PipelineStatus,
    /// When the run was created.
    pub created_at: DateTime<Utc>,
    /// When the run started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the run completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Status of each stage.
    pub stages: Vec<StageStatus>,
    /// Underlying workflow ID in aspen-jobs.
    pub workflow_id: Option<String>,
}

impl PipelineRun {
    /// Create a new pending pipeline run.
    pub fn new(pipeline_name: String, context: PipelineContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipeline_name,
            context,
            status: PipelineStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            stages: Vec::new(),
            workflow_id: None,
        }
    }
}

/// Pipeline orchestrator that manages CI pipeline execution.
///
/// Converts `PipelineConfig` into `WorkflowDefinition` and executes
/// pipelines using the aspen-jobs system.
#[allow(dead_code)] // Fields reserved for future artifact and job tracking features
pub struct PipelineOrchestrator<S: KeyValueStore + ?Sized> {
    /// Configuration.
    config: PipelineOrchestratorConfig,
    /// Workflow manager for job execution.
    workflow_manager: Arc<WorkflowManager<S>>,
    /// Job manager for submitting individual jobs.
    job_manager: Arc<JobManager<S>>,
    /// Optional blob store for artifacts.
    blob_store: Option<Arc<dyn BlobStore>>,
    /// Active pipeline runs (run_id -> PipelineRun).
    active_runs: RwLock<HashMap<String, PipelineRun>>,
    /// Runs per repository (repo_id -> count).
    runs_per_repo: RwLock<HashMap<RepoId, usize>>,
}

impl<S: KeyValueStore + ?Sized + 'static> PipelineOrchestrator<S> {
    /// Create a new pipeline orchestrator.
    pub fn new(
        config: PipelineOrchestratorConfig,
        workflow_manager: Arc<WorkflowManager<S>>,
        job_manager: Arc<JobManager<S>>,
        blob_store: Option<Arc<dyn BlobStore>>,
    ) -> Self {
        Self {
            config,
            workflow_manager,
            job_manager,
            blob_store,
            active_runs: RwLock::new(HashMap::new()),
            runs_per_repo: RwLock::new(HashMap::new()),
        }
    }

    /// Execute a pipeline with the given configuration and context.
    ///
    /// # Arguments
    ///
    /// * `pipeline_config` - The pipeline configuration
    /// * `context` - Execution context (repo, commit, env)
    ///
    /// # Returns
    ///
    /// The started pipeline run.
    pub async fn execute(&self, pipeline_config: PipelineConfig, context: PipelineContext) -> Result<PipelineRun> {
        // Validate job count
        let total_jobs: usize = pipeline_config.stages.iter().map(|s| s.jobs.len()).sum();
        if total_jobs > MAX_JOBS_PER_PIPELINE {
            return Err(CiError::InvalidConfig {
                reason: format!("Pipeline has {} jobs, maximum is {}", total_jobs, MAX_JOBS_PER_PIPELINE),
            });
        }

        // Check concurrent run limits
        self.check_run_limits(&context.repo_id).await?;

        // Create pipeline run
        let mut run = PipelineRun::new(pipeline_config.name.clone(), context.clone());

        // Initialize stage statuses
        for stage in &pipeline_config.stages {
            let mut jobs = HashMap::new();
            for job in &stage.jobs {
                jobs.insert(
                    job.name.clone(),
                    JobStatus {
                        job_id: None,
                        status: PipelineStatus::Pending,
                        started_at: None,
                        completed_at: None,
                        output: None,
                        error: None,
                    },
                );
            }

            run.stages.push(StageStatus {
                name: stage.name.clone(),
                status: PipelineStatus::Pending,
                started_at: None,
                completed_at: None,
                jobs,
            });
        }

        // Convert to workflow definition
        let workflow_def = self.build_workflow_definition(&pipeline_config, &context)?;

        // Build initial workflow data
        let workflow_data = serde_json::json!({
            "run_id": run.id,
            "pipeline_name": pipeline_config.name,
            "repo_id": hex::encode(context.repo_id.0),
            "commit_hash": hex::encode(context.commit_hash),
            "ref_name": context.ref_name,
            "env": context.env,
        });

        info!(
            run_id = %run.id,
            pipeline = %pipeline_config.name,
            repo_id = %context.repo_id.to_hex(),
            "Starting pipeline execution"
        );

        // Start the workflow
        let workflow_id = self.workflow_manager.start_workflow(&workflow_def, workflow_data).await.map_err(|e| {
            CiError::Workflow {
                reason: format!("Failed to start workflow: {}", e),
            }
        })?;

        run.workflow_id = Some(workflow_id.clone());
        run.status = PipelineStatus::Running;
        run.started_at = Some(Utc::now());

        // Track the run
        self.track_run(&run).await;

        info!(
            run_id = %run.id,
            workflow_id = %workflow_id,
            "Pipeline workflow started"
        );

        Ok(run)
    }

    /// Get a pipeline run by ID.
    pub async fn get_run(&self, run_id: &str) -> Option<PipelineRun> {
        self.active_runs.read().await.get(run_id).cloned()
    }

    /// List all active runs for a repository.
    pub async fn list_runs(&self, repo_id: &RepoId) -> Vec<PipelineRun> {
        self.active_runs.read().await.values().filter(|r| &r.context.repo_id == repo_id).cloned().collect()
    }

    /// Cancel a pipeline run.
    pub async fn cancel(&self, run_id: &str) -> Result<()> {
        let mut runs = self.active_runs.write().await;

        if let Some(run) = runs.get_mut(run_id) {
            if run.status.is_terminal() {
                return Err(CiError::InvalidConfig {
                    reason: "Cannot cancel completed pipeline".to_string(),
                });
            }

            run.status = PipelineStatus::Cancelled;
            run.completed_at = Some(Utc::now());

            // TODO: Cancel underlying workflow jobs

            info!(run_id = %run_id, "Pipeline cancelled");
        }

        Ok(())
    }

    /// Build a workflow definition from pipeline configuration.
    fn build_workflow_definition(
        &self,
        config: &PipelineConfig,
        context: &PipelineContext,
    ) -> Result<WorkflowDefinition> {
        let mut steps = HashMap::new();
        let mut terminal_states = std::collections::HashSet::new();

        // Get stages in topological order
        let ordered_stages = config.stages_in_order();

        // Build workflow steps from stages
        for (idx, stage) in ordered_stages.iter().enumerate() {
            let step_name = format!("stage_{}", stage.name);

            // Convert jobs to JobSpecs
            let job_specs: Vec<JobSpec> = stage
                .jobs
                .iter()
                .map(|job| self.job_config_to_spec(job, context, &config.env))
                .collect::<Result<Vec<_>>>()?;

            // Determine transitions
            let mut transitions = Vec::new();

            // Find the next stage (if any)
            let next_stage = ordered_stages.get(idx + 1);

            if let Some(next) = next_stage {
                // Success -> next stage
                transitions.push(WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: format!("stage_{}", next.name),
                });

                // Failure -> failed state (unless allow_failure is set for all jobs)
                let all_allow_failure = stage.jobs.iter().all(|j| j.allow_failure);
                if !all_allow_failure {
                    transitions.push(WorkflowTransition {
                        condition: TransitionCondition::AnyFailed,
                        target: "failed".to_string(),
                    });
                }
            } else {
                // Last stage - success goes to done
                transitions.push(WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: "done".to_string(),
                });

                transitions.push(WorkflowTransition {
                    condition: TransitionCondition::AnyFailed,
                    target: "failed".to_string(),
                });
            }

            let timeout_secs = stage.jobs.iter().map(|j| j.timeout_secs).max().unwrap_or(DEFAULT_STEP_TIMEOUT_SECS);

            steps.insert(
                step_name.clone(),
                WorkflowStep {
                    name: step_name,
                    jobs: job_specs,
                    transitions,
                    parallel: stage.parallel,
                    timeout: Some(Duration::from_secs(timeout_secs)),
                    retry_on_failure: stage.jobs.iter().any(|j| j.retry_count > 0),
                },
            );
        }

        // Add terminal states
        steps.insert(
            "done".to_string(),
            WorkflowStep {
                name: "done".to_string(),
                jobs: vec![],
                transitions: vec![],
                parallel: false,
                timeout: None,
                retry_on_failure: false,
            },
        );
        terminal_states.insert("done".to_string());

        steps.insert(
            "failed".to_string(),
            WorkflowStep {
                name: "failed".to_string(),
                jobs: vec![],
                transitions: vec![],
                parallel: false,
                timeout: None,
                retry_on_failure: false,
            },
        );
        terminal_states.insert("failed".to_string());

        // Initial state is first stage
        let initial_state =
            ordered_stages.first().map(|s| format!("stage_{}", s.name)).unwrap_or_else(|| "done".to_string());

        let timeout = Duration::from_secs(config.timeout_secs);

        Ok(WorkflowDefinition {
            name: config.name.clone(),
            initial_state,
            steps,
            terminal_states,
            timeout: Some(timeout),
        })
    }

    /// Convert a JobConfig to a JobSpec for the job system.
    fn job_config_to_spec(
        &self,
        job: &JobConfig,
        context: &PipelineContext,
        pipeline_env: &HashMap<String, String>,
    ) -> Result<JobSpec> {
        // Merge environment variables: pipeline-level + job-level + CI context
        let mut env = pipeline_env.clone();
        env.extend(job.env.clone());

        // Add CI context variables
        env.insert("CI".to_string(), "true".to_string());
        env.insert("ASPEN_CI".to_string(), "true".to_string());
        env.insert("ASPEN_GIT_COMMIT".to_string(), hex::encode(context.commit_hash));
        env.insert("ASPEN_GIT_REV_SHORT".to_string(), context.short_hash());
        env.insert("ASPEN_GIT_REF".to_string(), context.ref_name.clone());
        env.insert("ASPEN_REPO_ID".to_string(), context.repo_id.to_hex());

        let payload = match job.job_type {
            JobType::Shell => {
                let command = job.command.clone().ok_or_else(|| CiError::InvalidConfig {
                    reason: format!("Shell job '{}' missing command", job.name),
                })?;

                serde_json::json!({
                    "type": "shell",
                    "command": command,
                    "args": job.args,
                    "env": env,
                    "working_dir": job.working_dir,
                    "timeout_secs": job.timeout_secs,
                })
            }

            JobType::Nix => {
                let flake_url = job.flake_url.clone().unwrap_or_else(|| ".".to_string());
                let attribute = job.flake_attr.clone().unwrap_or_default();

                let nix_payload = NixBuildPayload {
                    flake_url,
                    attribute,
                    extra_args: job.args.clone(),
                    working_dir: job.working_dir.as_ref().map(std::path::PathBuf::from),
                    timeout_secs: job.timeout_secs,
                    sandbox: matches!(job.isolation, crate::config::types::IsolationMode::NixSandbox),
                    cache_key: job.cache_key.clone(),
                    artifacts: job.artifacts.clone(),
                };

                serde_json::to_value(&nix_payload).map_err(|e| CiError::InvalidConfig {
                    reason: format!("Failed to serialize Nix payload: {}", e),
                })?
            }

            JobType::Vm => {
                // VM jobs use Hyperlight or similar
                serde_json::json!({
                    "type": "vm",
                    "binary_hash": job.binary_hash,
                    "flake_attr": job.flake_attr,
                    "timeout_secs": job.timeout_secs,
                    "env": env,
                })
            }
        };

        let job_type = match job.job_type {
            JobType::Shell => "shell_command",
            JobType::Nix => "ci_nix_build",
            JobType::Vm => "vm_job",
        };

        let retry_policy = if job.retry_count > 0 {
            aspen_jobs::RetryPolicy::exponential(job.retry_count)
        } else {
            aspen_jobs::RetryPolicy::none()
        };

        let spec = JobSpec::new(job_type)
            .payload(payload)
            .map_err(|e| CiError::InvalidConfig {
                reason: format!("Failed to serialize job payload: {}", e),
            })?
            .priority(crate::config::types::Priority::default().into())
            .timeout(Duration::from_secs(job.timeout_secs))
            .retry_policy(retry_policy);

        Ok(spec)
    }

    /// Check and update run limits.
    async fn check_run_limits(&self, repo_id: &RepoId) -> Result<()> {
        let active_count = self.active_runs.read().await.len();
        if active_count >= self.config.max_total_runs {
            return Err(CiError::InvalidConfig {
                reason: format!("Maximum total concurrent runs ({}) reached", self.config.max_total_runs),
            });
        }

        let repo_count = self.runs_per_repo.read().await.get(repo_id).copied().unwrap_or(0);

        if repo_count >= self.config.max_runs_per_repo {
            return Err(CiError::InvalidConfig {
                reason: format!("Maximum concurrent runs per repository ({}) reached", self.config.max_runs_per_repo),
            });
        }

        Ok(())
    }

    /// Track a new pipeline run.
    async fn track_run(&self, run: &PipelineRun) {
        self.active_runs.write().await.insert(run.id.clone(), run.clone());

        let mut repo_runs = self.runs_per_repo.write().await;
        *repo_runs.entry(run.context.repo_id).or_insert(0) += 1;
    }

    /// Mark a pipeline run as completed and clean up.
    pub async fn complete_run(&self, run_id: &str, status: PipelineStatus) {
        let mut runs = self.active_runs.write().await;

        if let Some(run) = runs.get_mut(run_id) {
            let repo_id = run.context.repo_id;
            run.status = status;
            run.completed_at = Some(Utc::now());

            // Update repo run count
            if let Some(count) = self.runs_per_repo.write().await.get_mut(&repo_id) {
                *count = count.saturating_sub(1);
            }

            info!(
                run_id = %run_id,
                status = ?status,
                "Pipeline run completed"
            );
        }
    }

    /// Get the count of active runs.
    pub async fn active_run_count(&self) -> usize {
        self.active_runs.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_run_new() {
        let context = PipelineContext {
            repo_id: RepoId::from_hash(blake3::hash(b"test")),
            commit_hash: [1u8; 32],
            ref_name: "refs/heads/main".to_string(),
            triggered_by: "test".to_string(),
            env: HashMap::new(),
        };

        let run = PipelineRun::new("test-pipeline".to_string(), context);

        assert_eq!(run.pipeline_name, "test-pipeline");
        assert_eq!(run.status, PipelineStatus::Pending);
        assert!(run.workflow_id.is_none());
    }

    #[test]
    fn test_pipeline_status_is_terminal() {
        assert!(!PipelineStatus::Pending.is_terminal());
        assert!(!PipelineStatus::Running.is_terminal());
        assert!(PipelineStatus::Success.is_terminal());
        assert!(PipelineStatus::Failed.is_terminal());
        assert!(PipelineStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_pipeline_context_short_hash() {
        let context = PipelineContext {
            repo_id: RepoId::from_hash(blake3::hash(b"test")),
            commit_hash: [
                0xab, 0xcd, 0xef, 0x12, // first 4 bytes -> "abcdef12"
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            ref_name: "refs/heads/main".to_string(),
            triggered_by: "test".to_string(),
            env: HashMap::new(),
        };

        assert_eq!(context.short_hash(), "abcdef12");
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = PipelineOrchestratorConfig::default();
        assert_eq!(config.max_runs_per_repo, MAX_CONCURRENT_RUNS_PER_REPO);
        assert_eq!(config.max_total_runs, MAX_TOTAL_CONCURRENT_RUNS);
    }
}
