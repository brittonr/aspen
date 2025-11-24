//! Job lifecycle business logic (Facade over Command/Query services)
//!
//! This service provides a unified API over JobCommandService and JobQueryService,
//! maintaining backward compatibility while following CQRS principles internally.
//!
//! New code should prefer using JobCommandService and JobQueryService directly
//! for clearer intent (mutation vs read).

use std::sync::Arc;
use anyhow::Result;

use crate::domain::job_commands::JobCommandService;
use crate::domain::job_queries::JobQueryService;
use crate::domain::types::{Job, JobStatus, QueueStats};

// Re-export types for backward compatibility
pub use crate::domain::job_commands::JobSubmission;
pub use crate::domain::job_queries::{EnrichedJob, JobSortOrder};

/// Facade service providing unified job lifecycle API
///
/// This service delegates to JobCommandService (writes) and JobQueryService (reads)
/// following the CQRS pattern. It exists for backward compatibility and convenience.
///
/// **Recommendation:** New code should use JobCommandService and JobQueryService
/// directly for clearer separation of concerns.
pub struct JobLifecycleService {
    commands: Arc<JobCommandService>,
    queries: Arc<JobQueryService>,
}

impl JobLifecycleService {
    /// Create a new job lifecycle service from repository
    ///
    /// This creates both command and query services internally.
    pub fn new(work_repo: Arc<dyn crate::repositories::WorkRepository>) -> Self {
        let commands = Arc::new(JobCommandService::new(work_repo.clone()));
        let queries = Arc::new(JobQueryService::new(work_repo));

        Self { commands, queries }
    }

    /// Create from pre-built command and query services
    ///
    /// Useful when you want to inject event publishers into commands.
    pub fn from_services(
        commands: Arc<JobCommandService>,
        queries: Arc<JobQueryService>,
    ) -> Self {
        Self { commands, queries }
    }

    // ===== Command methods (delegate to JobCommandService) =====

    /// Submit a new job to the queue (Command)
    pub async fn submit_job(&self, submission: JobSubmission) -> Result<String> {
        self.commands.submit_job(submission).await
    }

    /// Claim a work item from the queue (Command)
    pub async fn claim_work(&self) -> Result<Option<Job>> {
        match self.commands.claim_job().await? {
            Some(job_id) => {
                // Need to fetch the full job for backward compatibility
                self.queries.get_job(&job_id).await
            }
            None => Ok(None),
        }
    }

    /// Update work item status (Command)
    pub async fn update_work_status(
        &self,
        job_id: &str,
        status: JobStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        self.commands.update_job_status(job_id, status, error_message).await
    }

    // ===== Query methods (delegate to JobQueryService) =====

    /// List all work items (Query)
    pub async fn list_all_work(&self) -> Result<Vec<Job>> {
        self.queries.list_all_jobs().await
    }

    /// Get queue statistics (Query)
    pub async fn get_queue_stats(&self) -> QueueStats {
        self.queries.get_queue_stats().await
    }

    /// List jobs with optional sorting and enrichment (Query)
    pub async fn list_jobs(
        &self,
        sort_order: JobSortOrder,
        limit: usize,
    ) -> Result<Vec<EnrichedJob>> {
        self.queries.list_jobs_with_options(sort_order, limit).await
    }
}

/// Format duration for display
pub fn format_duration(seconds: i64) -> (String, &'static str) {
    if seconds == 0 {
        ("-".to_string(), "")
    } else if seconds < 1 {
        (format!("{}ms", seconds * 1000), "fast")
    } else if seconds < 5 {
        (format!("{}s", seconds), "fast")
    } else if seconds < 30 {
        (format!("{}s", seconds), "medium")
    } else {
        (format!("{}s", seconds), "slow")
    }
}

/// Format time ago for display
pub fn format_time_ago(seconds: i64) -> String {
    if seconds < 5 {
        "just now".to_string()
    } else if seconds < 60 {
        format!("{}s ago", seconds)
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h ago", seconds / 3600)
    } else {
        format!("{}d ago", seconds / 86400)
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
        let service = JobLifecycleService::new(mock_repo);

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
        let service = JobLifecycleService::new(mock_repo.clone());

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
    async fn test_claim_work_delegates_to_repository() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobLifecycleService::new(mock_repo.clone());

        // Add a work item
        service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Act
        let result = service.claim_work().await;

        // Assert
        assert!(result.is_ok());
        let job = result.unwrap();
        assert!(job.is_some());
        assert_eq!(job.unwrap().status, JobStatus::Claimed);
    }

    #[tokio::test]
    async fn test_update_status_delegates_to_repository() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobLifecycleService::new(mock_repo.clone());

        // Submit and claim a job
        let job_id = service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        // Follow valid transition: Pending → Claimed → InProgress
        service.claim_work().await.unwrap();

        // Act
        let result = service.update_work_status(&job_id, JobStatus::InProgress, None).await;

        // Assert
        assert!(result.is_ok());

        // Verify the status was updated
        let jobs = service.list_all_work().await.unwrap();
        let job = jobs.iter().find(|job| job.id == job_id).unwrap();
        assert_eq!(job.status, JobStatus::InProgress);
    }

    #[tokio::test]
    async fn test_list_all_work_returns_raw_items() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobLifecycleService::new(mock_repo.clone());

        // Submit multiple jobs
        service.submit_job(JobSubmission {
            url: "https://example.com".to_string(),
        }).await.unwrap();

        service.submit_job(JobSubmission {
            url: "https://example.org".to_string(),
        }).await.unwrap();

        // Act
        let result = service.list_all_work().await;

        // Assert
        assert!(result.is_ok());
        let jobs = result.unwrap();
        assert_eq!(jobs.len(), 2);
    }
}
