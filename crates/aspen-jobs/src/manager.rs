//! Job manager for submitting and managing jobs.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_coordination::DequeuedItem;
use aspen_coordination::EnqueueOptions;
use aspen_coordination::QueueConfig;
use aspen_coordination::QueueManager;
use aspen_coordination::ServiceRegistry;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::dependency_tracker::DependencyGraph;

/// Callback type for job completion notifications.
pub type JobCompletionCallback =
    Arc<dyn Fn(JobId, JobResult) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;
use crate::dependency_tracker::JobDependencyInfo;
use crate::error::JobError;
use crate::error::Result;
use crate::job::DLQReason;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobResult;
use crate::job::JobSpec;
use crate::job::JobStatus;
use crate::types::DLQStats;
use crate::types::JobTypeStats;
use crate::types::Priority;
use crate::types::QueueStats;
use crate::types::Schedule;

/// Job storage key prefix.
const JOB_PREFIX: &str = "__jobs:";
/// Job schedule prefix.
const JOB_SCHEDULE_PREFIX: &str = "__jobs:schedule:";

/// Configuration for the job manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobManagerConfig {
    /// Default visibility timeout for jobs.
    pub default_visibility_timeout: Duration,
    /// Default job timeout.
    pub default_job_timeout: Duration,
    /// Whether to enable job deduplication.
    pub enable_deduplication: bool,
    /// TTL for deduplication entries.
    pub deduplication_ttl: Duration,
    /// Maximum jobs to schedule per tick.
    pub max_schedule_per_tick: usize,
    /// Scheduler tick interval.
    pub scheduler_interval: Duration,
}

impl Default for JobManagerConfig {
    fn default() -> Self {
        Self {
            default_visibility_timeout: Duration::from_secs(300), // 5 minutes
            default_job_timeout: Duration::from_secs(300),        // 5 minutes
            enable_deduplication: true,
            deduplication_ttl: Duration::from_secs(3600), // 1 hour
            max_schedule_per_tick: 100,
            scheduler_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Manager for job submission and lifecycle.
pub struct JobManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    queue_managers: HashMap<Priority, QueueManager<S>>,
    config: JobManagerConfig,
    #[allow(dead_code)] // Reserved for future service discovery integration
    service_registry: ServiceRegistry<S>,
    dependency_graph: Arc<DependencyGraph>,
    initialized: Arc<RwLock<bool>>,
    /// Optional callback for job completion notifications (used by workflow manager).
    completion_callback: RwLock<Option<JobCompletionCallback>>,
}

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
    /// Create a new job manager.
    pub fn new(store: Arc<S>) -> Self {
        Self::with_config(store, JobManagerConfig::default())
    }

    /// Create a job manager with custom configuration.
    pub fn with_config(store: Arc<S>, config: JobManagerConfig) -> Self {
        let mut queue_managers = HashMap::new();

        // Create queue manager for each priority level
        for priority in Priority::all_ordered() {
            let queue_manager = QueueManager::new(store.clone());
            queue_managers.insert(priority, queue_manager);
        }

        let service_registry = ServiceRegistry::new(store.clone());
        let dependency_graph = Arc::new(DependencyGraph::new());
        let initialized = Arc::new(RwLock::new(false));

        Self {
            store,
            queue_managers,
            config,
            service_registry,
            dependency_graph,
            initialized,
            completion_callback: RwLock::new(None),
        }
    }

    /// Set a callback to be called when jobs complete.
    ///
    /// This is used by the workflow manager to trigger state transitions.
    pub async fn set_completion_callback(&self, callback: JobCompletionCallback) {
        *self.completion_callback.write().await = Some(callback);
    }

    /// Ensure the job system is initialized (lazy initialization).
    async fn ensure_initialized(&self) -> Result<()> {
        // Fast path: already initialized
        let initialized = self.initialized.read().await;
        if *initialized {
            return Ok(());
        }
        drop(initialized);

        // Slow path: need to initialize
        let mut initialized = self.initialized.write().await;
        if !*initialized {
            self.initialize_internal().await?;
            *initialized = true;
        }
        Ok(())
    }

    /// Initialize the job system (create queues).
    pub async fn initialize(&self) -> Result<()> {
        self.ensure_initialized().await
    }

    /// Internal initialization logic.
    async fn initialize_internal(&self) -> Result<()> {
        // Create a queue for each priority level
        for priority in Priority::all_ordered() {
            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            let queue_config = QueueConfig {
                default_visibility_timeout_ms: Some(self.config.default_visibility_timeout.as_millis() as u64),
                default_ttl_ms: None,
                max_delivery_attempts: Some(3),
            };

            if let Some(queue_manager) = self.queue_managers.get(&priority) {
                queue_manager
                    .create(&queue_name, queue_config)
                    .await
                    .map_err(|e| JobError::QueueError { source: e })?;

                info!(queue_name, ?priority, "initialized job queue");
            }
        }

        Ok(())
    }

    /// Submit a new job.
    pub async fn submit(&self, spec: JobSpec) -> Result<JobId> {
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
    pub async fn dequeue_jobs(
        &self,
        worker_id: &str,
        max_jobs: u32,
        visibility_timeout: Duration,
    ) -> Result<Vec<(DequeuedItem, Job)>> {
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
                let items_to_dequeue = max_jobs - dequeued_jobs.len() as u32;

                let items = queue_manager
                    .dequeue(&queue_name, worker_id, items_to_dequeue, visibility_timeout_ms)
                    .await
                    .map_err(|e| JobError::QueueError { source: e })?;

                for item in items {
                    // Parse job ID from payload
                    let job_id_str = String::from_utf8(item.payload.clone()).map_err(|_| JobError::InvalidJobSpec {
                        reason: "invalid job ID in queue payload".to_string(),
                    })?;
                    let job_id = JobId::from_string(job_id_str);

                    // Retrieve job from storage
                    if let Some(job) = self.get_job(&job_id).await? {
                        dequeued_jobs.push((item, job));
                    } else {
                        warn!(job_id = %job_id, "job not found in storage, skipping");
                    }
                }
            }
        }

        debug!(worker_id, count = dequeued_jobs.len(), "dequeued jobs for processing");

        Ok(dequeued_jobs)
    }

