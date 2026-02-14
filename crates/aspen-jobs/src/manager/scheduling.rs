//! Job scheduling logic for the job manager.

use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use chrono::DateTime;
use chrono::Utc;
use tracing::debug;

use super::JOB_SCHEDULE_PREFIX;
use super::JobManager;
use crate::error::JobError;
use crate::error::Result;
use crate::job::JobId;
use crate::job::JobStatus;
use crate::types::Schedule;

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
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

    /// Add a job to the schedule index.
    pub(crate) async fn add_to_schedule_index(&self, job_id: &JobId, scheduled_at: DateTime<Utc>) -> Result<()> {
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
    pub(crate) async fn remove_from_schedule_index(&self, job_id: &JobId, scheduled_at: DateTime<Utc>) -> Result<()> {
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
    pub(crate) async fn get_scheduled_jobs(&self, up_to: DateTime<Utc>) -> Result<Vec<JobId>> {
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
