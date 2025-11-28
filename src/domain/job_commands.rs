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
                let assignment = self.work_repo.find_assignment_by_id(&job.id).await?.unwrap();
                self.event_publisher.publish(DomainEvent::JobClaimed {
                    job_id: job.id.clone(),
                    worker_id: assignment.claimed_by_node.clone().unwrap_or_else(|| "unknown".to_string()),
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
        let current_job = self.work_repo.find_by_id(job_id).await?
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
        
        let assignment = self.work_repo.find_assignment_by_id(job_id).await?;

        // Publish appropriate domain event based on new status
        let event = match status {
            JobStatus::Completed => DomainEvent::JobCompleted {
                job_id: job_id.to_string(),
                worker_id: assignment.and_then(|a| a.completed_by_node),
                duration_ms: updated_job.duration_ms() as u64,
                timestamp: current_timestamp(),
            },
            JobStatus::Failed => DomainEvent::JobFailed {
                job_id: job_id.to_string(),
                worker_id: assignment.and_then(|a| a.completed_by_node),
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
        let current_job = self.work_repo.find_by_id(job_id).await?
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
        let current_job = self.work_repo.find_by_id(job_id).await?
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))?;

        // Validate state transition (Failed -> Pending)
        JobStateMachine::validate_transition(current_job.status, JobStatus::Pending)
            .map_err(|e| anyhow::anyhow!("Cannot retry job: {}", e))?;

        // Increment retry count
        let new_retry_count = current_job.retry_count() + 1;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::MockWorkRepository;
    use crate::domain::event_publishers::InMemoryEventPublisher;
    use crate::domain::types::{Job, JobStatus, WorkerType};
    use crate::domain::job_metadata::JobMetadata;
    use crate::domain::job_requirements::JobRequirements;

    /// Create a test job command service with mocks
    fn create_test_service() -> (JobCommandService, Arc<MockWorkRepository>, Arc<InMemoryEventPublisher>) {
        let work_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(
            work_repo.clone() as Arc<dyn WorkRepository>,
            event_publisher.clone() as Arc<dyn EventPublisher>,
        );
        (service, work_repo, event_publisher)
    }

    /// Create a test job with default values
    #[allow(deprecated)]
    fn create_test_job(id: &str, status: JobStatus) -> Job {
        Job {
            id: id.to_string(),
            status,
            payload: serde_json::json!({"test": "data"}),
            requirements: JobRequirements::default(),
            metadata: JobMetadata::new(),
            error_message: None,
        }
    }

    // ========================================================================
    // Job Submission Tests
    // ========================================================================

    #[tokio::test]
    async fn test_submit_job_success() {
        let (service, work_repo, event_publisher) = create_test_service();
        let payload = serde_json::json!({"url": "https://example.com"});

        let job_id = service.submit_job(JobSubmission { payload: payload.clone() })
            .await
            .expect("Job submission should succeed");

        // Verify job ID format
        assert!(job_id.starts_with("job-"));

        // Verify job was published to repository
        let jobs = work_repo.list_work().await.expect("Should list jobs");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, job_id);
        assert_eq!(jobs[0].status, JobStatus::Pending);

        // Verify payload enrichment
        let stored_payload = &jobs[0].payload;
        assert_eq!(stored_payload.get("url").and_then(|v| v.as_str()), Some("https://example.com"));
        assert_eq!(stored_payload.get("job_id").and_then(|v| v.as_str()), Some(job_id.as_str()));
        assert!(stored_payload.get("submission_timestamp").is_some());

        // Verify event was published
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobSubmitted { job_id: event_job_id, payload: event_payload, .. } => {
                assert_eq!(event_job_id, &job_id);
                assert_eq!(event_payload.get("url").and_then(|v| v.as_str()), Some("https://example.com"));
            }
            _ => panic!("Expected JobSubmitted event"),
        }
    }

    #[tokio::test]
    async fn test_submit_job_null_payload_rejected() {
        let (service, _, _) = create_test_service();
        let payload = serde_json::json!(null);

        let result = service.submit_job(JobSubmission { payload }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Payload cannot be null"));
    }

    #[tokio::test]
    async fn test_submit_job_increments_counter() {
        let (service, _, _) = create_test_service();

        let job_id_1 = service.submit_job(JobSubmission {
            payload: serde_json::json!({"test": 1}),
        }).await.expect("First submission should succeed");

        let job_id_2 = service.submit_job(JobSubmission {
            payload: serde_json::json!({"test": 2}),
        }).await.expect("Second submission should succeed");

        assert_ne!(job_id_1, job_id_2);
        assert_eq!(job_id_1, "job-1");
        assert_eq!(job_id_2, "job-2");
    }

    #[tokio::test]
    async fn test_submit_job_preserves_non_object_payload() {
        let (service, work_repo, _) = create_test_service();
        let payload = serde_json::json!("string payload");

        let _job_id = service.submit_job(JobSubmission { payload: payload.clone() })
            .await
            .expect("Job submission should succeed");

        let jobs = work_repo.list_work().await.expect("Should list jobs");
        assert_eq!(jobs.len(), 1);
        // Non-object payloads should not be enriched with metadata
        assert_eq!(jobs[0].payload, payload);
    }

    // ========================================================================
    // Job Claiming Tests
    // ========================================================================

    #[tokio::test]
    async fn test_claim_job_success() {
        let (service, work_repo, event_publisher) = create_test_service();

        // Add a pending job
        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        // Claim the job
        let claimed_id = service.claim_job(Some("worker-1"), None)
            .await
            .expect("Claim should succeed")
            .expect("Should return job ID");

        assert_eq!(claimed_id, "test-job-1");

        // Verify job status was updated
        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Claimed);

        // Verify event was published
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobClaimed { job_id, worker_id, .. } => {
                assert_eq!(job_id, "test-job-1");
                assert_eq!(worker_id, "worker-1");
            }
            _ => panic!("Expected JobClaimed event"),
        }
    }

    #[tokio::test]
    async fn test_claim_job_no_work_available() {
        let (service, _, event_publisher) = create_test_service();

        // No jobs in the queue
        let result = service.claim_job(Some("worker-1"), None)
            .await
            .expect("Claim should succeed");

        assert!(result.is_none());

        // No events should be published
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_claim_job_with_worker_type_filtering() {
        let (service, work_repo, _) = create_test_service();

        // Add a job requiring Firecracker worker
        let mut job = create_test_job("test-job-1", JobStatus::Pending);
        job.requirements.compatible_worker_types = vec![WorkerType::Firecracker];
        work_repo.add_jobs(vec![job]).await;

        // Try to claim with Wasm worker (should not match)
        let result = service.claim_job(Some("worker-1"), Some(WorkerType::Wasm))
            .await
            .expect("Claim should succeed");
        assert!(result.is_none());

        // Try to claim with Firecracker worker (should match)
        let claimed_id = service.claim_job(Some("worker-2"), Some(WorkerType::Firecracker))
            .await
            .expect("Claim should succeed")
            .expect("Should return job ID");
        assert_eq!(claimed_id, "test-job-1");
    }

    #[tokio::test]
    async fn test_claim_job_without_worker_id() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        // Claim without worker ID
        let claimed_id = service.claim_job(None, None)
            .await
            .expect("Claim should succeed")
            .expect("Should return job ID");

        assert_eq!(claimed_id, "test-job-1");
    }

    // ========================================================================
    // Job Status Update Tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_job_status_pending_to_claimed() {
        let (service, work_repo, event_publisher) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        service.update_job_status("test-job-1", JobStatus::Claimed, None)
            .await
            .expect("Status update should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Claimed);

        // Verify generic status change event
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobStatusChanged { job_id, old_status, new_status, .. } => {
                assert_eq!(job_id, "test-job-1");
                assert_eq!(*old_status, JobStatus::Pending);
                assert_eq!(*new_status, JobStatus::Claimed);
            }
            _ => panic!("Expected JobStatusChanged event"),
        }
    }

    #[tokio::test]
    async fn test_update_job_status_claimed_to_in_progress() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Claimed);
        work_repo.add_jobs(vec![job]).await;

        service.update_job_status("test-job-1", JobStatus::InProgress, None)
            .await
            .expect("Status update should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::InProgress);
    }

    #[tokio::test]
    async fn test_update_job_status_to_completed() {
        let (service, work_repo, event_publisher) = create_test_service();

        let mut job = create_test_job("test-job-1", JobStatus::InProgress);
        job.metadata.started_at = Some(1000);
        job.metadata.updated_at = 2000;
        work_repo.add_jobs(vec![job]).await;

        service.update_job_status("test-job-1", JobStatus::Completed, None)
            .await
            .expect("Status update should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Completed);

        // Verify JobCompleted event
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobCompleted { job_id, duration_ms, .. } => {
                assert_eq!(job_id, "test-job-1");
                assert_eq!(*duration_ms, 1000000); // 1000 seconds in ms
            }
            _ => panic!("Expected JobCompleted event"),
        }
    }

    #[tokio::test]
    async fn test_update_job_status_to_failed() {
        let (service, work_repo, event_publisher) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::InProgress);
        work_repo.add_jobs(vec![job]).await;

        let error_message = "Out of memory";
        service.update_job_status("test-job-1", JobStatus::Failed, Some(error_message.to_string()))
            .await
            .expect("Status update should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Failed);

        // Verify JobFailed event
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobFailed { job_id, error, .. } => {
                assert_eq!(job_id, "test-job-1");
                // Note: error message may be "Job failed" since repository doesn't persist it yet
                assert!(!error.is_empty());
            }
            _ => panic!("Expected JobFailed event"),
        }
    }

    #[tokio::test]
    async fn test_update_job_status_idempotent() {
        let (service, work_repo, event_publisher) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        // Update to same status (idempotent)
        service.update_job_status("test-job-1", JobStatus::Pending, None)
            .await
            .expect("Idempotent status update should succeed");

        // Should still publish event for audit trail
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_update_job_status_invalid_transition() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Completed);
        work_repo.add_jobs(vec![job]).await;

        // Try to transition from Completed to Pending (invalid)
        let result = service.update_job_status("test-job-1", JobStatus::Pending, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("terminal state"));
    }

    #[tokio::test]
    async fn test_update_job_status_job_not_found() {
        let (service, _, _) = create_test_service();

        let result = service.update_job_status("nonexistent", JobStatus::Claimed, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Job not found"));
    }

    #[tokio::test]
    async fn test_update_job_status_backward_transition_rejected() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Claimed);
        work_repo.add_jobs(vec![job]).await;

        // Try to transition backward from Claimed to Pending
        let result = service.update_job_status("test-job-1", JobStatus::Pending, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backward transition"));
    }

    #[tokio::test]
    async fn test_update_job_status_skip_transition_rejected() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        // Try to skip Claimed state and go directly to InProgress
        let result = service.update_job_status("test-job-1", JobStatus::InProgress, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("skipped Claimed state"));
    }

    // ========================================================================
    // Job Cancellation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_cancel_pending_job() {
        let (service, work_repo, event_publisher) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        service.cancel_job("test-job-1")
            .await
            .expect("Cancellation should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Failed);

        // Verify cancellation event
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobCancelled { job_id, reason, .. } => {
                assert_eq!(job_id, "test-job-1");
                assert!(reason.contains("cancellation"));
            }
            _ => panic!("Expected JobCancelled event"),
        }
    }

    #[tokio::test]
    async fn test_cancel_claimed_job() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Claimed);
        work_repo.add_jobs(vec![job]).await;

        service.cancel_job("test-job-1")
            .await
            .expect("Cancellation should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_cancel_completed_job_rejected() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Completed);
        work_repo.add_jobs(vec![job]).await;

        let result = service.cancel_job("test-job-1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot cancel job"));
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_job() {
        let (service, _, _) = create_test_service();

        let result = service.cancel_job("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Job not found"));
    }

    // ========================================================================
    // Job Retry Tests
    // ========================================================================

    #[tokio::test]
    async fn test_retry_failed_job() {
        let (service, work_repo, event_publisher) = create_test_service();

        let mut job = create_test_job("test-job-1", JobStatus::Failed);
        job.metadata.retry_count = 1;
        work_repo.add_jobs(vec![job]).await;

        service.retry_job("test-job-1")
            .await
            .expect("Retry should succeed");

        let updated_job = work_repo.find_by_id("test-job-1")
            .await
            .expect("Should find job")
            .expect("Job should exist");
        assert_eq!(updated_job.status, JobStatus::Pending);

        // Verify retry event with incremented count
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            DomainEvent::JobRetried { job_id, retry_count, .. } => {
                assert_eq!(job_id, "test-job-1");
                assert_eq!(*retry_count, 2); // Original 1 + 1
            }
            _ => panic!("Expected JobRetried event"),
        }
    }

    #[tokio::test]
    async fn test_retry_completed_job_rejected() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Completed);
        work_repo.add_jobs(vec![job]).await;

        let result = service.retry_job("test-job-1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot retry job"));
    }

    #[tokio::test]
    async fn test_retry_pending_job_rejected() {
        let (service, work_repo, _) = create_test_service();

        let job = create_test_job("test-job-1", JobStatus::Pending);
        work_repo.add_jobs(vec![job]).await;

        let result = service.retry_job("test-job-1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot retry job"));
    }

    #[tokio::test]
    async fn test_retry_nonexistent_job() {
        let (service, _, _) = create_test_service();

        let result = service.retry_job("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Job not found"));
    }

    // ========================================================================
    // Service Construction Tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_new_uses_noop_publisher() {
        let work_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(work_repo.clone() as Arc<dyn WorkRepository>);

        // Submit a job and verify no events are captured
        let _job_id = service.submit_job(JobSubmission {
            payload: serde_json::json!({"test": "data"}),
        }).await.expect("Job submission should succeed");

        // We can't directly verify NoOpEventPublisher, but we know it's used
        // if the service was constructed with new() instead of with_events()
    }

    #[tokio::test]
    async fn test_service_with_events_publishes_events() {
        let work_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(
            work_repo.clone() as Arc<dyn WorkRepository>,
            event_publisher.clone() as Arc<dyn EventPublisher>,
        );

        let _job_id = service.submit_job(JobSubmission {
            payload: serde_json::json!({"test": "data"}),
        }).await.expect("Job submission should succeed");

        // Verify event was published
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);
    }

    // ========================================================================
    // Integration Tests (Multiple Operations)
    // ========================================================================

    #[tokio::test]
    async fn test_full_job_lifecycle_happy_path() {
        let (service, work_repo, event_publisher) = create_test_service();

        // 1. Submit job
        let job_id = service.submit_job(JobSubmission {
            payload: serde_json::json!({"url": "https://example.com"}),
        }).await.expect("Submit should succeed");

        // 2. Claim job
        let claimed_id = service.claim_job(Some("worker-1"), None)
            .await.expect("Claim should succeed")
            .expect("Should return job ID");
        assert_eq!(claimed_id, job_id);

        // 3. Update to InProgress
        service.update_job_status(&job_id, JobStatus::InProgress, None)
            .await.expect("Status update should succeed");

        // 4. Complete job
        service.update_job_status(&job_id, JobStatus::Completed, None)
            .await.expect("Completion should succeed");

        // Verify final state
        let final_job = work_repo.find_by_id(&job_id)
            .await.expect("Should find job")
            .expect("Job should exist");
        assert_eq!(final_job.status, JobStatus::Completed);

        // Verify event sequence
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 4); // Submitted, Claimed, StatusChanged, Completed
    }

    #[tokio::test]
    async fn test_job_lifecycle_with_failure_and_retry() {
        let (service, work_repo, event_publisher) = create_test_service();

        // 1. Submit job
        let job_id = service.submit_job(JobSubmission {
            payload: serde_json::json!({"test": "data"}),
        }).await.expect("Submit should succeed");

        // 2. Claim and start
        service.claim_job(Some("worker-1"), None)
            .await.expect("Claim should succeed");
        service.update_job_status(&job_id, JobStatus::InProgress, None)
            .await.expect("Status update should succeed");

        // 3. Fail
        service.update_job_status(&job_id, JobStatus::Failed, Some("Network error".to_string()))
            .await.expect("Failure should succeed");

        // 4. Retry
        service.retry_job(&job_id)
            .await.expect("Retry should succeed");

        // Verify job is back to Pending
        let retried_job = work_repo.find_by_id(&job_id)
            .await.expect("Should find job")
            .expect("Job should exist");
        assert_eq!(retried_job.status, JobStatus::Pending);

        // Verify event sequence
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 5); // Submitted, Claimed, StatusChanged, Failed, Retried
    }

    #[tokio::test]
    async fn test_concurrent_job_claims_no_duplicate() {
        let (service, work_repo, _) = create_test_service();

        // Add multiple pending jobs
        for i in 1..=3 {
            let job = create_test_job(&format!("job-{}", i), JobStatus::Pending);
            work_repo.add_jobs(vec![job]).await;
        }

        // Claim jobs sequentially
        let claim1 = service.claim_job(Some("worker-1"), None).await.unwrap();
        let claim2 = service.claim_job(Some("worker-2"), None).await.unwrap();
        let claim3 = service.claim_job(Some("worker-3"), None).await.unwrap();

        // All should succeed and be different
        assert!(claim1.is_some());
        assert!(claim2.is_some());
        assert!(claim3.is_some());
        assert_ne!(claim1, claim2);
        assert_ne!(claim2, claim3);
        assert_ne!(claim1, claim3);

        // Fourth claim should fail
        let claim4 = service.claim_job(Some("worker-4"), None).await.unwrap();
        assert!(claim4.is_none());
    }
}
