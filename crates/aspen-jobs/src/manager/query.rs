//! Query operations for the job manager.

use aspen_kv_types::ReadRequest;
use aspen_traits::KeyValueStore;

use super::JOB_PREFIX;
use super::JobManager;
use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobStatus;
use crate::types::DLQStats;
use crate::types::JobTypeStats;
use crate::types::QueueStats;

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
    /// Get a job by ID.
    pub async fn get_job(&self, id: &JobId) -> Result<Option<Job>> {
        // Tiger Style: job ID must not be empty
        debug_assert!(!id.as_str().is_empty(), "job ID must not be empty");

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

    /// Get job status.
    pub async fn get_status(&self, id: &JobId) -> Result<JobStatus> {
        let job = self.get_job(id).await?.ok_or_else(|| JobError::JobNotFound { id: id.to_string() })?;

        Ok(job.status)
    }

    /// Get queue statistics.
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        // Ensure queues are initialized before getting stats
        self.ensure_initialized().await?;

        // Tiger Style: queue_managers must not be empty after initialization
        debug_assert!(!self.queue_managers.is_empty(), "queue_managers must be populated after initialization");

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
}
