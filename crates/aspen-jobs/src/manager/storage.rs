//! Internal storage helpers for the job manager.

use std::time::Duration;

use aspen_coordination::EnqueueOptions;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::warn;

use super::JOB_PREFIX;
use super::JobManager;
use super::MAX_NOT_LEADER_RETRIES;
use super::is_not_leader_error;
use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobStatus;

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
    /// Store a job to the database.
    pub(crate) async fn store_job(&self, job: &Job) -> Result<()> {
        // Tiger Style: job ID must not be empty
        assert!(!job.id.as_str().is_empty(), "cannot store job with empty ID");

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
    pub(crate) async fn job_exists(&self, job_id: &JobId) -> Result<bool> {
        let key = format!("{}{}", JOB_PREFIX, job_id.as_str());
        match self.store.read(ReadRequest::new(key)).await {
            Ok(result) => Ok(result.kv.is_some()),
            Err(_) => Ok(false),
        }
    }

    /// Atomically update a job using compare-and-swap.
    /// Returns the updated job if successful, or an error if the version doesn't match.
    pub(crate) async fn atomic_update_job<F>(&self, id: &JobId, mut update_fn: F) -> Result<Job>
    where F: FnMut(&mut Job) -> Result<()> {
        const MAX_RETRIES: u32 = 3;
        let mut retries = 0;
        let mut not_leader_retries = 0u32;

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
            let current = match self.store.read(ReadRequest::new(key.clone())).await {
                Ok(result) => result,
                Err(e) => {
                    // Handle NotLeader errors with exponential backoff
                    if let Some(leader) = is_not_leader_error(&e) {
                        not_leader_retries += 1;
                        if not_leader_retries >= MAX_NOT_LEADER_RETRIES {
                            return Err(JobError::NotLeader { leader });
                        }
                        // Exponential backoff: 1ms, 2ms, 4ms, ..., capped at 128ms
                        let delay_ms = 1u64 << not_leader_retries.min(7);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    return Err(JobError::StorageError { source: e });
                }
            };

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
                    tokio::time::sleep(Duration::from_millis(10 * retries as u64)).await;
                    continue;
                }
            }

            // Version matches, proceed with write
            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Set { key, value: new_value },
                })
                .await
            {
                Ok(_) => return Ok(job),
                Err(e) => {
                    // Handle NotLeader errors with exponential backoff
                    if let Some(leader) = is_not_leader_error(&e) {
                        not_leader_retries += 1;
                        if not_leader_retries >= MAX_NOT_LEADER_RETRIES {
                            return Err(JobError::NotLeader { leader });
                        }
                        // Exponential backoff: 1ms, 2ms, 4ms, ..., capped at 128ms
                        let delay_ms = 1u64 << not_leader_retries.min(7);
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    return Err(JobError::StorageError { source: e });
                }
            }
        }
    }

    /// Enqueue a job to the appropriate priority queue.
    pub(crate) async fn enqueue_job(&self, job: &Job) -> Result<()> {
        // Tiger Style: job ID and type must not be empty
        assert!(!job.id.as_str().is_empty(), "cannot enqueue job with empty ID");
        assert!(!job.spec.job_type.is_empty(), "cannot enqueue job with empty job type");

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
    pub(crate) async fn check_idempotency_key(&self, key: &str) -> Result<Option<JobId>> {
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
    pub(crate) async fn store_idempotency_key(&self, key: &str, job_id: &JobId) -> Result<()> {
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
}
