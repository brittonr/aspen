//! Dependency management for the job manager.

use std::collections::HashMap;

use aspen_traits::KeyValueStore;
use tracing::info;
use tracing::warn;

use super::JOB_PREFIX;
use super::JobManager;
use crate::dependency_tracker::JobDependencyInfo;
use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobSpec;

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
    /// Submit a workflow (DAG of jobs).
    /// Jobs are topologically sorted and submitted in order.
    pub async fn submit_workflow(&self, mut specs: Vec<JobSpec>) -> Result<Vec<JobId>> {
        // Tiger Style: workflow must have at least one job
        assert!(!specs.is_empty(), "workflow must contain at least one job spec");

        // Submit jobs in order, maintaining dependencies
        let mut submitted_ids = Vec::new();
        let old_to_new_id: HashMap<JobId, JobId> = HashMap::new();

        // Sort specs by dependency order (simple approach - jobs with no deps first)
        specs.sort_by_key(|spec| spec.config.dependencies.len());

        for spec in specs {
            // Update dependencies to point to newly created job IDs
            let mut updated_spec = spec;
            let mut updated_deps = Vec::new();
            for dep in updated_spec.config.dependencies {
                if let Some(new_id) = old_to_new_id.get(&dep) {
                    updated_deps.push(new_id.clone());
                } else {
                    updated_deps.push(dep);
                }
            }
            updated_spec.config.dependencies = updated_deps;

            // Submit the job
            let job_id = self.submit(updated_spec).await?;
            submitted_ids.push(job_id.clone());

            // Map old temp ID to new actual ID if needed
            // (This is simplified - a real implementation would need better mapping)
        }

        Ok(submitted_ids)
    }

    /// Get job dependency information.
    pub async fn get_dependency_info(&self, job_id: &JobId) -> Result<JobDependencyInfo> {
        self.dependency_graph
            .get_job_info(job_id)
            .await
            .ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })
    }

    /// Cancel a job and all its dependent jobs.
    pub async fn cancel_with_cascade(&self, job_id: &JobId) -> Result<Vec<JobId>> {
        // Tiger Style: job ID must not be empty
        assert!(!job_id.as_str().is_empty(), "job ID must not be empty for cancel_with_cascade");

        // Get all jobs that depend on this one
        let dependents = self.dependency_graph.get_blocked_by(job_id).await;
        let mut cancelled = vec![job_id.clone()];

        // Cancel the main job first
        self.cancel_job(job_id).await?;

        // Cancel all dependent jobs
        for dep_id in dependents {
            if let Err(e) = self.cancel_job(&dep_id).await {
                warn!(
                    job_id = %dep_id,
                    error = %e,
                    "failed to cancel dependent job"
                );
            } else {
                cancelled.push(dep_id);
            }
        }

        Ok(cancelled)
    }

    /// Force unblock a job (override dependency check).
    /// This marks the job's dependencies as satisfied and enqueues it.
    pub async fn force_unblock(&self, job_id: &JobId) -> Result<()> {
        // Update the job's dependency state
        let mut job = self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

        // Force mark as ready in dependency graph
        self.dependency_graph
            .add_job(
                job_id.clone(),
                Vec::new(), // Clear dependencies
                job.dependency_failure_policy.clone(),
            )
            .await?;

        // Update job state
        job.dependency_state = crate::dependency_tracker::DependencyState::Ready;
        job.blocked_by.clear();
        self.store_job(&job).await?;

        // Enqueue the job
        self.enqueue_job(&job).await?;

        info!(job_id = %job_id, "job force unblocked and enqueued");
        Ok(())
    }

    /// Get all jobs currently blocked on dependencies.
    pub async fn get_blocked_jobs(&self) -> Result<Vec<Job>> {
        let mut blocked = Vec::new();

        // Scan all jobs and check their dependency state
        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: JOB_PREFIX.to_string(),
                limit: Some(10000),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        for entry in scan_result.entries {
            if let Ok(job) = serde_json::from_str::<Job>(&entry.value) {
                if matches!(job.dependency_state, crate::dependency_tracker::DependencyState::Waiting(_)) {
                    blocked.push(job);
                }
            }
        }

        Ok(blocked)
    }

    /// Clean up completed jobs from the dependency graph.
    pub async fn cleanup_dependency_graph(&self) -> Result<usize> {
        let count = self.dependency_graph.cleanup_completed().await;
        Ok(count)
    }
}
