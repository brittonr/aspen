//! Job command service - handles mutations (writes)
//!
//! This service is responsible for all job state modifications following the
//! Command Query Responsibility Segregation (CQRS) pattern. Commands change
//! state but don't return domain data (only success/failure and identifiers).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Result;

use crate::repositories::WorkRepository;
use crate::domain::types::JobStatus;
use crate::domain::events::{DomainEvent, EventPublisher, current_timestamp};
use crate::domain::event_publishers::NoOpEventPublisher;
use crate::domain::state_machine::JobStateMachine;

/// Job submission parameters
#[derive(Debug, Clone)]
pub struct JobSubmission {
    pub payload: serde_json::Value,
}

/// Command service for job mutations
///
/// Handles all write operations for jobs:
/// - Creating new jobs
/// - Claiming jobs for execution
/// - Updating job status
/// - Canceling jobs
///
/// This service follows the CQRS pattern by separating writes from reads.
/// It publishes domain events after successful state mutations.
pub struct JobCommandService {
    work_repo: Arc<dyn WorkRepository>,
    job_counter: Arc<AtomicUsize>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl JobCommandService {
    /// Create a new job command service (no events)
    ///
    /// Uses NoOpEventPublisher by default. For event publishing,
    /// use `with_events()` instead.
    pub fn new(work_repo: Arc<dyn WorkRepository>) -> Self {
        Self::with_events(work_repo, Arc::new(NoOpEventPublisher::new()))
    }

    /// Create a new job command service with event publishing
    ///
    /// # Arguments
    /// * `work_repo` - Repository for job storage
    /// * `event_publisher` - Publisher for domain events
    pub fn with_events(
        work_repo: Arc<dyn WorkRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            work_repo,
            job_counter: Arc::new(AtomicUsize::new(1)),
            event_publisher,
        }
    }

    /// Submit a new job to the queue
    ///
    /// This is a **command** - it creates a new job and returns only the job ID.
    /// For querying job details, use JobQueryService.
    ///
    /// # Arguments
    /// * `submission` - Job submission parameters with generic payload
    ///
    /// # Returns
    /// The unique job ID for the created job
    ///
    /// # Errors
    /// Returns error if validation fails or the repository operation fails
    pub async fn submit_job(&self, submission: JobSubmission) -> Result<String> {
        // Validate payload (basic validation)
        if submission.payload.is_null() {
            anyhow::bail!("Payload cannot be null");
        }

        // Generate unique job ID
        let id = self.job_counter.fetch_add(1, Ordering::SeqCst);
        let job_id = format!("job-{}", id);

        // Add job metadata to payload
        let mut payload = submission.payload.clone();
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("job_id".to_string(), serde_json::json!(job_id.clone()));
            obj.insert("submission_timestamp".to_string(), serde_json::json!(current_timestamp()));
        }

        // Publish to distributed queue
        self.work_repo.publish_work(job_id.clone(), payload.clone()).await?;

        tracing::info!(job_id = %job_id, "Job submitted with payload");

        // Publish domain event
        self.event_publisher.publish(DomainEvent::JobSubmitted {
            job_id: job_id.clone(),
            payload: payload,
            timestamp: current_timestamp(),
        }).await?;

