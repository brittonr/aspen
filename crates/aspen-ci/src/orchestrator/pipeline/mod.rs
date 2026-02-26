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
//! │ stages[]   │──────>│ Convert to         │──────>│ Workflow    │
//! │ jobs[]     │       │ WorkflowDefinition │       │ Manager     │
//! │ depends_on │       └────────────────────┘       └─────────────┘
//! └────────────┘                │                          │
//!                               │                          │
//!                    ┌──────────v──────────┐               │
//!                    │    PipelineRun      │<──────────────┘
//!                    │ - status            │   (status updates)
//!                    │ - stage_statuses    │
//!                    │ - job_results       │
//!                    └─────────────────────┘
//! ```

mod executor;
mod persistence;
mod status;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_core::ScanRequest;
use aspen_forge::identity::RepoId;
use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use aspen_jobs::WorkflowManager;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::config::types::PipelineConfig;
use crate::error::CiError;
use crate::error::Result;

// Tiger Style: Bounded resources
/// Maximum concurrent pipeline runs per repository.
const MAX_CONCURRENT_RUNS_PER_REPO: u32 = 5;
/// Maximum total concurrent pipeline runs.
const MAX_TOTAL_CONCURRENT_RUNS: u32 = 50;
/// Maximum jobs per pipeline.
const MAX_JOBS_PER_PIPELINE: u32 = 100;
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
    pub max_runs_per_repo: u32,
    /// Maximum total concurrent runs.
    pub max_total_runs: u32,
    /// Default step timeout.
    pub default_step_timeout: Duration,
    /// Avoid scheduling CI jobs on the Raft leader node.
    ///
    /// When true, CI jobs will be assigned the `AvoidLeader` affinity to
    /// prevent resource contention with Raft consensus operations.
    pub avoid_leader: bool,
    /// Enable resource isolation using cgroups.
    ///
    /// When true, CI job processes will be placed in cgroups with memory
    /// and CPU limits to prevent resource exhaustion.
    pub resource_isolation: bool,
    /// Maximum memory per CI job in bytes (used with resource_isolation).
    pub max_job_memory_bytes: u64,
}

