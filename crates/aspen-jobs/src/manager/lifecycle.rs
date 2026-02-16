//! Job lifecycle operations: submit, cancel, start, complete, ack, nack.

use aspen_coordination::DequeuedItem;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use chrono::Utc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::JOB_PREFIX;
use super::JobCompletionCallback;
use super::JobManager;
use crate::error::JobError;
use crate::error::Result;
use crate::job::DLQReason;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobResult;
use crate::job::JobSpec;
use crate::job::JobStatus;
use crate::types::Priority;

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
    /// Set a callback to be called when jobs complete.
    ///
    /// This is used by the workflow manager to trigger state transitions.
    pub async fn set_completion_callback(&self, callback: JobCompletionCallback) {
        *self.completion_callback.write().await = Some(callback);
    }

    /// Submit a new job.
    pub async fn submit(&self, spec: JobSpec) -> Result<JobId> {
        // Tiger Style: job type must not be empty
        assert!(!spec.job_type.is_empty(), "job type must not be empty when submitting");

        // Ensure queues are initialized before submitting
        self.ensure_initialized().await?;
        // Validate dependencies exist
        for dep_id in &spec.config.dependencies {
            if !self.job_exists(dep_id).await? {
                return Err(JobError::InvalidJobSpec {
                    reason: format!("Dependency not found: {}", dep_id),
                });
            }
        }

        // Create job from spec
        let mut job = Job::from_spec(spec);

        // Check for deduplication
        if let Some(ref idempotency_key) = job.spec.idempotency_key {
            if self.config.enable_deduplication {
                let existing_id = self.check_idempotency_key(idempotency_key).await?;
                if let Some(id) = existing_id {
                    debug!(
                        idempotency_key,
                        job_id = %id,
                        "job already exists with idempotency key"
                    );
                    return Ok(id);
                }
            }
        }

        // Register with dependency graph
        self.dependency_graph
            .add_job(job.id.clone(), job.spec.config.dependencies.clone(), job.dependency_failure_policy.clone())
            .await?;

        // Check if dependencies are ready
        let should_enqueue = if !job.spec.config.dependencies.is_empty() {
            let is_ready = self.dependency_graph.check_dependencies(&job.id).await?;
            if !is_ready {
                // Update job's dependency state from the graph
                if let Some(dep_info) = self.dependency_graph.get_job_info(&job.id).await {
                    job.dependency_state = dep_info.state;
                }
                info!(job_id = %job.id, "job blocked on dependencies");
            }
            is_ready
        } else {
            true
        };

        // Store job metadata
        self.store_job(&job).await?;

        // If job is scheduled for future, add to schedule index
        if let Some(scheduled_at) = job.scheduled_at {
            if scheduled_at > Utc::now() {
                self.add_to_schedule_index(&job.id, scheduled_at).await?;
                info!(job_id = %job.id, ?scheduled_at, "job scheduled for future execution");
                return Ok(job.id);
            }
        }

        // Only enqueue if dependencies are satisfied
        if should_enqueue {
            self.enqueue_job(&job).await?;
        }

        // Store idempotency key if provided
        if let Some(ref idempotency_key) = job.spec.idempotency_key {
            if self.config.enable_deduplication {
                self.store_idempotency_key(idempotency_key, &job.id).await?;
            }
        }

        info!(
            job_id = %job.id,
            job_type = %job.spec.job_type,
            priority = ?job.spec.config.priority,
            blocked = !should_enqueue,
            "job submitted"
        );

        Ok(job.id)
    }

    /// Dequeue jobs for processing by workers.
    ///
    /// Returns up to `max_jobs` jobs from the highest priority queues first.
    /// If `excluded_types` is provided, jobs with those types are skipped.
    pub async fn dequeue_jobs(
        &self,
        worker_id: &str,
        max_jobs: u32,
        visibility_timeout: std::time::Duration,
    ) -> Result<Vec<(DequeuedItem, Job)>> {
        self.dequeue_jobs_filtered(worker_id, max_jobs, visibility_timeout, &[]).await
    }

    /// Dequeue jobs for processing by workers, excluding certain job types.
    ///
    /// Returns up to `max_jobs` jobs from the highest priority queues first.
    /// Jobs with types in `excluded_types` are skipped and remain visible for
    /// other workers to claim.
    pub async fn dequeue_jobs_filtered(
        &self,
        worker_id: &str,
        max_jobs: u32,
        visibility_timeout: std::time::Duration,
        excluded_types: &[String],
    ) -> Result<Vec<(DequeuedItem, Job)>> {
        // Tiger Style: worker ID must not be empty
        assert!(!worker_id.is_empty(), "worker_id must not be empty for dequeue_jobs_filtered");
        // Tiger Style: must request at least one job
        assert!(max_jobs > 0, "max_jobs must be positive, got 0");

        // Ensure queues are initialized before dequeuing
        self.ensure_initialized().await?;
        let mut dequeued_jobs = Vec::new();
        let visibility_timeout_ms = visibility_timeout.as_millis() as u64;

        // Process queues by priority order
        for priority in Priority::all_ordered() {
            if dequeued_jobs.len() >= max_jobs as usize {
                break;
            }

            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            if let Some(queue_manager) = self.queue_managers.get(&priority) {
                // Only try once per priority level - if we hit excluded jobs, we'll let other
                // workers have a chance rather than retrying in a tight loop
                if dequeued_jobs.len() < max_jobs as usize {
                    let items_to_dequeue = max_jobs - dequeued_jobs.len() as u32;

                    let items = queue_manager
                        .dequeue(&queue_name, worker_id, items_to_dequeue, visibility_timeout_ms)
                        .await
                        .map_err(|e| JobError::QueueError { source: e })?;

                    // If no items available, move to next priority
                    if items.is_empty() {
                        continue;
                    }

                    info!(worker_id, queue_name, items_count = items.len(), "dequeue returned items");

                    for item in items {
                        // Parse job ID from payload
                        let job_id_str =
                            String::from_utf8(item.payload.clone()).map_err(|_| JobError::InvalidJobSpec {
                                reason: "invalid job ID in queue payload".to_string(),
                            })?;
                        let job_id = JobId::from_string(job_id_str);

                        // Retrieve job from storage
                        if let Some(job) = self.get_job(&job_id).await? {
                            // Check if job type is excluded
                            // Decomposed: check if filter is active, then check if type matches
                            let has_exclusions = !excluded_types.is_empty();
                            let is_excluded_type = excluded_types.contains(&job.spec.job_type);
                            if has_exclusions && is_excluded_type {
                                // Release the job back to the queue immediately so other workers can claim it.
                                // Use nack with move_to_dlq=false to return it without marking as failed.
                                info!(
                                    worker_id,
                                    job_id = %job_id,
                                    job_type = %job.spec.job_type,
                                    receipt_handle = %item.receipt_handle,
                                    "skipping excluded job type, releasing back to queue"
                                );
                                match queue_manager
                                    .nack(
                                        &queue_name,
                                        &item.receipt_handle,
                                        false, // Don't move to DLQ
                                        Some(format!("job type {} excluded by worker filter", job.spec.job_type)),
                                    )
                                    .await
                                {
                                    Ok(()) => {
                                        info!(
                                            worker_id,
                                            job_id = %job_id,
                                            "excluded job successfully released back to queue"
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            worker_id,
                                            job_id = %job_id,
                                            error = %e,
                                            "CRITICAL: failed to release excluded job back to queue - job may be lost"
                                        );
                                    }
                                }
                                continue;
                            }
                            dequeued_jobs.push((item, job));
                        } else {
                            warn!(job_id = %job_id, "job not found in storage, skipping");
                        }
                    }

                    // We only dequeue once per priority level. If we hit excluded jobs,
                    // they've been nacked back to the queue and will be available for
                    // other workers (e.g., VM workers) to claim on their next poll.
                }
            }
        }

        debug!(worker_id, count = dequeued_jobs.len(), "dequeued jobs for processing");

        Ok(dequeued_jobs)
    }

    /// Acknowledge successful job completion.
    ///
    /// The `execution_token` must match the token returned by `mark_started`.
    /// This prevents race conditions where a stale worker tries to complete
    /// a job that has been reassigned to another worker.
    pub async fn ack_job(
        &self,
        job_id: &JobId,
        receipt_handle: &str,
        execution_token: &str,
        result: JobResult,
    ) -> Result<()> {
        // Tiger Style: receipt handle and execution token must not be empty
        assert!(!receipt_handle.is_empty(), "receipt_handle must not be empty for ack_job on {job_id}");
        assert!(!execution_token.is_empty(), "execution_token must not be empty for ack_job on {job_id}");

        // Get job and validate execution token
        let job = self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

        // Validate execution token
        match &job.execution_token {
            Some(stored_token) if stored_token == execution_token => {
                // Token matches, proceed
            }
            Some(_) => {
                // Token mismatch - stale execution attempt
                warn!(
                    job_id = %job_id,
                    "ack_job rejected: stale execution token"
                );
                return Err(JobError::InvalidJobState {
                    state: "Stale execution token".to_string(),
                    operation: "ack_job".to_string(),
                });
            }
            None => {
                // No token stored - shouldn't happen for Running jobs
                warn!(
                    job_id = %job_id,
                    "ack_job rejected: job has no execution token"
                );
                return Err(JobError::InvalidJobState {
                    state: "No execution token".to_string(),
                    operation: "ack_job".to_string(),
                });
            }
        }

        let priority = job.spec.config.priority;
        let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());

        // Acknowledge the queue item
        if let Some(queue_manager) = self.queue_managers.get(&priority) {
            queue_manager
                .ack(&queue_name, receipt_handle)
                .await
                .map_err(|e| JobError::QueueError { source: e })?;
        }

        // Mark job as completed
        self.mark_completed(job_id, result).await?;

        Ok(())
    }

    /// Negative acknowledge a job (return to queue or move to DLQ).
    ///
    /// The `execution_token` must match the token returned by `mark_started`.
    /// This prevents race conditions where a stale worker tries to nack
    /// a job that has been reassigned to another worker.
    pub async fn nack_job(
        &self,
        job_id: &JobId,
        receipt_handle: &str,
        execution_token: &str,
        error: String,
    ) -> Result<()> {
        // Tiger Style: receipt handle and execution token must not be empty
        assert!(!receipt_handle.is_empty(), "receipt_handle must not be empty for nack_job on {job_id}");
        assert!(!execution_token.is_empty(), "execution_token must not be empty for nack_job on {job_id}");

        // First validate execution token before making any changes
        let current_job =
            self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

        // Validate execution token
        match &current_job.execution_token {
            Some(stored_token) if stored_token == execution_token => {
                // Token matches, proceed
            }
            Some(_) => {
                // Token mismatch - stale execution attempt
                warn!(
                    job_id = %job_id,
                    "nack_job rejected: stale execution token"
                );
                return Err(JobError::InvalidJobState {
                    state: "Stale execution token".to_string(),
                    operation: "nack_job".to_string(),
                });
            }
            None => {
                // No token stored - shouldn't happen for Running jobs
                warn!(
                    job_id = %job_id,
                    "nack_job rejected: job has no execution token"
                );
                return Err(JobError::InvalidJobState {
                    state: "No execution token".to_string(),
                    operation: "nack_job".to_string(),
                });
            }
        }

        // Now atomically update the job status
        let job = self
            .atomic_update_job(job_id, |job| {
                // Only allow nack for jobs that are currently running
                if job.status != JobStatus::Running {
                    // If it's already in a terminal state, that's ok (idempotent)
                    if job.status.is_terminal() {
                        return Ok(());
                    }
                    // Otherwise it's an invalid state transition
                    return Err(JobError::InvalidJobState {
                        state: format!("{:?}", job.status),
                        operation: "nack_job".to_string(),
                    });
                }

                // Check if job should be retried
                let should_retry = !job.exceeded_retry_limit();

                if should_retry {
                    // Calculate next retry time
                    if let Some(next_retry) = job.calculate_next_retry() {
                        job.mark_retry(next_retry, error.clone());
                    } else {
                        // Shouldn't happen, but handle gracefully
                        job.mark_dlq(DLQReason::MaxRetriesExceeded, error.clone());
                    }
                } else {
                    // Move to DLQ as retry limit exceeded
                    job.mark_dlq(DLQReason::MaxRetriesExceeded, error.clone());
                }

                Ok(())
            })
            .await?;

        let priority = job.spec.config.priority;
        let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());

        // Check the final status to determine queue action
        let move_to_dlq = job.status == JobStatus::DeadLetter;

        // Nack the queue item
        if let Some(queue_manager) = self.queue_managers.get(&priority) {
            queue_manager
                .nack(&queue_name, receipt_handle, move_to_dlq, Some(error.clone()))
                .await
                .map_err(|e| JobError::QueueError { source: e })?;
        }

        // Log the outcome
        match job.status {
            JobStatus::Retrying => {
                info!(
                    job_id = %job_id,
                    next_retry = ?job.next_retry_at,
                    attempts = job.attempts,
                    "job scheduled for retry"
                );
            }
            JobStatus::DeadLetter => {
                warn!(
                    job_id = %job_id,
                    attempts = job.attempts,
                    reason = ?job.dlq_metadata.as_ref().map(|m| &m.reason),
                    "job moved to dead letter queue"
                );

                // Notify completion callback for dead letter jobs (workflow integration)
                // This ensures workflows can track failed jobs that exhausted retries
                if let Some(callback) = self.completion_callback.read().await.as_ref() {
                    let result = JobResult::failure(error.clone());
                    callback(job_id.clone(), result).await;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, id: &JobId) -> Result<()> {
        let mut job = self.get_job(id).await?.ok_or_else(|| JobError::JobNotFound { id: id.to_string() })?;

        if job.status.is_terminal() {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", job.status),
                operation: "cancel".to_string(),
            });
        }

        job.mark_cancelled();
        self.store_job(&job).await?;

        info!(job_id = %id, "job cancelled");
        Ok(())
    }

    /// Update job progress.
    pub async fn update_progress(&self, id: &JobId, progress: u8, message: Option<String>) -> Result<()> {
        self.atomic_update_job(id, move |job| {
            if job.status != JobStatus::Running {
                return Err(JobError::InvalidJobState {
                    state: format!("{:?}", job.status),
                    operation: "update_progress".to_string(),
                });
            }

            job.update_progress(progress, message.clone());
            Ok(())
        })
        .await?;

        Ok(())
    }

    /// Mark a job as started and return an execution token.
    ///
    /// The execution token must be provided to `ack_job` or `nack_job` to prove
    /// ownership of the job execution. This prevents race conditions where:
    /// - Visibility timeout re-enqueues a Running job
    /// - A second worker dequeues and tries to execute
    /// - Both workers try to complete the same job
    ///
    /// With execution tokens, only the worker holding the current token can complete.
    pub async fn mark_started(&self, id: &JobId, worker_id: String) -> Result<String> {
        // Tiger Style: worker ID must not be empty
        assert!(!worker_id.is_empty(), "worker_id must not be empty for mark_started on job {id}");
        // Tiger Style: job ID must not be empty
        assert!(!id.as_str().is_empty(), "job ID must not be empty for mark_started");

        let worker_id_clone = worker_id.clone();
        let worker_id_for_closure = worker_id.clone();

        let job = self
            .atomic_update_job(id, move |job| {
                // If job is already running with the same worker, treat as idempotent success
                // but still return the existing token
                if job.status == JobStatus::Running {
                    if let Some(ref current_worker) = job.worker_id {
                        if current_worker == &worker_id_for_closure {
                            // Already running with same worker, no-op (token stays the same)
                            return Ok(());
                        }
                    }
                    // Different worker trying to start an already running job
                    return Err(JobError::InvalidJobState {
                        state: format!("{:?}", job.status),
                        operation: "mark_started".to_string(),
                    });
                }

                if !job.can_execute_now() {
                    return Err(JobError::InvalidJobState {
                        state: format!("{:?}", job.status),
                        operation: "mark_started".to_string(),
                    });
                }

                job.mark_started(worker_id_for_closure.clone());
                Ok(())
            })
            .await?;

        // Update dependency graph
        self.dependency_graph.mark_running(id).await?;

        // Extract execution token from the updated job
        let token = job.execution_token.ok_or_else(|| JobError::InvalidJobState {
            state: "No execution token generated".to_string(),
            operation: "mark_started".to_string(),
        })?;

        debug!(job_id = %id, worker_id = worker_id_clone, "job marked as started");
        Ok(token)
    }

    /// Update job heartbeat.
    ///
    /// Workers should call this periodically during job execution to indicate
    /// the job is still being processed. This is used by recovery mechanisms
    /// to detect orphaned jobs after leader failover.
    pub async fn update_heartbeat(&self, id: &JobId) -> Result<()> {
        let heartbeat_key = format!("_jobs:heartbeat:{}", id.as_str());
        let timestamp = Utc::now().timestamp_millis().to_string();

        let write_request = WriteRequest {
            command: WriteCommand::Set {
                key: heartbeat_key,
                value: timestamp,
            },
        };

        self.store.write(write_request).await.map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Remove job heartbeat.
    ///
    /// Called when a job completes or fails to clean up the heartbeat entry.
    pub async fn remove_heartbeat(&self, id: &JobId) -> Result<()> {
        let heartbeat_key = format!("_jobs:heartbeat:{}", id.as_str());

        let write_request = WriteRequest {
            command: WriteCommand::Delete { key: heartbeat_key },
        };

        self.store.write(write_request).await.map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Mark a job as completed.
    pub async fn mark_completed(&self, id: &JobId, result: JobResult) -> Result<()> {
        let is_success = result.is_success();
        let result_clone = result.clone();

        self.atomic_update_job(id, move |job| {
            if job.status != JobStatus::Running && job.status != JobStatus::Retrying {
                return Err(JobError::InvalidJobState {
                    state: format!("{:?}", job.status),
                    operation: "mark_completed".to_string(),
                });
            }

            job.mark_completed(result_clone.clone());
            Ok(())
        })
        .await?;

        // Handle dependency graph updates
        if is_success {
            // Mark as completed and get unblocked jobs
            let unblocked = self.dependency_graph.mark_completed(id).await?;

            // Enqueue each unblocked job
            for job_id in unblocked {
                if let Some(mut job) = self.get_job(&job_id).await? {
                    // Update job's dependency state
                    if let Some(dep_info) = self.dependency_graph.get_job_info(&job_id).await {
                        job.dependency_state = dep_info.state;
                        self.store_job(&job).await?;
                    }

                    self.enqueue_job(&job).await?;
                    info!(job_id = %job_id, "job unblocked and enqueued");
                }
            }

            info!(job_id = %id, "job completed successfully");
        } else {
            // Handle failure propagation
            let failure_reason = match &result {
                JobResult::Failure(failure) => failure.reason.clone(),
                JobResult::Cancelled => "Job was cancelled".to_string(),
                _ => "Unknown failure".to_string(),
            };

            let affected = self.dependency_graph.mark_failed(id, failure_reason.clone()).await?;

            // Update affected jobs based on their failure policy
            for job_id in affected {
                if let Some(mut job) = self.get_job(&job_id).await? {
                    // Update job's dependency state from the graph
                    if let Some(dep_info) = self.dependency_graph.get_job_info(&job_id).await {
                        job.dependency_state = dep_info.state.clone();

                        // If the job was marked as failed due to cascade, update its status
                        if matches!(dep_info.state, crate::dependency_tracker::DependencyState::Failed(_)) {
                            job.status = JobStatus::Failed;
                            job.last_error = Some(format!("Dependency {} failed: {}", id, failure_reason));
                        }

                        self.store_job(&job).await?;
                    }
                }
            }

            warn!(job_id = %id, "job failed");
        }

        // Notify completion callback (for workflow integration)
        if let Some(callback) = self.completion_callback.read().await.as_ref() {
            callback(id.clone(), result).await;
        }

        Ok(())
    }

    /// Release a dequeued job back to the queue without executing it.
    ///
    /// This is used when a worker dequeues a job but cannot handle it (e.g., no
    /// registered handler for the job type). Unlike `nack_job`, this does not
    /// require an execution token because the job was never started via
    /// `mark_started`.
    ///
    /// The job's status remains unchanged (typically Pending) and the queue
    /// item is returned to the queue for another worker to pick up.
    pub async fn release_unhandled_job(&self, job_id: &JobId, receipt_handle: &str, reason: String) -> Result<()> {
        // Get job to determine priority/queue
        let job = self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

        // Only allow releasing jobs that were never started (not Running)
        if job.status == JobStatus::Running {
            return Err(JobError::InvalidJobState {
                state: "Running".to_string(),
                operation: "release_unhandled_job (use nack_job for running jobs)".to_string(),
            });
        }

        let priority = job.spec.config.priority;
        let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());

        // Release the queue item back without counting against delivery attempts.
        // This is important for job type filtering - a worker that can't handle a job
        // shouldn't exhaust its retries.
        if let Some(queue_manager) = self.queue_managers.get(&priority) {
            queue_manager
                .release_unchanged(&queue_name, receipt_handle)
                .await
                .map_err(|e| JobError::QueueError { source: e })?;
        }

        info!(
            job_id = %job_id,
            reason = %reason,
            "released unhandled job back to queue (delivery count unchanged)"
        );

        Ok(())
    }
}
