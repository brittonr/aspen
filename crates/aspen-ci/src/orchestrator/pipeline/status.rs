//! Pipeline run status synchronization.
//!
//! Syncs pipeline run status from the underlying aspen-jobs workflow state,
//! handling cases where the workflow manager may not be updated when jobs
//! complete directly.

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_core::KeyValueStore;
use aspen_jobs::JobId;
use aspen_jobs::JobStatus as AspenJobStatus;
use chrono::DateTime;
use chrono::Utc;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::PipelineOrchestrator;
use super::PipelineRun;
use super::PipelineStatus;

#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "pipeline status sync stamps wall-clock completion times")]
fn current_time_utc() -> DateTime<Utc> {
    Utc::now()
}

struct WorkflowStatusInputs<'a> {
    state_name: &'a str,
    has_no_failed_jobs: bool,
    has_pending_deploys: bool,
}

impl<S: KeyValueStore + ?Sized + 'static> PipelineOrchestrator<S> {
    /// Sync pipeline run status from the underlying workflow state.
    ///
    /// If the workflow has completed (reached terminal state), updates the
    /// pipeline run status accordingly and persists the change.
    ///
    /// Since the workflow manager's state may not be updated when jobs complete
    /// (there's no callback from JobManager.mark_completed to WorkflowManager),
    /// we also check the actual job statuses directly.
    pub(crate) async fn sync_run_status(&self, run: &PipelineRun) -> Option<PipelineRun> {
        let workflow_id = run.workflow_id.as_ref()?;

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

        let all_job_ids: Vec<_> = workflow_state
            .active_jobs
            .iter()
            .chain(workflow_state.completed_jobs.iter())
            .chain(workflow_state.failed_jobs.iter())
            .collect();

        let job_name_to_info = self.collect_job_info(&all_job_ids, run).await;

        let mut new_status = determine_status_from_workflow(WorkflowStatusInputs {
            state_name: &workflow_state.state,
            has_no_failed_jobs: workflow_state.failed_jobs.is_empty(),
            has_pending_deploys: run.has_pending_deploys,
        });

        info!(
            run_id = %run.id,
            workflow_state = %workflow_state.state,
            active_jobs = workflow_state.active_jobs.len(),
            completed_jobs = workflow_state.completed_jobs.len(),
            failed_jobs = workflow_state.failed_jobs.len(),
            workflow_status = ?new_status,
            "sync_run_status: workflow state check"
        );

        if new_status.is_none() && !workflow_state.active_jobs.is_empty() {
            new_status = self.check_active_job_statuses(&workflow_state.active_jobs, run).await;
            info!(
                run_id = %run.id,
                job_check_result = ?new_status,
                "sync_run_status: active job status check result"
            );
        }

        let mut updated_run = run.clone();
        let was_any_job_updated = update_stage_job_info(&mut updated_run, &job_name_to_info);

        if let Some(status) = new_status {
            self.finalize_status_sync(&mut updated_run, status, workflow_id).await;
            return Some(updated_run);
        }

        if was_any_job_updated {
            self.persist_job_info_update(&updated_run).await;
            return Some(updated_run);
        }

        None
    }

    /// Collect job name to (job_id, status) mapping from job ID lists.
    async fn collect_job_info(
        &self,
        job_ids: &[&JobId],
        run: &PipelineRun,
    ) -> HashMap<String, (JobId, PipelineStatus)> {
        let mut job_name_to_info: HashMap<String, (JobId, PipelineStatus)> = HashMap::with_capacity(job_ids.len());
        for job_id in job_ids {
            if let Ok(Some(job)) = self.job_manager.get_job(job_id).await {
                if let Some(name) = job.spec.payload.get("job_name").and_then(|value| value.as_str()) {
                    let pipeline_status = aspen_job_status_to_pipeline(job.status);
                    job_name_to_info.insert(name.to_string(), ((*job_id).clone(), pipeline_status));
                }
            } else {
                debug!(
                    run_id = %run.id,
                    job_id = %job_id,
                    "Could not fetch job info during status sync"
                );
            }
        }
        job_name_to_info
    }

    /// Check active job statuses directly to determine pipeline completion.
    async fn check_active_job_statuses(
        &self,
        active_job_ids: &HashSet<JobId>,
        run: &PipelineRun,
    ) -> Option<PipelineStatus> {
        let mut is_every_job_completed = true;
        let mut has_any_failed_job = false;

        for job_id in active_job_ids {
            match self.job_manager.get_job(job_id).await {
                Ok(Some(job)) => {
                    debug!(
                        run_id = %run.id,
                        job_id = %job_id,
                        status = ?job.status,
                        "check_active_job_statuses: job found"
                    );
                    match job.status {
                        AspenJobStatus::Completed => {}
                        AspenJobStatus::Failed | AspenJobStatus::Cancelled | AspenJobStatus::DeadLetter => {
                            has_any_failed_job = true;
                        }
                        AspenJobStatus::Pending
                        | AspenJobStatus::Running
                        | AspenJobStatus::Retrying
                        | AspenJobStatus::Scheduled
                        | AspenJobStatus::Unknown => {
                            is_every_job_completed = false;
                        }
                    }
                }
                Ok(None) => {
                    warn!(
                        run_id = %run.id,
                        job_id = %job_id,
                        "Job not found during status sync, treating as incomplete (read failure or replication lag)"
                    );
                    is_every_job_completed = false;
                }
                Err(e) => {
                    debug!(
                        run_id = %run.id,
                        job_id = %job_id,
                        error = %e,
                        "Failed to get job status during sync"
                    );
                    is_every_job_completed = false;
                }
            }
        }

        if is_every_job_completed {
            Some(if has_any_failed_job {
                PipelineStatus::Failed
            } else {
                PipelineStatus::Success
            })
        } else {
            None
        }
    }

    /// Finalize a status sync by updating caches, persisting, and cleaning up.
    async fn finalize_status_sync(&self, updated_run: &mut PipelineRun, status: PipelineStatus, workflow_id: &str) {
        updated_run.status = status;
        updated_run.completed_at = Some(current_time_utc());

        self.active_runs.write().await.insert(updated_run.id.clone(), updated_run.clone());

        if let Err(e) = self.persist_run(updated_run).await {
            warn!(
                run_id = %updated_run.id,
                error = %e,
                "Failed to persist synced run status to KV store"
            );
        }

        if status.is_terminal() {
            if let Some(count) = self.runs_per_repo.write().await.get_mut(&updated_run.context.repo_id) {
                *count = count.saturating_sub(1);
            }

            if let Some(ref checkout_dir) = updated_run.context.checkout_dir {
                if let Err(e) = crate::checkout::cleanup_checkout(checkout_dir).await {
                    warn!(
                        run_id = %updated_run.id,
                        checkout_dir = %checkout_dir.display(),
                        error = %e,
                        "Failed to cleanup checkout directory (non-fatal)"
                    );
                }
            }
        }

        info!(
            run_id = %updated_run.id,
            workflow_id = %workflow_id,
            status = ?status,
            "Pipeline run status synced from workflow"
        );

        if status.is_terminal() {
            if let Some(reporter) = self.get_status_reporter() {
                let check_state = crate::status_reporter::pipeline_status_to_check_state(&status);
                let description = pipeline_completion_description(status, &updated_run.pipeline_name);
                let commit_status_update = crate::status_reporter::CommitStatusReport {
                    repo_id_hex: updated_run.context.repo_id.to_hex(),
                    commit_hash: updated_run.context.commit_hash,
                    context: "ci/pipeline".to_string(),
                    state: check_state,
                    description,
                    pipeline_run_id: updated_run.id.clone(),
                    ref_name: updated_run.context.ref_name.clone(),
                };
                if let Err(e) = reporter.report_status(commit_status_update).await {
                    warn!(
                        run_id = %updated_run.id,
                        error = %e,
                        "Failed to report terminal status"
                    );
                }
            }
        }
    }

    /// Persist job info updates without a pipeline status change.
    async fn persist_job_info_update(&self, updated_run: &PipelineRun) {
        self.active_runs.write().await.insert(updated_run.id.clone(), updated_run.clone());

        if let Err(e) = self.persist_run(updated_run).await {
            debug!(
                run_id = %updated_run.id,
                error = %e,
                "Failed to persist job info update to KV store"
            );
        }
    }
}