impl Default for PipelineOrchestratorConfig {
    fn default() -> Self {
        Self {
            max_runs_per_repo: MAX_CONCURRENT_RUNS_PER_REPO,
            max_total_runs: MAX_TOTAL_CONCURRENT_RUNS,
            default_step_timeout: Duration::from_secs(DEFAULT_STEP_TIMEOUT_SECS),
            avoid_leader: true,       // Default to avoiding leader for stability
            resource_isolation: true, // Default to cgroup isolation
            max_job_memory_bytes: aspen_core::MAX_CI_JOB_MEMORY_BYTES,
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
    /// Working directory for jobs (checkout directory).
    ///
    /// If set, all jobs in the pipeline will use this as their working directory.
    /// This is typically the path where the repository was checked out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_dir: Option<std::path::PathBuf>,

    /// Source archive hash for VM jobs.
    ///
    /// When set, VM jobs will download the checkout from blob store instead of
    /// trying to access `checkout_dir` directly. This is required for VM isolation
    /// since VMs cannot access the host's checkout directory directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
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
    /// Pipeline run created, awaiting checkout.
    Initializing,
    /// Repository checkout in progress.
    CheckingOut,
    /// Checkout failed - see error_message for details.
    CheckoutFailed,
    /// Pipeline is pending execution (checkout complete, workflow pending).
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
        matches!(self, Self::Success | Self::Failed | Self::Cancelled | Self::CheckoutFailed)
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
    /// Error message if pipeline failed during initialization or checkout.
    /// This field stores actionable error context for debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl PipelineRun {
    /// Create a new pipeline run in Initializing state.
    ///
    /// The run starts in `Initializing` status to indicate it has been created
    /// but checkout hasn't started yet. This ensures the run is persisted early
    /// so it can be queried even if checkout fails.
    pub fn new(pipeline_name: String, context: PipelineContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipeline_name,
            context,
            status: PipelineStatus::Initializing,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            stages: Vec::new(),
            workflow_id: None,
            error_message: None,
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

    /// Get the blob store if configured.
    ///
    /// Used by adapters to create source archives for VM jobs.
    pub fn blob_store(&self) -> Option<Arc<dyn BlobStore>> {
        self.blob_store.clone()
    }

    /// Initialize the orchestrator by setting up job completion callbacks.
    ///
    /// This must be called before executing pipelines to enable stage transitions.
    pub async fn init(self: &Arc<Self>) {
        let workflow_manager = self.workflow_manager.clone();
        let job_manager = self.job_manager.clone();

        // Set up callback that triggers workflow transitions when jobs complete
        let callback: aspen_jobs::JobCompletionCallback = Arc::new(move |job_id, _result| {
            let wm = workflow_manager.clone();
            let jm = job_manager.clone();
            let jid = job_id.clone();

            Box::pin(async move {
                // Get the job to extract workflow_id from payload
                if let Ok(Some(job)) = jm.get_job(&jid).await {
                    if let Some(workflow_id) = job.spec.payload.get("__workflow_id").and_then(|v| v.as_str()) {
                        // Get workflow definition and process completion
                        if let Some(definition) = wm.get_definition(workflow_id).await {
                            if let Err(e) = wm.process_job_completion(workflow_id, &jid, &definition).await {
                                tracing::warn!(
                                    job_id = %jid,
                                    workflow_id = %workflow_id,
                                    error = %e,
                                    "Failed to process job completion for workflow"
                                );
                            }
                        }
                    }
                }
            })
        });

        self.job_manager.set_completion_callback(callback).await;
    }

    /// Execute a pipeline with the given configuration and context.
    pub async fn execute(&self, pipeline_config: PipelineConfig, context: PipelineContext) -> Result<PipelineRun> {
        // Validate job count
        let total_jobs: usize = pipeline_config.stages.iter().map(|s| s.jobs.len()).sum();
        if total_jobs as u32 > MAX_JOBS_PER_PIPELINE {
            return Err(CiError::InvalidConfig {
                reason: format!("Pipeline has {} jobs, maximum is {}", total_jobs, MAX_JOBS_PER_PIPELINE),
            });
        }

        // Check concurrent run limits
        self.check_run_limits(&context.repo_id).await.map_err(|e| CiError::InvalidConfig {
            reason: format!("run limit check failed for repo {}: {}", context.repo_id.to_hex(), e),
        })?;

        // Create pipeline run
        let mut run = PipelineRun::new(pipeline_config.name.clone(), context.clone());

        // Initialize stage statuses
        init_stage_statuses(&mut run, &pipeline_config);

        // Convert to workflow definition
        let workflow_def = self.build_workflow_definition(&pipeline_config, &context)?;

        // Build initial workflow data
        let workflow_data = build_workflow_data(&run, &pipeline_config, &context);

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
    pub async fn list_all_runs(&self, repo_id: Option<&RepoId>, status: Option<&str>, limit: u32) -> Vec<PipelineRun> {
        let limit = limit.min(MAX_LIST_RUNS);

        // Determine scan prefix based on repo_id filter
        let (prefix, use_index) = if let Some(repo) = repo_id {
            (format!("{}{}", KV_PREFIX_CI_RUNS_BY_REPO, repo.to_hex()), true)
        } else {
            (KV_PREFIX_CI_RUNS.to_string(), false)
        };

        let scan_request = ScanRequest {
            prefix,
            limit_results: Some(limit * 2), // Fetch extra for filtering
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
            let run = self.load_run_from_scan_entry(&entry, use_index).await;
            let mut run = match run {
                Some(r) => r,
                None => continue,
            };

            // Sync status from workflow for in-progress runs
            if !run.status.is_terminal() {
                if let Some(synced) = self.sync_run_status(&run).await {
                    run = synced;
                }
            }

            // Apply status filter if specified
            if let Some(status_filter) = status {
                if pipeline_status_str(run.status) != status_filter {
                    continue;
                }
            }

            runs.push(run);

            if runs.len() >= limit as usize {
                break;
            }
        }

        // Sort by created_at descending (newest first)
        runs.sort_by_key(|b| std::cmp::Reverse(b.created_at));

        runs
    }

    /// Load a pipeline run from a KV scan entry.
    async fn load_run_from_scan_entry(
        &self,
        entry: &aspen_core::KeyValueWithRevision,
        use_index: bool,
    ) -> Option<PipelineRun> {
        if use_index {
            // Index entries store the run_id as the value
            let run_id = entry.value.clone();
            self.load_run_from_kv(&run_id).await
        } else {
            // Direct run entries - parse the JSON from string
            match serde_json::from_str::<PipelineRun>(&entry.value) {
                Ok(r) => Some(r),
                Err(e) => {
                    debug!(key = %entry.key, error = %e, "Failed to parse pipeline run");
                    None
                }
            }
        }
    }

    /// Cancel a pipeline run.
    ///
    /// Updates both in-memory cache and KV store.
    pub async fn cancel(&self, run_id: &str) -> Result<()> {
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

        // Cancel underlying workflow and all its active jobs
        self.cancel_workflow(&updated_run, run_id).await;

        info!(run_id = %run_id, "Pipeline cancelled");

        // Clean up checkout directory for cancelled pipelines
        if let Some(ref checkout_dir) = updated_run.context.checkout_dir {
            if let Err(e) = crate::checkout::cleanup_checkout(checkout_dir).await {
                warn!(
                    run_id = %run_id,
                    checkout_dir = %checkout_dir.display(),
                    error = %e,
                    "Failed to cleanup checkout directory after cancellation (non-fatal)"
                );
            }
        }

        Ok(())
    }

    /// Cancel the underlying workflow for a run.
    async fn cancel_workflow(&self, run: &PipelineRun, run_id: &str) {
        if let Some(workflow_id) = &run.workflow_id {
            match self.workflow_manager.cancel_workflow(workflow_id).await {
                Ok(cancelled_jobs) => {
                    info!(
                        run_id = %run_id,
                        workflow_id = %workflow_id,
                        cancelled_jobs = cancelled_jobs.len(),
                        "Cancelled workflow and active jobs"
                    );
                }
                Err(e) => {
                    // Workflow may not exist if pipeline failed early or was never started
                    warn!(
                        run_id = %run_id,
                        workflow_id = %workflow_id,
                        error = %e,
                        "Failed to cancel workflow (may have already completed)"
                    );
                }
            }
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

    /// Create and persist an early pipeline run before checkout begins.
    ///
    /// This ensures the run is queryable even if checkout fails.
    /// The run starts in `Initializing` status.
    pub async fn create_early_run(&self, pipeline_name: String, context: PipelineContext) -> Result<PipelineRun> {
        // Check concurrent run limits first
        self.check_run_limits(&context.repo_id).await.map_err(|e| CiError::InvalidConfig {
            reason: format!("run limit check failed for repo {}: {}", context.repo_id.to_hex(), e),
        })?;

        // Create the run in Initializing state
        let run = PipelineRun::new(pipeline_name, context);

        // Persist immediately so it's queryable
        self.track_run(&run).await;

        info!(
            run_id = %run.id,
            pipeline = %run.pipeline_name,
            repo_id = %run.context.repo_id.to_hex(),
            "Created early pipeline run (pre-checkout)"
        );

        Ok(run)
    }

    /// Update an existing run's status and optionally set an error message.
    ///
    /// This is used to update runs that failed during checkout or initialization.
    pub async fn update_run_status(
        &self,
        run_id: &str,
        status: PipelineStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut runs = self.active_runs.write().await;

        if let Some(run) = runs.get_mut(run_id) {
            run.status = status;
            run.error_message = error_message.clone();

            if status.is_terminal() {
                run.completed_at = Some(Utc::now());

                // Update repo run count
                let repo_id = run.context.repo_id;
                if let Some(count) = self.runs_per_repo.write().await.get_mut(&repo_id) {
                    *count = count.saturating_sub(1);
                }
            }

            // Persist the update
            if let Err(e) = self.persist_run(run).await {
                warn!(run_id = %run_id, error = %e, "Failed to persist run status update");
            }

            info!(
                run_id = %run_id,
                status = ?status,
                has_error = error_message.is_some(),
                "Updated pipeline run status"
            );

            Ok(())
        } else {
            // Try to load from KV store
            drop(runs);
            self.update_run_status_from_kv(run_id, status, error_message).await
        }
    }

    /// Update run status when the run is only in KV store, not in-memory cache.
    async fn update_run_status_from_kv(
        &self,
        run_id: &str,
        status: PipelineStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        if let Some(mut run) = self.load_run_from_kv(run_id).await {
            run.status = status;
            run.error_message = error_message;

            if status.is_terminal() {
                run.completed_at = Some(Utc::now());
            }

            // Persist the update
            if let Err(e) = self.persist_run(&run).await {
                warn!(run_id = %run_id, error = %e, "Failed to persist run status update");
            }

            // Add to active runs cache
            self.active_runs.write().await.insert(run_id.to_string(), run);

            Ok(())
        } else {
            Err(CiError::InvalidConfig {
                reason: format!("Pipeline run {} not found", run_id),
            })
        }
    }

    /// Update the context of an existing run (e.g., to add checkout_dir after checkout).
    pub async fn update_run_context(&self, run_id: &str, context: PipelineContext) -> Result<()> {
        let mut runs = self.active_runs.write().await;

        if let Some(run) = runs.get_mut(run_id) {
            run.context = context;

            // Persist the update
            if let Err(e) = self.persist_run(run).await {
                warn!(run_id = %run_id, error = %e, "Failed to persist context update");
            }

            Ok(())
        } else {
            Err(CiError::InvalidConfig {
                reason: format!("Pipeline run {} not found", run_id),
            })
        }
    }

    /// Continue execution of a pre-created run.
    ///
    /// This is called after checkout succeeds to actually start the workflow.
    /// The run must already exist and be in a non-terminal state.
    pub async fn execute_existing_run(&self, run_id: &str, pipeline_config: PipelineConfig) -> Result<PipelineRun> {
        // Get the existing run
        let mut run = {
            let runs = self.active_runs.read().await;
            runs.get(run_id).cloned()
        }
        .ok_or_else(|| CiError::InvalidConfig {
            reason: format!("Pipeline run {} not found", run_id),
        })?;

        if run.status.is_terminal() {
            return Err(CiError::InvalidConfig {
                reason: format!("Pipeline run {} is already in terminal state: {:?}", run_id, run.status),
            });
        }

        // Validate job count
        let total_jobs: usize = pipeline_config.stages.iter().map(|s| s.jobs.len()).sum();
        if total_jobs as u32 > MAX_JOBS_PER_PIPELINE {
            return Err(CiError::InvalidConfig {
                reason: format!("Pipeline has {} jobs, maximum is {}", total_jobs, MAX_JOBS_PER_PIPELINE),
            });
        }

        // Initialize stage statuses
        init_stage_statuses(&mut run, &pipeline_config);

        // Convert to workflow definition
        let workflow_def = self.build_workflow_definition(&pipeline_config, &run.context)?;

        // Build workflow data
        let workflow_data = build_workflow_data(&run, &pipeline_config, &run.context);

        info!(
            run_id = %run.id,
            pipeline = %pipeline_config.name,
            repo_id = %run.context.repo_id.to_hex(),
            "Starting pipeline execution for existing run"
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

        // Update the tracked run
        self.active_runs.write().await.insert(run.id.clone(), run.clone());

        // Persist updated run
        if let Err(e) = self.persist_run(&run).await {
            warn!(run_id = %run.id, error = %e, "Failed to persist running run");
        }

        info!(
            run_id = %run.id,
            workflow_id = %workflow_id,
            "Pipeline workflow started for existing run"
        );

        Ok(run)
    }

    /// Get the count of active runs.
    pub async fn active_run_count(&self) -> usize {
        self.active_runs.read().await.len()
    }
}

/// Initialize stage statuses from a pipeline config onto a run.
fn init_stage_statuses(run: &mut PipelineRun, pipeline_config: &PipelineConfig) {
    for stage in &pipeline_config.stages {
        let mut jobs = HashMap::new();
        for job in &stage.jobs {
            jobs.insert(job.name.clone(), JobStatus {
                job_id: None,
                status: PipelineStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
                error: None,
            });
        }

        run.stages.push(StageStatus {
            name: stage.name.clone(),
            status: PipelineStatus::Pending,
            started_at: None,
            completed_at: None,
            jobs,
        });
    }
}

/// Build initial workflow data JSON from run context.
fn build_workflow_data(
    run: &PipelineRun,
    pipeline_config: &PipelineConfig,
    context: &PipelineContext,
) -> serde_json::Value {
    serde_json::json!({
        "run_id": run.id,
        "pipeline_name": pipeline_config.name,
        "repo_id": hex::encode(context.repo_id.0),
        "commit_hash": hex::encode(context.commit_hash),
        "ref_name": context.ref_name,
        "env": context.env,
    })
}

/// Convert a pipeline status to its string representation.
fn pipeline_status_str(status: PipelineStatus) -> &'static str {
    match status {
        PipelineStatus::Initializing => "initializing",
        PipelineStatus::CheckingOut => "checking_out",
        PipelineStatus::CheckoutFailed => "checkout_failed",
        PipelineStatus::Pending => "pending",
        PipelineStatus::Running => "running",
        PipelineStatus::Success => "success",
        PipelineStatus::Failed => "failed",
        PipelineStatus::Cancelled => "cancelled",
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
            checkout_dir: None,
            source_hash: None,
        };

        let run = PipelineRun::new("test-pipeline".to_string(), context);

        assert_eq!(run.pipeline_name, "test-pipeline");
        assert_eq!(run.status, PipelineStatus::Initializing);
        assert!(run.workflow_id.is_none());
        assert!(run.error_message.is_none());
    }

    #[test]
    fn test_pipeline_status_is_terminal() {
        // Non-terminal states
        assert!(!PipelineStatus::Initializing.is_terminal());
        assert!(!PipelineStatus::CheckingOut.is_terminal());
        assert!(!PipelineStatus::Pending.is_terminal());
        assert!(!PipelineStatus::Running.is_terminal());
        // Terminal states
        assert!(PipelineStatus::CheckoutFailed.is_terminal());
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
            checkout_dir: None,
            source_hash: None,
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
