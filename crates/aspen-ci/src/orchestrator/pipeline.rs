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
use aspen_core::ReadConsistency;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_forge::identity::RepoId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::JobStatus as AspenJobStatus;
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
/// Maximum number of runs to list from KV store.
const MAX_LIST_RUNS: u32 = 500;

// KV storage prefixes for CI data persistence
/// KV key prefix for pipeline runs.
/// Key format: `{KV_PREFIX_CI_RUNS}{run_id}`
/// Value: JSON-serialized `PipelineRun`
const KV_PREFIX_CI_RUNS: &str = "_ci:runs:";

/// KV key prefix for repository run index.
/// Key format: `{KV_PREFIX_CI_RUNS_BY_REPO}{repo_id_hex}:{created_at_ms}:{run_id}`
/// Value: run_id string (for efficient repo-based listing)
const KV_PREFIX_CI_RUNS_BY_REPO: &str = "_ci:runs:by-repo:";

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
///
/// # Persistence
///
/// Pipeline runs are persisted to the KV store using two key patterns:
/// - `_ci:runs:{run_id}` - Full run data (JSON serialized)
/// - `_ci:runs:by-repo:{repo_id}:{created_at_ms}:{run_id}` - Index for per-repo listing
///
/// Active runs are also cached in memory for fast access.
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
    /// KV store for persistent run storage.
    kv_store: Arc<S>,
    /// Active pipeline runs (run_id -> PipelineRun) - in-memory cache.
    active_runs: RwLock<HashMap<String, PipelineRun>>,
    /// Runs per repository (repo_id -> count) - in-memory counter.
    runs_per_repo: RwLock<HashMap<RepoId, usize>>,
}