/// Convert an aspen-jobs status to a pipeline status.
fn aspen_job_status_to_pipeline(status: AspenJobStatus) -> PipelineStatus {
    match status {
        AspenJobStatus::Pending | AspenJobStatus::Scheduled => PipelineStatus::Pending,
        AspenJobStatus::Running | AspenJobStatus::Retrying => PipelineStatus::Running,
        AspenJobStatus::Unknown => PipelineStatus::Running,
        AspenJobStatus::Completed => PipelineStatus::Success,
        AspenJobStatus::Failed | AspenJobStatus::DeadLetter => PipelineStatus::Failed,
        AspenJobStatus::Cancelled => PipelineStatus::Cancelled,
    }
}

/// Determine pipeline status from workflow state fields.
///
/// When `has_pending_deploys` is true, reaching "done" does NOT produce
/// `Success` — deploy stages still need to run. The deploy monitor will
/// set the final status after deploys complete.
fn determine_status_from_workflow(input: WorkflowStatusInputs<'_>) -> Option<PipelineStatus> {
    if input.state_name == "done" {
        if input.has_pending_deploys {
            None
        } else {
            Some(PipelineStatus::Success)
        }
    } else if input.state_name == "failed" {
        Some(PipelineStatus::Failed)
    } else if !input.has_no_failed_jobs {
        Some(PipelineStatus::Failed)
    } else {
        None
    }
}

