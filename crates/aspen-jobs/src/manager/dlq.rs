//! Dead letter queue operations for the job manager.

use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::info;
use tracing::warn;

use super::JOB_PREFIX;
use super::JobManager;
use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobStatus;
use crate::types::Priority;

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
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
}