impl<S: KeyValueStore + ?Sized + 'static> PipelineOrchestrator<S> {
    /// Create a new pipeline orchestrator.
    ///
    /// # Arguments
    ///
    /// * `config` - Orchestrator configuration (limits, timeouts)
    /// * `workflow_manager` - Manager for aspen-jobs workflows
    /// * `job_manager` - Manager for individual job submission
    /// * `blob_store` - Optional blob store for artifacts
    /// * `kv_store` - KV store for persistent run storage
    pub fn new(
        config: PipelineOrchestratorConfig,
        workflow_manager: Arc<WorkflowManager<S>>,
        job_manager: Arc<JobManager<S>>,
        blob_store: Option<Arc<dyn BlobStore>>,
        kv_store: Arc<S>,
    ) -> Self {
        Self {
            config,
            workflow_manager,
            job_manager,
            blob_store,
            kv_store,
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
    ///
    /// First checks the in-memory cache, then falls back to KV store.
    /// If the run is still running, syncs status from the underlying workflow.
    pub async fn get_run(&self, run_id: &str) -> Option<PipelineRun> {
        // Check in-memory cache first
        let run = if let Some(run) = self.active_runs.read().await.get(run_id).cloned() {
            Some(run)
        } else {
            // Fall back to KV store
            self.load_run_from_kv(run_id).await
        };

        // Sync status from workflow if run is still in progress
        if let Some(run) = run {
            if !run.status.is_terminal() {
                if let Some(synced) = self.sync_run_status(&run).await {
                    return Some(synced);
                }
            }
            return Some(run);
        }

        None
    }

    /// Sync pipeline run status from the underlying workflow state.
    ///
    /// If the workflow has completed (reached terminal state), updates the
    /// pipeline run status accordingly and persists the change.
    ///
    /// Since the workflow manager's state may not be updated when jobs complete
    /// (there's no callback from JobManager.mark_completed to WorkflowManager),
    /// we also check the actual job statuses directly.
    async fn sync_run_status(&self, run: &PipelineRun) -> Option<PipelineRun> {
        let workflow_id = run.workflow_id.as_ref()?;

        // Get workflow state
        let workflow_state = match self.workflow_manager.get_workflow_state(workflow_id).await {
            Ok(state) => state,
            Err(e) => {
                debug!(
                    run_id = %run.id,
                    workflow_id = %workflow_id,
                    error = %e,
                    "Failed to get workflow state for status sync"
                );
                return None;
            }
        };

        // Collect all job IDs from the workflow
        let all_job_ids: Vec<_> = workflow_state
            .active_jobs
            .iter()
            .chain(workflow_state.completed_jobs.iter())
            .chain(workflow_state.failed_jobs.iter())
            .collect();

        // Build a map of job_name -> (job_id, status) by looking up each job's payload and status
        let mut job_name_to_info: HashMap<String, (JobId, PipelineStatus)> = HashMap::new();
        for job_id in &all_job_ids {
            if let Ok(Some(job)) = self.job_manager.get_job(job_id).await {
                // Extract job_name from payload
                if let Some(name) = job.spec.payload.get("job_name").and_then(|v| v.as_str()) {
                    // Convert job status to pipeline status
                    let pipeline_status = match job.status {
                        AspenJobStatus::Pending | AspenJobStatus::Scheduled => PipelineStatus::Pending,
                        AspenJobStatus::Running | AspenJobStatus::Retrying => PipelineStatus::Running,
                        AspenJobStatus::Completed => PipelineStatus::Success,
                        AspenJobStatus::Failed | AspenJobStatus::DeadLetter => PipelineStatus::Failed,
                        AspenJobStatus::Cancelled => PipelineStatus::Cancelled,
                    };
                    job_name_to_info.insert(name.to_string(), ((*job_id).clone(), pipeline_status));
                }
            }
        }

        // Check if workflow reached a terminal state
        let mut new_status = if workflow_state.state == "done" {
            Some(PipelineStatus::Success)
        } else if workflow_state.state == "failed" {
            Some(PipelineStatus::Failed)
        } else if !workflow_state.failed_jobs.is_empty() {
            // Any failed job means pipeline failed
            Some(PipelineStatus::Failed)
        } else {
            None
        };

        // If workflow state doesn't show completion, check actual job statuses
        // This handles the case where JobManager.mark_completed doesn't update workflow state
        if new_status.is_none() && !workflow_state.active_jobs.is_empty() {
            let mut all_completed = true;
            let mut any_failed = false;

            for job_id in &workflow_state.active_jobs {
                match self.job_manager.get_job(job_id).await {
                    Ok(Some(job)) => match job.status {
                        AspenJobStatus::Completed => {
                            // Job completed successfully
                        }
                        AspenJobStatus::Failed => {
                            any_failed = true;
                            all_completed = true; // Failed is also "done"
                        }
                        AspenJobStatus::Pending
                        | AspenJobStatus::Running
                        | AspenJobStatus::Retrying
                        | AspenJobStatus::Scheduled => {
                            all_completed = false;
                        }
                        AspenJobStatus::Cancelled => {
                            any_failed = true;
                            all_completed = true;
                        }
                        AspenJobStatus::DeadLetter => {
                            any_failed = true;
                            all_completed = true;
                        }
                    },
                    Ok(None) => {
                        // Job not found - treat as completed (possibly cleaned up)
                        debug!(
                            run_id = %run.id,
                            job_id = %job_id,
                            "Job not found during status sync, treating as completed"
                        );
                    }
                    Err(e) => {
                        debug!(
                            run_id = %run.id,
                            job_id = %job_id,
                            error = %e,
                            "Failed to get job status during sync"
                        );
                        all_completed = false;
                    }
                }
            }

            if all_completed {
                new_status = Some(if any_failed {
                    PipelineStatus::Failed
                } else {
                    PipelineStatus::Success
                });
            }
        }

        // Always update job IDs and statuses in stages, even if pipeline status hasn't changed
        let mut updated_run = run.clone();
        let mut any_job_updated = false;

        for stage in &mut updated_run.stages {
            for (job_name, job_status) in &mut stage.jobs {
                if let Some((job_id, status)) = job_name_to_info.get(job_name) {
                    // Update job ID if not set
                    if job_status.job_id.is_none() {
                        job_status.job_id = Some(job_id.clone());
                        any_job_updated = true;
                    }
                    // Always update job status to reflect actual state
                    if job_status.status != *status {
                        job_status.status = *status;
                        any_job_updated = true;
                    }
                }
            }
        }

        if let Some(status) = new_status {
            updated_run.status = status;
            updated_run.completed_at = Some(Utc::now());

            // Update in-memory cache
            self.active_runs.write().await.insert(run.id.clone(), updated_run.clone());

            // Persist to KV store
            if let Err(e) = self.persist_run(&updated_run).await {
                warn!(
                    run_id = %run.id,
                    error = %e,
                    "Failed to persist synced run status to KV store"
                );
            }

            // Update repo run count if completed
            if status.is_terminal() {
                if let Some(count) = self.runs_per_repo.write().await.get_mut(&run.context.repo_id) {
                    *count = count.saturating_sub(1);
                }
            }

            info!(
                run_id = %run.id,
                workflow_id = %workflow_id,
                status = ?status,
                "Pipeline run status synced from workflow"
            );

            return Some(updated_run);
        }

        // If job info was updated but no pipeline status change, still return updated run
        if any_job_updated {
            // Update in-memory cache with new job info
            self.active_runs.write().await.insert(run.id.clone(), updated_run.clone());

            // Persist to KV store
            if let Err(e) = self.persist_run(&updated_run).await {
                debug!(
                    run_id = %run.id,
                    error = %e,
                    "Failed to persist job info update to KV store"
                );
            }

            return Some(updated_run);
        }

        None
    }

    /// List all active runs for a repository (from in-memory cache only).
    ///
    /// For listing all runs including completed ones, use `list_all_runs`.
    /// Syncs status from workflow for in-progress runs.
    pub async fn list_runs(&self, repo_id: &RepoId) -> Vec<PipelineRun> {
        let runs: Vec<_> =
            self.active_runs.read().await.values().filter(|r| &r.context.repo_id == repo_id).cloned().collect();

        // Sync status for in-progress runs
        let mut result = Vec::with_capacity(runs.len());
        for run in runs {
            if !run.status.is_terminal() {
                if let Some(synced) = self.sync_run_status(&run).await {
                    result.push(synced);
                    continue;
                }
            }
            result.push(run);
        }
        result
    }

    /// List all runs (active and completed) with optional filtering.
    ///
    /// Queries the KV store for persisted runs.
    ///
    /// # Arguments
    ///
    /// * `repo_id` - Optional filter by repository
    /// * `status` - Optional filter by status string (pending, running, success, failed, cancelled)
    /// * `limit` - Maximum number of runs to return (default: 50, max: 500)
    pub async fn list_all_runs(&self, repo_id: Option<&RepoId>, status: Option<&str>, limit: u32) -> Vec<PipelineRun> {
        let limit = limit.min(MAX_LIST_RUNS);

        // Determine scan prefix based on repo_id filter
        let (prefix, use_index) = if let Some(repo) = repo_id {
            // Use the by-repo index for efficient filtering
            (format!("{}{}", KV_PREFIX_CI_RUNS_BY_REPO, repo.to_hex()), true)
        } else {
            // Scan all runs
            (KV_PREFIX_CI_RUNS.to_string(), false)
        };

        let scan_request = ScanRequest {
            prefix,
            limit: Some(limit * 2), // Fetch extra for filtering
            continuation_token: None,
        };

        let scan_result = match self.kv_store.scan(scan_request).await {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, "Failed to scan CI runs from KV store");
                return Vec::new();
            }
        };

        let mut runs = Vec::new();

        for entry in scan_result.entries {
            // If using the index, we need to fetch the actual run by ID
            let mut run = if use_index {
                // Index entries store the run_id as the value
                let run_id = entry.value.clone();
                match self.load_run_from_kv(&run_id).await {
                    Some(r) => r,
                    None => continue, // Stale index entry
                }
            } else {
                // Direct run entries - parse the JSON from string
                match serde_json::from_str::<PipelineRun>(&entry.value) {
                    Ok(r) => r,
                    Err(e) => {
                        debug!(key = %entry.key, error = %e, "Failed to parse pipeline run");
                        continue;
                    }
                }
            };

            // Sync status from workflow for in-progress runs
            if !run.status.is_terminal() {
                if let Some(synced) = self.sync_run_status(&run).await {
                    run = synced;
                }
            }

            // Apply status filter if specified
            if let Some(status_filter) = status {
                let run_status = match run.status {
                    PipelineStatus::Pending => "pending",
                    PipelineStatus::Running => "running",
                    PipelineStatus::Success => "success",
                    PipelineStatus::Failed => "failed",
                    PipelineStatus::Cancelled => "cancelled",
                };
                if run_status != status_filter {
                    continue;
                }
            }

            runs.push(run);

            if runs.len() >= limit as usize {
                break;
            }
        }

        // Sort by created_at descending (newest first)
        runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        runs
    }

    /// Cancel a pipeline run.
    ///
    /// Updates both in-memory cache and KV store.
    pub async fn cancel(&self, run_id: &str) -> Result<()> {
        // Get the run (from cache or KV)
        let run = match self.get_run(run_id).await {
            Some(r) => r,
            None => {
                return Err(CiError::InvalidConfig {
                    reason: format!("Pipeline run {} not found", run_id),
                });
            }
        };

        if run.status.is_terminal() {
            return Err(CiError::InvalidConfig {
                reason: "Cannot cancel completed pipeline".to_string(),
            });
        }

        // Update the run
        let mut updated_run = run;
        updated_run.status = PipelineStatus::Cancelled;
        updated_run.completed_at = Some(Utc::now());

        // Update in-memory cache
        self.active_runs.write().await.insert(run_id.to_string(), updated_run.clone());

        // Persist to KV store
        if let Err(e) = self.persist_run(&updated_run).await {
            warn!(run_id = %run_id, error = %e, "Failed to persist cancelled run to KV store");
        }

        // TODO: Cancel underlying workflow jobs

        info!(run_id = %run_id, "Pipeline cancelled");

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

                // Determine if we need to wrap command in sh -c
                // This is needed when:
                // 1. Command contains shell metacharacters (spaces, quotes, pipes, etc.)
                // 2. And no separate args are provided (command is a full shell expression)
                let needs_shell_wrap = job.args.is_empty()
                    && (command.contains(' ')
                        || command.contains('\'')
                        || command.contains('"')
                        || command.contains('|')
                        || command.contains('>')
                        || command.contains('<')
                        || command.contains('&')
                        || command.contains(';')
                        || command.contains('$'));

                let (final_command, final_args) = if needs_shell_wrap {
                    // Wrap in sh -c for shell interpretation
                    ("sh".to_string(), vec!["-c".to_string(), command])
                } else {
                    // Use command directly with provided args
                    (command, job.args.clone())
                };

                serde_json::json!({
                    "type": "shell",
                    "job_name": job.name,
                    "command": final_command,
                    "args": final_args,
                    "env": env,
                    "working_dir": job.working_dir,
                    "timeout_secs": job.timeout_secs,
                })
            }

            JobType::Nix => {
                let flake_url = job.flake_url.clone().unwrap_or_else(|| ".".to_string());
                let attribute = job.flake_attr.clone().unwrap_or_default();

                let nix_payload = NixBuildPayload {
                    job_name: Some(job.name.clone()),
                    flake_url,
                    attribute,
                    extra_args: job.args.clone(),
                    working_dir: job.working_dir.as_ref().map(std::path::PathBuf::from),
                    timeout_secs: job.timeout_secs,
                    sandbox: matches!(job.isolation, crate::config::types::IsolationMode::NixSandbox),
                    cache_key: job.cache_key.clone(),
                    artifacts: job.artifacts.clone(),
                    upload_result: true, // Always upload results to blob store
                };

                serde_json::to_value(&nix_payload).map_err(|e| CiError::InvalidConfig {
                    reason: format!("Failed to serialize Nix payload: {}", e),
                })?
            }

            JobType::Vm => {
                // VM jobs use Hyperlight or similar
                serde_json::json!({
                    "type": "vm",
                    "job_name": job.name,
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
    ///
    /// Updates both in-memory cache and persists to KV store.
    async fn track_run(&self, run: &PipelineRun) {
        // Add to in-memory cache
        self.active_runs.write().await.insert(run.id.clone(), run.clone());

        let mut repo_runs = self.runs_per_repo.write().await;
        *repo_runs.entry(run.context.repo_id).or_insert(0) += 1;

        // Persist to KV store
        if let Err(e) = self.persist_run(run).await {
            warn!(run_id = %run.id, error = %e, "Failed to persist pipeline run to KV store");
        }
    }

    /// Mark a pipeline run as completed and clean up.
    ///
    /// Updates both in-memory cache and KV store.
    pub async fn complete_run(&self, run_id: &str, status: PipelineStatus) {
        let mut runs = self.active_runs.write().await;

        if let Some(run) = runs.get_mut(run_id) {
            let repo_id = run.context.repo_id;
            run.status = status;
            run.completed_at = Some(Utc::now());

            // Persist updated run to KV store
            if let Err(e) = self.persist_run(run).await {
                warn!(run_id = %run_id, error = %e, "Failed to persist completed run to KV store");
            }

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

    // ========================================================================
    // KV Persistence Helpers
    // ========================================================================

    /// Persist a pipeline run to the KV store.
    ///
    /// Stores:
    /// - The full run data at `_ci:runs:{run_id}`
    /// - A repo index entry at `_ci:runs:by-repo:{repo_id}:{created_at_ms}:{run_id}`
    async fn persist_run(&self, run: &PipelineRun) -> std::result::Result<(), CiError> {
        // Serialize the run to JSON string
        let run_json = serde_json::to_string(run).map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to serialize pipeline run: {}", e),
        })?;

        // Build keys
        let run_key = format!("{}{}", KV_PREFIX_CI_RUNS, run.id);
        let created_at_ms = run.created_at.timestamp_millis() as u64;
        let index_key =
            format!("{}{}:{}:{}", KV_PREFIX_CI_RUNS_BY_REPO, run.context.repo_id.to_hex(), created_at_ms, run.id);

        // Write both keys atomically using SetMulti
        // Values are String in the KV store
        let write_request = WriteRequest {
            command: WriteCommand::SetMulti {
                pairs: vec![(run_key.clone(), run_json), (index_key.clone(), run.id.clone())],
            },
        };

        self.kv_store.write(write_request).await.map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to write pipeline run to KV store: {}", e),
        })?;

        debug!(
            run_id = %run.id,
            run_key = %run_key,
            index_key = %index_key,
            "Persisted pipeline run to KV store"
        );

        Ok(())
    }

    /// Load a pipeline run from the KV store.
    async fn load_run_from_kv(&self, run_id: &str) -> Option<PipelineRun> {
        let key = format!("{}{}", KV_PREFIX_CI_RUNS, run_id);

        let read_request = ReadRequest {
            key: key.clone(),
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv_store.read(read_request).await {
            Ok(result) => {
                // ReadResult has a `kv` field with Option<KeyValueWithRevision>
                if let Some(kv_entry) = result.kv {
                    match serde_json::from_str::<PipelineRun>(&kv_entry.value) {
                        Ok(run) => Some(run),
                        Err(e) => {
                            debug!(key = %key, error = %e, "Failed to parse pipeline run from KV store");
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Err(e) => {
                debug!(key = %key, error = %e, "Failed to read pipeline run from KV store");
                None
            }
        }
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
