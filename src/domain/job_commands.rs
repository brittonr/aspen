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
    pub url: String,
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
    /// * `submission` - Job submission parameters (URL, etc.)
    ///
    /// # Returns
    /// The unique job ID for the created job
    ///
    /// # Errors
    /// Returns error if validation fails or the repository operation fails
    pub async fn submit_job(&self, submission: JobSubmission) -> Result<String> {
        // Validate URL (basic validation)
        if submission.url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }

        // Generate unique job ID
        let id = self.job_counter.fetch_add(1, Ordering::SeqCst);
        let job_id = format!("job-{}", id);

        // Create job payload
        let payload = serde_json::json!({
            "id": id,
            "url": submission.url.clone(),
        });

        // Publish to distributed queue
        self.work_repo.publish_work(job_id.clone(), payload).await?;

        tracing::info!(job_id = %job_id, url = %submission.url, "Job submitted");

        // Publish domain event
        self.event_publisher.publish(DomainEvent::JobSubmitted {
            job_id: job_id.clone(),
            url: submission.url,
            timestamp: current_timestamp(),
        }).await?;

        Ok(job_id)
    }

    /// Claim the next available job for execution
    ///
    /// This is a **command** - it mutates the job state to "Claimed".
    /// Workers call this to take ownership of pending work.
    ///
    /// # Returns
    /// The job ID if a job was claimed, None if no work is available
    pub async fn claim_job(&self) -> Result<Option<String>> {
        match self.work_repo.claim_work().await? {
            Some(job) => {
                tracing::info!(job_id = %job.id, "Job claimed");

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

        // Future: Track retry count
        self.work_repo.update_status(job_id, JobStatus::Pending).await?;

        tracing::info!(job_id = %job_id, "Job reset for retry");

        // Publish retry event
        self.event_publisher.publish(DomainEvent::JobRetried {
            job_id: job_id.to_string(),
            retry_count: 0, // TODO: Track actual retry count
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

    #[tokio::test]
    async fn test_submit_job_validates_empty_url() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo);

        // Act
        let result = service.submit_job(JobSubmission {
            url: String::new(),
        }).await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "URL cannot be empty");
    }

    #[tokio::test]
    async fn test_submit_job_generates_unique_ids() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        // Act
        let job_id_1 = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        let job_id_2 = service.submit_job(JobSubmission {
            url: "https://example.org".to_string(),
        }).await.unwrap();

        // Assert
        assert_ne!(job_id_1, job_id_2);
        assert!(job_id_1.starts_with("job-"));
        assert!(job_id_2.starts_with("job-"));
    }

    #[tokio::test]
    async fn test_submit_job_publishes_to_repository() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        // Act
        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Assert - verify job was added to repository
        let jobs = mock_repo.list_work().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, job_id);
        assert_eq!(jobs[0].status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_claim_job_returns_job_id_when_available() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act
        let claimed_id = service.claim_job().await.unwrap();

        // Assert
        assert!(claimed_id.is_some());
    }

    #[tokio::test]
    async fn test_claim_job_returns_none_when_no_work() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo);

        // Act
        let claimed_id = service.claim_job().await.unwrap();

        // Assert
        assert!(claimed_id.is_none());
    }

    #[tokio::test]
    async fn test_update_job_status_changes_status() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act - Follow valid state transitions: Pending → Claimed → InProgress
        service.claim_job().await.unwrap(); // Claim first
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();

        // Assert
        let jobs = mock_repo.list_work().await.unwrap();
        let job = jobs.iter().find(|j| j.id == job_id).unwrap();
        assert_eq!(job.status, JobStatus::InProgress);
    }

    #[tokio::test]
    async fn test_cancel_job_sets_failed_status() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act
        service.cancel_job(&job_id).await.unwrap();

        // Assert
        let jobs = mock_repo.list_work().await.unwrap();
        let job = jobs.iter().find(|j| j.id == job_id).unwrap();
        assert_eq!(job.status, JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_retry_job_resets_to_pending() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Simulate failure
        service.update_job_status(&job_id, JobStatus::Failed, None).await.unwrap();

        // Act
        service.retry_job(&job_id).await.unwrap();

        // Assert
        let jobs = mock_repo.list_work().await.unwrap();
        let job = jobs.iter().find(|j| j.id == job_id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
    }

    // =============================================================================
    // EVENT PUBLISHING TESTS
    // =============================================================================

    #[tokio::test]
    async fn test_submit_job_publishes_job_submitted_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo, event_publisher.clone());

        // Act
        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobSubmitted { job_id: id, url, .. } => {
                assert_eq!(id, &job_id);
                assert_eq!(url, "https://example.com");
            }
            _ => panic!("Expected JobSubmitted event"),
        }
    }

    #[tokio::test]
    async fn test_claim_job_publishes_job_claimed_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        // Submit a job first
        service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        event_publisher.clear().await; // Clear submit event

        // Act
        let claimed_id = service.claim_job().await.unwrap();

        // Assert
        assert!(claimed_id.is_some());
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobClaimed { job_id, .. } => {
                assert_eq!(job_id, &claimed_id.unwrap());
            }
            _ => panic!("Expected JobClaimed event"),
        }
    }

    #[tokio::test]
    async fn test_update_status_to_completed_publishes_job_completed_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Follow valid transitions: Pending → Claimed → InProgress → Completed
        service.claim_job().await.unwrap();
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();

        event_publisher.clear().await;

        // Act
        service.update_job_status(&job_id, JobStatus::Completed, None).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobCompleted { job_id: id, .. } => {
                assert_eq!(id, &job_id);
            }
            _ => panic!("Expected JobCompleted event, got {:?}", events[0]),
        }
    }

    #[tokio::test]
    async fn test_update_status_to_failed_publishes_job_failed_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        event_publisher.clear().await;

        // Act
        service.update_job_status(&job_id, JobStatus::Failed, None).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobFailed { job_id: id, error, .. } => {
                assert_eq!(id, &job_id);
                assert!(error.contains("Job failed"));
            }
            _ => panic!("Expected JobFailed event, got {:?}", events[0]),
        }
    }

    #[tokio::test]
    async fn test_update_status_to_in_progress_publishes_status_changed_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Follow valid transition: Pending → Claimed → InProgress
        service.claim_job().await.unwrap();

        event_publisher.clear().await;

        // Act
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobStatusChanged { job_id: id, old_status, new_status, .. } => {
                assert_eq!(id, &job_id);
                assert_eq!(old_status, &JobStatus::Claimed);  // Changed from Pending
                assert_eq!(new_status, &JobStatus::InProgress);
            }
            _ => panic!("Expected JobStatusChanged event, got {:?}", events[0]),
        }
    }

    #[tokio::test]
    async fn test_cancel_job_publishes_job_cancelled_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        event_publisher.clear().await;

        // Act
        service.cancel_job(&job_id).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobCancelled { job_id: id, reason, .. } => {
                assert_eq!(id, &job_id);
                assert!(reason.contains("cancellation"));
            }
            _ => panic!("Expected JobCancelled event"),
        }
    }

    #[tokio::test]
    async fn test_retry_job_publishes_job_retried_event() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.update_job_status(&job_id, JobStatus::Failed, None).await.unwrap();

        event_publisher.clear().await;

        // Act
        service.retry_job(&job_id).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 1);

        match &events[0] {
            DomainEvent::JobRetried { job_id: id, .. } => {
                assert_eq!(id, &job_id);
            }
            _ => panic!("Expected JobRetried event"),
        }
    }

    #[tokio::test]
    async fn test_multiple_operations_publish_multiple_events() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        // Act - perform multiple operations
        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.claim_job().await.unwrap();
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();
        service.update_job_status(&job_id, JobStatus::Completed, None).await.unwrap();

        // Assert
        let events = event_publisher.get_events().await;
        assert_eq!(events.len(), 4); // submit + claim + in_progress + completed

        // Verify event sequence
        assert!(matches!(events[0], DomainEvent::JobSubmitted { .. }));
        assert!(matches!(events[1], DomainEvent::JobClaimed { .. }));
        assert!(matches!(events[2], DomainEvent::JobStatusChanged { .. }));
        assert!(matches!(events[3], DomainEvent::JobCompleted { .. }));
    }

    #[tokio::test]
    async fn test_events_for_specific_job() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let service = JobCommandService::with_events(mock_repo.clone(), event_publisher.clone());

        // Act - create multiple jobs
        let job1_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        let job2_id = service.submit_job(JobSubmission {
            url: "https://example.org".to_string(),
        }).await.unwrap();

        // Follow valid transitions for job1: Pending → Claimed → InProgress → Completed
        service.claim_job().await.unwrap(); // Claims job1
        service.update_job_status(&job1_id, JobStatus::InProgress, None).await.unwrap();
        service.update_job_status(&job1_id, JobStatus::Completed, None).await.unwrap();

        // Assert
        let job1_events = event_publisher.get_events_for_job(&job1_id).await;
        assert_eq!(job1_events.len(), 4); // submit + claim + in_progress + completed

        let job2_events = event_publisher.get_events_for_job(&job2_id).await;
        assert_eq!(job2_events.len(), 1); // only submit
    }

    // =============================================================================
    // STATE MACHINE VALIDATION TESTS
    // =============================================================================

    #[tokio::test]
    async fn test_reject_invalid_skip_transition_pending_to_in_progress() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act - Try to skip Claimed state
        let result = service.update_job_status(&job_id, JobStatus::InProgress, None).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("skipped Claimed"));
    }

    #[tokio::test]
    async fn test_reject_invalid_skip_transition_pending_to_completed() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act - Try to skip directly to Completed
        let result = service.update_job_status(&job_id, JobStatus::Completed, None).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("skipped states"));
    }

    #[tokio::test]
    async fn test_reject_invalid_backward_transition_claimed_to_pending() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.claim_job().await.unwrap();

        // Act - Try to go backward
        let result = service.update_job_status(&job_id, JobStatus::Pending, None).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("unclaim") || error.contains("backward"));
    }

    #[tokio::test]
    async fn test_reject_invalid_backward_transition_in_progress_to_claimed() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.claim_job().await.unwrap();
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();

        // Act - Try to go backward
        let result = service.update_job_status(&job_id, JobStatus::Claimed, None).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("revert") || error.contains("backward"));
    }

    #[tokio::test]
    async fn test_reject_transition_from_completed() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.claim_job().await.unwrap();
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();
        service.update_job_status(&job_id, JobStatus::Completed, None).await.unwrap();

        // Act - Try to change completed job
        let result = service.update_job_status(&job_id, JobStatus::Pending, None).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("immutable") || error.contains("terminal"));
    }

    #[tokio::test]
    async fn test_reject_cancel_completed_job() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.claim_job().await.unwrap();
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();
        service.update_job_status(&job_id, JobStatus::Completed, None).await.unwrap();

        // Act - Try to cancel completed job
        let result = service.cancel_job(&job_id).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Cannot cancel"));
    }

    #[tokio::test]
    async fn test_reject_retry_non_failed_job() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Move to Completed (terminal state)
        service.claim_job().await.unwrap();
        service.update_job_status(&job_id, JobStatus::InProgress, None).await.unwrap();
        service.update_job_status(&job_id, JobStatus::Completed, None).await.unwrap();

        // Act - Try to retry a completed job
        let result = service.retry_job(&job_id).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Cannot retry"));
    }

    #[tokio::test]
    async fn test_accept_idempotent_updates() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act - Update to same status (idempotent)
        let result = service.update_job_status(&job_id, JobStatus::Pending, None).await;

        // Assert - Should be allowed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_accept_retry_failed_job() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.update_job_status(&job_id, JobStatus::Failed, None).await.unwrap();

        // Act - Retry failed job (valid transition)
        let result = service.retry_job(&job_id).await;

        // Assert - Should succeed
        assert!(result.is_ok());

        let jobs = mock_repo.list_work().await.unwrap();
        let job = jobs.iter().find(|j| j.id == job_id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_accept_all_failure_transitions() {
        // Can fail from Pending, Claimed, or InProgress
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobCommandService::new(mock_repo.clone());

        // From Pending
        let job1_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();
        assert!(service.update_job_status(&job1_id, JobStatus::Failed, None).await.is_ok());

        // From Claimed
        let job2_id = service.submit_job(JobSubmission {
            url: "https://example.org".to_string(),
        }).await.unwrap();
        service.claim_job().await.unwrap();
        assert!(service.update_job_status(&job2_id, JobStatus::Failed, None).await.is_ok());

        // From InProgress
        let job3_id = service.submit_job(JobSubmission {
            url: "https://example.net".to_string(),
        }).await.unwrap();
        service.claim_job().await.unwrap();
        service.update_job_status(&job3_id, JobStatus::InProgress, None).await.unwrap();
        assert!(service.update_job_status(&job3_id, JobStatus::Failed, None).await.is_ok());
    }
}
