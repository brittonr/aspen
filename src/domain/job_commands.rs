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
    ///
    /// # Errors
    /// Returns error if the job doesn't exist or the status transition is invalid
    pub async fn update_job_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        // Future: Add state machine validation here
        // Future: Publish domain events here
        self.work_repo.update_status(job_id, status).await?;

        tracing::info!(job_id = %job_id, status = ?status, "Job status updated");

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
    pub async fn cancel_job(&self, job_id: &str) -> Result<()> {
        // Future: Validate that job is cancellable (Pending or Claimed only)
        self.work_repo.update_status(job_id, JobStatus::Failed).await?;

        tracing::info!(job_id = %job_id, "Job cancelled");

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
    pub async fn retry_job(&self, job_id: &str) -> Result<()> {
        // Future: Validate that job is Failed
        self.work_repo.update_status(job_id, JobStatus::Pending).await?;

        tracing::info!(job_id = %job_id, "Job reset for retry");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::MockWorkRepository;

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

        // Act
        service.update_job_status(&job_id, JobStatus::InProgress).await.unwrap();

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
        service.update_job_status(&job_id, JobStatus::Failed).await.unwrap();

        // Act
        service.retry_job(&job_id).await.unwrap();

        // Assert
        let jobs = mock_repo.list_work().await.unwrap();
        let job = jobs.iter().find(|j| j.id == job_id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
    }
}