    /// Acknowledge successful job completion.
    pub async fn ack_job(&self, job_id: &JobId, receipt_handle: &str, result: JobResult) -> Result<()> {
        // Get job priority to determine queue
        let job = self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

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
    pub async fn nack_job(&self, job_id: &JobId, receipt_handle: &str, error: String) -> Result<()> {
        // First, atomically update the job status
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

    /// Get a job by ID.
    pub async fn get_job(&self, id: &JobId) -> Result<Option<Job>> {
        let key = format!("{}{}", JOB_PREFIX, id.as_str());

        match self.store.read(ReadRequest::new(key)).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    let job: Job =
                        serde_json::from_str(&kv.value).map_err(|e| JobError::SerializationError { source: e })?;
                    Ok(Some(job))
                } else {
                    Ok(None)
                }
            }
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(JobError::StorageError { source: e }),
        }
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

    /// Get job status.
    pub async fn get_status(&self, id: &JobId) -> Result<JobStatus> {
        let job = self.get_job(id).await?.ok_or_else(|| JobError::JobNotFound { id: id.to_string() })?;

        Ok(job.status)
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

    /// Mark a job as started.
    pub async fn mark_started(&self, id: &JobId, worker_id: String) -> Result<()> {
        let worker_id_clone = worker_id.clone();
        let worker_id_for_closure = worker_id.clone();

        self.atomic_update_job(id, move |job| {
            // If job is already running with the same worker, treat as idempotent success
            if job.status == JobStatus::Running {
                if let Some(ref current_worker) = job.worker_id {
                    if current_worker == &worker_id_for_closure {
                        // Already running with same worker, no-op
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

        debug!(job_id = %id, worker_id = worker_id_clone, "job marked as started");
        Ok(())
    }

    /// Update job heartbeat.
    ///
    /// Workers should call this periodically during job execution to indicate
    /// the job is still being processed. This is used by recovery mechanisms
    /// to detect orphaned jobs after leader failover.
    ///
    /// # Arguments
    ///
    /// * `id` - The job ID
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the heartbeat could not be updated.
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

    /// Process scheduled jobs.
    pub async fn process_scheduled(&self) -> Result<usize> {
        // Ensure queues are initialized before processing scheduled jobs
        self.ensure_initialized().await?;
        let now = Utc::now();
        let scheduled = self.get_scheduled_jobs(now).await?;
        let mut processed = 0;

        for job_id in scheduled.iter().take(self.config.max_schedule_per_tick) {
            if let Some(mut job) = self.get_job(job_id).await? {
                if let Some(scheduled_at) = job.scheduled_at {
                    if scheduled_at <= now {
                        // Remove from schedule index
                        self.remove_from_schedule_index(job_id, scheduled_at).await?;

                        // Check if it's a recurring job
                        if let Some(Schedule::Recurring(ref cron_expr)) = job.spec.schedule {
                            // Calculate next execution time
                            use std::str::FromStr;
                            if let Ok(schedule) = cron::Schedule::from_str(cron_expr) {
                                if let Some(next) = schedule.upcoming(Utc).next() {
                                    job.scheduled_at = Some(next);
                                    self.add_to_schedule_index(job_id, next).await?;
                                }
                            }
                        } else {
                            job.status = JobStatus::Pending;
                            job.scheduled_at = None;
                        }

                        // Enqueue for processing
                        self.enqueue_job(&job).await?;
                        self.store_job(&job).await?;

                        processed += 1;
                        debug!(job_id = %job_id, "scheduled job enqueued");
                    }
                }
            }
        }

        Ok(processed)
    }

    /// Get queue statistics.
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        // Ensure queues are initialized before getting stats
        self.ensure_initialized().await?;
        let mut stats = QueueStats::default();
        let mut dlq_stats = DLQStats::default();

        for (priority, queue_manager) in &self.queue_managers {
            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            let queue_status =
                queue_manager.status(&queue_name).await.map_err(|e| JobError::QueueError { source: e })?;

            stats.by_priority.insert(*priority, queue_status.visible_count);
            stats.total_queued += queue_status.visible_count;
            stats.processing += queue_status.pending_count;

            // Get DLQ count for this priority
            dlq_stats.by_priority.insert(*priority, queue_status.dlq_count);
            dlq_stats.total_count += queue_status.dlq_count;
        }

        stats.depth = stats.total_queued;
        stats.dlq_stats = dlq_stats;
        Ok(stats)
    }

    /// Get job type statistics.
    pub async fn get_job_type_stats(&self, _job_type: &str) -> Result<JobTypeStats> {
        // This would scan through job history and calculate stats
        // For now, return placeholder
        Ok(JobTypeStats {
            total_submitted: 0,
            pending: 0,
            running: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            avg_execution_time_ms: 0,
            success_rate: 0.0,
        })
    }

    /// Get jobs from the Dead Letter Queue.
    ///
    /// Returns jobs from DLQ with optional filtering by priority.
    pub async fn get_dlq_jobs(&self, priority: Option<Priority>, limit: u32) -> Result<Vec<Job>> {
        // Ensure queues are initialized
        self.ensure_initialized().await?;
        let mut dlq_jobs = Vec::new();

        let priorities_to_check = if let Some(p) = priority {
            vec![p]
        } else {
            Priority::all_ordered()
        };

        for priority in priorities_to_check {
            if dlq_jobs.len() >= limit as usize {
                break;
            }

            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            if let Some(queue_manager) = self.queue_managers.get(&priority) {
                let remaining_limit = (limit as usize - dlq_jobs.len()) as u32;

                let dlq_items = queue_manager
                    .get_dlq(&queue_name, remaining_limit)
                    .await
                    .map_err(|e| JobError::QueueError { source: e })?;

                for item in dlq_items {
                    // Parse job ID from payload
                    let job_id_str = String::from_utf8(item.payload.clone()).map_err(|_| JobError::InvalidJobSpec {
                        reason: "invalid job ID in DLQ payload".to_string(),
                    })?;
                    let job_id = JobId::from_string(job_id_str);

                    // Retrieve job from storage
                    if let Some(job) = self.get_job(&job_id).await? {
                        dlq_jobs.push(job);
                    }
                }
            }
        }

        Ok(dlq_jobs)
    }

    /// Redrive a job from the Dead Letter Queue back to the main queue.
    ///
    /// This resets the job's attempts counter and moves it back to pending status.
    pub async fn redrive_job(&self, job_id: &JobId) -> Result<()> {
        // Ensure queues are initialized
        self.ensure_initialized().await?;
        // First, update the job status
        self.atomic_update_job(job_id, |job| {
            if job.status != JobStatus::DeadLetter {
                return Err(JobError::InvalidJobState {
                    state: format!("{:?}", job.status),
                    operation: "redrive_job".to_string(),
                });
            }

            // Prepare job for redrive
            job.prepare_for_redrive();
            Ok(())
        })
        .await?;

        // Get the updated job
        let job = self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

        // Find the DLQ item in the queue system and redrive it
        let priority = job.spec.config.priority;
        let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());

        if let Some(queue_manager) = self.queue_managers.get(&priority) {
            // Get the numeric item ID from the job ID
            // This assumes the job ID contains the original queue item ID
            // In a real implementation, we'd need to store this mapping
            let item_id: u64 = job_id.as_str().split('-').next().and_then(|s| s.parse().ok()).unwrap_or(0);

            queue_manager
                .redrive_dlq(&queue_name, item_id)
                .await
                .map_err(|e| JobError::QueueError { source: e })?;
        }

        // Re-enqueue the job for processing
        self.enqueue_job(&job).await?;

        info!(
            job_id = %job_id,
            "job redriven from DLQ"
        );

        Ok(())
    }

    /// Redrive multiple jobs from the DLQ with filtering options.
    pub async fn redrive_jobs_batch(
        &self,
        filter_priority: Option<Priority>,
        filter_job_type: Option<String>,
        max_jobs: u32,
    ) -> Result<u32> {
        let dlq_jobs = self.get_dlq_jobs(filter_priority, max_jobs).await?;
        let mut redriven_count = 0;

        for job in dlq_jobs {
            // Apply job type filter if specified
            if let Some(ref job_type) = filter_job_type {
                if job.spec.job_type != *job_type {
                    continue;
                }
            }

            match self.redrive_job(&job.id).await {
                Ok(_) => redriven_count += 1,
                Err(e) => {
                    warn!(
                        job_id = %job.id,
                        error = %e,
                        "failed to redrive job from DLQ"
                    );
                }
            }

            // Limit the number of redriven jobs
            if redriven_count >= max_jobs {
                break;
            }
        }

        Ok(redriven_count)
    }

    /// Purge a job from the Dead Letter Queue permanently.
    ///
    /// This removes the job from DLQ without reprocessing it.
    pub async fn purge_dlq_job(&self, job_id: &JobId) -> Result<()> {
        let job = self.get_job(job_id).await?.ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

        if job.status != JobStatus::DeadLetter {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", job.status),
                operation: "purge_dlq_job".to_string(),
            });
        }

        // Delete the job from storage
        let key = format!("{}{}", JOB_PREFIX, job_id.as_str());
        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        info!(
            job_id = %job_id,
            "job purged from DLQ"
        );

        Ok(())
    }

    /// Purge all jobs from the Dead Letter Queue.
    pub async fn purge_all_dlq(&self) -> Result<u32> {
        let dlq_jobs = self.get_dlq_jobs(None, 10000).await?;
        let mut purged_count = 0;

        for job in dlq_jobs {
            match self.purge_dlq_job(&job.id).await {
                Ok(_) => purged_count += 1,
                Err(e) => {
                    warn!(
                        job_id = %job.id,
                        error = %e,
                        "failed to purge job from DLQ"
                    );
                }
            }
        }

        Ok(purged_count)
    }

    // =========================================================================
    // Dependency Management
    // =========================================================================

    /// Submit a workflow (DAG of jobs).
    /// Jobs are topologically sorted and submitted in order.
    pub async fn submit_workflow(&self, mut specs: Vec<JobSpec>) -> Result<Vec<JobId>> {
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

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Store a job to the database.
    async fn store_job(&self, job: &Job) -> Result<()> {
        let key = format!("{}{}", JOB_PREFIX, job.id.as_str());
        let value = serde_json::to_string(job).map_err(|e| JobError::SerializationError { source: e })?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Check if a job exists in storage.
    async fn job_exists(&self, job_id: &JobId) -> Result<bool> {
        let key = format!("{}{}", JOB_PREFIX, job_id.as_str());
        match self.store.read(ReadRequest::new(key)).await {
            Ok(result) => Ok(result.kv.is_some()),
            Err(_) => Ok(false),
        }
    }

    /// Atomically update a job using compare-and-swap.
    /// Returns the updated job if successful, or an error if the version doesn't match.
    async fn atomic_update_job<F>(&self, id: &JobId, mut update_fn: F) -> Result<Job>
    where F: FnMut(&mut Job) -> Result<()> {
        const MAX_RETRIES: u32 = 3;
        let mut retries = 0;

        loop {
            // Read current job
            let mut job = self.get_job(id).await?.ok_or_else(|| JobError::JobNotFound { id: id.to_string() })?;

            let expected_version = job.version;

            // Apply update
            update_fn(&mut job)?;

            // Try to store with CAS
            let key = format!("{}{}", JOB_PREFIX, id.as_str());
            let new_value = serde_json::to_string(&job).map_err(|e| JobError::SerializationError { source: e })?;

            // Read current value to check version
            let current = self
                .store
                .read(ReadRequest::new(key.clone()))
                .await
                .map_err(|e| JobError::StorageError { source: e })?;

            if let Some(kv) = current.kv {
                let current_job: Job =
                    serde_json::from_str(&kv.value).map_err(|e| JobError::SerializationError { source: e })?;

                if current_job.version != expected_version {
                    // Version mismatch, retry if we haven't exceeded retries
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        return Err(JobError::InvalidJobState {
                            state: format!(
                                "version mismatch: expected {}, got {}",
                                expected_version, current_job.version
                            ),
                            operation: "atomic_update".to_string(),
                        });
                    }
                    // Small delay before retry
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * retries as u64)).await;
                    continue;
                }
            }

            // Version matches, proceed with write
            self.store
                .write(WriteRequest {
                    command: WriteCommand::Set { key, value: new_value },
                })
                .await
                .map_err(|e| JobError::StorageError { source: e })?;

            return Ok(job);
        }
    }

    /// Enqueue a job to the appropriate priority queue.
    async fn enqueue_job(&self, job: &Job) -> Result<()> {
        // Check dependency state first
        if !job.dependency_state.is_ready() {
            return Err(JobError::InvalidJobState {
                state: format!("Dependencies not satisfied: {:?}", job.dependency_state),
                operation: "enqueue".to_string(),
            });
        }

        // Validate that job is in a state that can be enqueued
        match job.status {
            JobStatus::Pending | JobStatus::Scheduled | JobStatus::Retrying => {
                // These are valid states for enqueueing
            }
            JobStatus::Running => {
                // Job is already being processed, should not re-enqueue
                warn!(
                    job_id = %job.id,
                    "attempted to enqueue job that is already running"
                );
                return Err(JobError::InvalidJobState {
                    state: format!("{:?}", job.status),
                    operation: "enqueue".to_string(),
                });
            }
            status if status.is_terminal() => {
                // Job is in terminal state, should not enqueue
                return Err(JobError::InvalidJobState {
                    state: format!("{:?}", status),
                    operation: "enqueue".to_string(),
                });
            }
            _ => {
                // Other states, log warning but proceed
                warn!(
                    job_id = %job.id,
                    status = ?job.status,
                    "enqueueing job with unexpected status"
                );
            }
        }

        let priority = job.spec.config.priority;
        let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());

        let queue_manager = self.queue_managers.get(&priority).ok_or_else(|| JobError::InvalidJobSpec {
            reason: format!("invalid priority: {:?}", priority),
        })?;

        // Serialize job ID as payload
        let payload = job.id.as_str().as_bytes().to_vec();

        let options = EnqueueOptions {
            ttl_ms: job.spec.config.ttl_after_completion.map(|d| d.as_millis() as u64),
            message_group_id: Some(job.spec.job_type.clone()),
            deduplication_id: job.spec.idempotency_key.clone(),
        };

        queue_manager
            .enqueue(&queue_name, payload, options)
            .await
            .map_err(|e| JobError::QueueError { source: e })?;

        Ok(())
    }

