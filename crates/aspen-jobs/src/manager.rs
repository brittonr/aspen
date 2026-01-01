//! Job manager for submitting and managing jobs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use aspen_coordination::{
    EnqueueOptions, QueueConfig, QueueManager, ServiceRegistry,
};
use aspen_core::{KeyValueStore, ReadRequest, WriteCommand, WriteRequest};

use crate::error::{JobError, JobErrorKind, Result};
use crate::job::{Job, JobId, JobResult, JobSpec, JobStatus};
use crate::types::{JobTypeStats, Priority, QueueStats, Schedule};

/// Job storage key prefix.
const JOB_PREFIX: &str = "__jobs:";
/// Job index prefix.
const JOB_INDEX_PREFIX: &str = "__jobs:index:";
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
    service_registry: ServiceRegistry<S>,
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

        Self {
            store,
            queue_managers,
            config,
            service_registry,
        }
    }

    /// Initialize the job system (create queues).
    pub async fn initialize(&self) -> Result<()> {
        // Create a queue for each priority level
        for priority in Priority::all_ordered() {
            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            let queue_config = QueueConfig {
                default_visibility_timeout_ms: Some(
                    self.config.default_visibility_timeout.as_millis() as u64
                ),
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

        // Enqueue job for immediate processing
        self.enqueue_job(&job).await?;

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
            "job submitted"
        );

        Ok(job.id)
    }

    /// Get a job by ID.
    pub async fn get_job(&self, id: &JobId) -> Result<Option<Job>> {
        let key = format!("{}{}", JOB_PREFIX, id.as_str());

        match self.store.read(ReadRequest::new(key)).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    let job: Job = serde_json::from_str(&kv.value)
                        .map_err(|e| JobError::SerializationError { source: e })?;
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
        let mut job = self
            .get_job(id)
            .await?
            .ok_or_else(|| JobError::JobNotFound {
                id: id.to_string(),
            })?;

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
        let job = self
            .get_job(id)
            .await?
            .ok_or_else(|| JobError::JobNotFound {
                id: id.to_string(),
            })?;

        Ok(job.status)
    }

    /// Update job progress.
    pub async fn update_progress(
        &self,
        id: &JobId,
        progress: u8,
        message: Option<String>,
    ) -> Result<()> {
        let mut job = self
            .get_job(id)
            .await?
            .ok_or_else(|| JobError::JobNotFound {
                id: id.to_string(),
            })?;

        if job.status != JobStatus::Running {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", job.status),
                operation: "update_progress".to_string(),
            });
        }

        job.update_progress(progress, message);
        self.store_job(&job).await?;

        Ok(())
    }

    /// Mark a job as started.
    pub async fn mark_started(&self, id: &JobId, worker_id: String) -> Result<()> {
        let mut job = self
            .get_job(id)
            .await?
            .ok_or_else(|| JobError::JobNotFound {
                id: id.to_string(),
            })?;

        if !job.can_execute_now() {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", job.status),
                operation: "mark_started".to_string(),
            });
        }

        job.mark_started(worker_id);
        self.store_job(&job).await?;

        debug!(job_id = %id, "job marked as started");
        Ok(())
    }

    /// Mark a job as completed.
    pub async fn mark_completed(&self, id: &JobId, result: JobResult) -> Result<()> {
        let mut job = self
            .get_job(id)
            .await?
            .ok_or_else(|| JobError::JobNotFound {
                id: id.to_string(),
            })?;

        if job.status != JobStatus::Running && job.status != JobStatus::Retrying {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", job.status),
                operation: "mark_completed".to_string(),
            });
        }

        let is_success = result.is_success();
        job.mark_completed(result);
        self.store_job(&job).await?;

        if is_success {
            info!(job_id = %id, "job completed successfully");
        } else {
            warn!(job_id = %id, "job failed");
        }

        Ok(())
    }

    /// Process scheduled jobs.
    pub async fn process_scheduled(&self) -> Result<usize> {
        let now = Utc::now();
        let scheduled = self.get_scheduled_jobs(now).await?;
        let mut processed = 0;

        for job_id in scheduled.iter().take(self.config.max_schedule_per_tick) {
            if let Some(mut job) = self.get_job(job_id).await? {
                if let Some(scheduled_at) = job.scheduled_at {
                    if scheduled_at <= now {
                        // Remove from schedule index
                        self.remove_from_schedule_index(job_id, scheduled_at)
                            .await?;

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
        let mut stats = QueueStats::default();

        for (priority, queue_manager) in &self.queue_managers {
            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            let queue_status = queue_manager
                .status(&queue_name)
                .await
                .map_err(|e| JobError::QueueError { source: e })?;

            stats.by_priority.insert(*priority, queue_status.visible_count);
            stats.total_queued += queue_status.visible_count;
            stats.processing += queue_status.pending_count;
        }

        stats.depth = stats.total_queued;
        Ok(stats)
    }

    /// Get job type statistics.
    pub async fn get_job_type_stats(&self, job_type: &str) -> Result<JobTypeStats> {
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

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Store a job to the database.
    async fn store_job(&self, job: &Job) -> Result<()> {
        let key = format!("{}{}", JOB_PREFIX, job.id.as_str());
        let value = serde_json::to_string(job)
            .map_err(|e| JobError::SerializationError { source: e })?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Enqueue a job to the appropriate priority queue.
    async fn enqueue_job(&self, job: &Job) -> Result<()> {
        let priority = job.spec.config.priority;
        let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());

        let queue_manager = self
            .queue_managers
            .get(&priority)
            .ok_or_else(|| JobError::InvalidJobSpec {
                reason: format!("invalid priority: {:?}", priority),
            })?;

        // Serialize job ID as payload
        let payload = job.id.as_str().as_bytes().to_vec();

        let options = EnqueueOptions {
            ttl_ms: job.spec.config.ttl_after_completion
                .map(|d| d.as_millis() as u64),
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
    async fn add_to_schedule_index(
        &self,
        job_id: &JobId,
        scheduled_at: DateTime<Utc>,
    ) -> Result<()> {
        let key = format!(
            "{}{}:{}",
            JOB_SCHEDULE_PREFIX,
            scheduled_at.timestamp(),
            job_id.as_str()
        );
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
    async fn remove_from_schedule_index(
        &self,
        job_id: &JobId,
        scheduled_at: DateTime<Utc>,
    ) -> Result<()> {
        let key = format!(
            "{}{}:{}",
            JOB_SCHEDULE_PREFIX,
            scheduled_at.timestamp(),
            job_id.as_str()
        );

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