        Ok(job_id)
    }

    /// Claim the next available job for execution
    ///
    /// This is a **command** - it mutates the job state to "Claimed".
    /// Workers call this to take ownership of pending work.
    ///
    /// # Arguments
    /// * `worker_id` - Optional worker ID to assign the job to
    /// * `worker_type` - Optional worker type for filtering compatible jobs
    ///
    /// # Returns
    /// The job ID if a job was claimed, None if no work is available
    pub async fn claim_job(&self, worker_id: Option<&str>, worker_type: Option<crate::domain::types::WorkerType>) -> Result<Option<String>> {
        match self.work_repo.claim_work(worker_id, worker_type).await? {
            Some(job) => {
                tracing::info!(job_id = %job.id, worker_id = ?worker_id, worker_type = ?worker_type, "Job claimed");

                // Publish domain event
                self.event_publisher.publish(DomainEvent::JobClaimed {
                    job_id: job.id.clone(),
                    worker_id: job.claimed_by.clone().unwrap_or_else(|| "unknown".to_string()),
                    timestamp: current_timestamp(),
                }).await?;

                Ok(Some(job.id))
            }
            None => Ok(None),
        }
    }

    /// Update job status
    ///
    /// This is a **command** - it mutates the job state.
    /// Workers call this to report progress (InProgress, Completed, Failed).
    ///
    /// # Arguments
    /// * `job_id` - The job to update
    /// * `status` - The new status
    /// * `error_message` - Optional error message (required when status is Failed)
    ///
    /// # Errors
    /// Returns error if the job doesn't exist or the status transition is invalid
    pub async fn update_job_status(
        &self,
        job_id: &str,
        status: JobStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        // Get current job to validate transition
        let jobs = self.work_repo.list_work().await?;
        let current_job = jobs
            .iter()
            .find(|j| j.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))?;

        let old_status = current_job.status;

        // Validate state transition using business rules
        JobStateMachine::validate_transition(old_status, status)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Store error message if provided (typically for Failed status)
        if let Some(error_msg) = error_message.clone() {
            // We'll need to update the full job - for now just update status
            // The error_message will be captured in a future enhancement
            tracing::warn!(job_id = %job_id, error = %error_msg, "Error message provided but not yet persisted to database");
        }

        // Transition is valid - update in repository
        self.work_repo.update_status(job_id, status.clone()).await?;

        tracing::info!(job_id = %job_id, status = ?status, "Job status updated");

        // Fetch updated job to get current field values
        let updated_job = self.work_repo.find_by_id(job_id).await?
            .ok_or_else(|| anyhow::anyhow!("Job not found after status update: {}", job_id))?;

        // Publish appropriate domain event based on new status
        let event = match status {
            JobStatus::Completed => DomainEvent::JobCompleted {
                job_id: job_id.to_string(),
                worker_id: updated_job.completed_by.clone(),
                duration_ms: updated_job.duration_ms() as u64,
                timestamp: current_timestamp(),
            },
            JobStatus::Failed => DomainEvent::JobFailed {
                job_id: job_id.to_string(),
                worker_id: updated_job.completed_by.clone(),
                error: updated_job.error_message.clone().unwrap_or_else(|| "Job failed".to_string()),
                timestamp: current_timestamp(),
            },
            _ => {
                // Generic status change event (always have old_status now)
                DomainEvent::JobStatusChanged {
                    job_id: job_id.to_string(),
                    old_status,
                    new_status: status,
                    timestamp: current_timestamp(),
                }
            }
        };

        self.event_publisher.publish(event).await?;

        Ok(())
    }

    /// Cancel a pending job
    ///
    /// This is a **command** - it transitions a job to Failed status.
    /// Can only cancel jobs that haven't started execution.
    ///
    /// # Arguments
    /// * `job_id` - The job to cancel
    ///
    /// # Errors
    /// Returns error if the job is already in progress or completed
    #[allow(dead_code)] // Future API feature
    pub async fn cancel_job(&self, job_id: &str) -> Result<()> {
        // Get current job to validate cancellation
        let jobs = self.work_repo.list_work().await?;
        let current_job = jobs
            .iter()
            .find(|j| j.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))?;

        // Validate state transition (Pending/Claimed -> Failed)
        JobStateMachine::validate_transition(current_job.status, JobStatus::Failed)
            .map_err(|e| anyhow::anyhow!("Cannot cancel job: {}", e))?;

        self.work_repo.update_status(job_id, JobStatus::Failed).await?;

        tracing::info!(job_id = %job_id, "Job cancelled");

        // Publish cancellation event
        self.event_publisher.publish(DomainEvent::JobCancelled {
            job_id: job_id.to_string(),
            reason: "User requested cancellation".to_string(),
            timestamp: current_timestamp(),
        }).await?;

        Ok(())
    }

    /// Retry a failed job
    ///
    /// This is a **command** - it resets a failed job to Pending status.
    /// Allows failed jobs to be retried by workers.
    ///
    /// # Arguments
    /// * `job_id` - The failed job to retry
    ///
    /// # Errors
    /// Returns error if the job is not in Failed status
    #[allow(dead_code)] // Future API feature
    pub async fn retry_job(&self, job_id: &str) -> Result<()> {
        // Get current job to validate retry
        let jobs = self.work_repo.list_work().await?;
        let current_job = jobs
            .iter()
            .find(|j| j.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))?;

        // Validate state transition (Failed -> Pending)
        JobStateMachine::validate_transition(current_job.status, JobStatus::Pending)
            .map_err(|e| anyhow::anyhow!("Cannot retry job: {}", e))?;

        // Increment retry count
        let new_retry_count = current_job.retry_count + 1;

        // Update status and retry count
        self.work_repo.update_status(job_id, JobStatus::Pending).await?;
        // Note: We need to also update the retry count in the repository
        // For now, we'll track it here for the event

        tracing::info!(
            job_id = %job_id,
            retry_count = new_retry_count,
            "Job reset for retry"
        );

        // Publish retry event with actual retry count
        self.event_publisher.publish(DomainEvent::JobRetried {
            job_id: job_id.to_string(),
            retry_count: new_retry_count as usize,
            timestamp: current_timestamp(),
        }).await?;

        Ok(())
    }
}