    /// Check if an idempotency key already exists.
    async fn check_idempotency_key(&self, key: &str) -> Result<Option<JobId>> {
        let storage_key = format!("{}idempotency:{}", JOB_PREFIX, key);

        match self.store.read(ReadRequest::new(storage_key)).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    Ok(Some(JobId::from_string(kv.value)))
                } else {
                    Ok(None)
                }
            }
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(JobError::StorageError { source: e }),
        }
    }

    /// Store an idempotency key.
    async fn store_idempotency_key(&self, key: &str, job_id: &JobId) -> Result<()> {
        let storage_key = format!("{}idempotency:{}", JOB_PREFIX, key);
        let value = job_id.to_string();

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: storage_key,
                    value,
                },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Add a job to the schedule index.
    async fn add_to_schedule_index(&self, job_id: &JobId, scheduled_at: DateTime<Utc>) -> Result<()> {
        let key = format!("{}{}:{}", JOB_SCHEDULE_PREFIX, scheduled_at.timestamp(), job_id.as_str());
        let value = job_id.to_string();

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Remove a job from the schedule index.
    async fn remove_from_schedule_index(&self, job_id: &JobId, scheduled_at: DateTime<Utc>) -> Result<()> {
        let key = format!("{}{}:{}", JOB_SCHEDULE_PREFIX, scheduled_at.timestamp(), job_id.as_str());

        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Get jobs scheduled up to a given time.
    async fn get_scheduled_jobs(&self, up_to: DateTime<Utc>) -> Result<Vec<JobId>> {
        let prefix = JOB_SCHEDULE_PREFIX;
        let end_key = format!("{}{}", JOB_SCHEDULE_PREFIX, up_to.timestamp());

        // Scan for scheduled jobs
        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: prefix.to_string(),
                limit: Some(self.config.max_schedule_per_tick as u32),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut job_ids = Vec::new();
        for entry in scan_result.entries {
            if entry.key <= end_key {
                // Extract job ID from key
                if let Some(job_id_str) = entry.key.rsplit(':').next() {
                    job_ids.push(JobId::from_string(job_id_str.to_string()));
                }
            }
        }

        Ok(job_ids)
    }
}