/// Update stage job IDs, job statuses, and aggregate stage statuses from collected info.
/// Returns true if any were updated.
fn update_stage_job_info(run: &mut PipelineRun, job_name_to_info: &HashMap<String, (JobId, PipelineStatus)>) -> bool {
    debug_assert!(!run.id.is_empty());
    debug_assert!(run.stages.iter().all(|stage| !stage.name.is_empty()));
    let mut was_any_stage_updated = false;

    for stage in &mut run.stages {
        for (job_name, job_status) in &mut stage.jobs {
            if let Some((job_id, status)) = job_name_to_info.get(job_name) {
                if job_status.job_id.is_none() {
                    job_status.job_id = Some(job_id.clone());
                    was_any_stage_updated = true;
                }
                if job_status.status != *status {
                    job_status.status = *status;
                    was_any_stage_updated = true;
                }
            }
        }

        let new_stage_status = compute_stage_status(&stage.jobs);
        if stage.status != new_stage_status {
            stage.status = new_stage_status;
            if stage.started_at.is_none() && new_stage_status != PipelineStatus::Pending {
                stage.started_at = Some(current_time_utc());
            }
            if stage.completed_at.is_none() && new_stage_status.is_terminal() {
                stage.completed_at = Some(current_time_utc());
            }
            was_any_stage_updated = true;
        }
    }

    debug_assert!(run.stages.iter().all(|stage| stage.completed_at.is_none() || stage.started_at.is_some()));
    was_any_stage_updated
}

/// Compute aggregate stage status from its job statuses.
///
/// Rules:
/// - Any job failed/cancelled → stage is failed/cancelled
/// - Any job running → stage is running
/// - All jobs succeeded → stage is success
/// - Otherwise → stage is pending
fn compute_stage_status(jobs: &HashMap<String, super::JobStatus>) -> PipelineStatus {
    debug_assert!(jobs.keys().all(|job_name| !job_name.is_empty()));
    if jobs.is_empty() {
        return PipelineStatus::Pending;
    }

    let mut is_every_job_successful = true;
    let mut has_any_running_job = false;
    let mut has_any_failed_job = false;
    let mut has_any_cancelled_job = false;

    for job in jobs.values() {
        match job.status {
            PipelineStatus::Success => {}
            PipelineStatus::Failed => {
                has_any_failed_job = true;
                is_every_job_successful = false;
            }
            PipelineStatus::Cancelled => {
                has_any_cancelled_job = true;
                is_every_job_successful = false;
            }
            PipelineStatus::Running => {
                has_any_running_job = true;
                is_every_job_successful = false;
            }
            _ => {
                is_every_job_successful = false;
            }
        }
    }

    debug_assert!(
        has_any_failed_job
            || has_any_cancelled_job
            || has_any_running_job
            || is_every_job_successful
            || !jobs.is_empty()
    );
    if has_any_failed_job {
        return PipelineStatus::Failed;
    }
    if has_any_cancelled_job {
        return PipelineStatus::Cancelled;
    }
    if is_every_job_successful {
        return PipelineStatus::Success;
    }
    if has_any_running_job {
        return PipelineStatus::Running;
    }
    PipelineStatus::Pending
}

fn pipeline_completion_description(status: PipelineStatus, pipeline_name: &str) -> String {
    debug_assert!(!pipeline_name.is_empty());
    match status {
        PipelineStatus::Success => format!("Pipeline '{}' succeeded", pipeline_name),
        PipelineStatus::Failed => format!("Pipeline '{}' failed", pipeline_name),
        PipelineStatus::Cancelled => format!("Pipeline '{}' cancelled", pipeline_name),
        _ => format!("Pipeline '{}' completed", pipeline_name),
    }
}
