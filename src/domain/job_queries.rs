//! Job query service - handles reads (queries)
//!
//! This service is responsible for all job data retrieval following the
//! Command Query Responsibility Segregation (CQRS) pattern. Queries return
//! domain data but never modify state.

use std::sync::Arc;
use anyhow::Result;

use crate::repositories::WorkRepository;
use crate::domain::types::{Job, QueueStats};

/// Job with enriched metadata for display
#[derive(Debug, Clone)]
pub struct EnrichedJob {
    pub job_id: String,
    pub status: crate::domain::types::JobStatus,
    pub url: String,
    pub duration_seconds: i64,
    pub time_ago_seconds: i64,
    pub claimed_by: Option<String>,
}

/// Job sorting options
#[derive(Debug, Clone, Copy)]
pub enum JobSortOrder {
    Time,
    Status,
    JobId,
}

impl JobSortOrder {
    pub fn from_str(s: &str) -> Self {
        match s {
            "status" => Self::Status,
            "job_id" => Self::JobId,
            _ => Self::Time,
        }
    }
}

/// Query service for job reads
///
/// Handles all read operations for jobs:
/// - Listing jobs with filtering and sorting
/// - Getting job details
/// - Computing queue statistics
/// - Enriching jobs with computed metadata
///
/// This service follows the CQRS pattern by separating reads from writes.
/// It never modifies job state - use JobCommandService for mutations.
pub struct JobQueryService {
    work_repo: Arc<dyn WorkRepository>,
}

impl JobQueryService {
    /// Create a new job query service
    pub fn new(work_repo: Arc<dyn WorkRepository>) -> Self {
        Self { work_repo }
    }

    /// List all jobs (raw, unsorted)
    ///
    /// This is a **query** - it returns data without modifying state.
    /// For filtered/sorted lists, use `list_jobs_with_options`.
    ///
    /// # Returns
    /// All jobs in the queue (unordered)
    pub async fn list_all_jobs(&self) -> Result<Vec<Job>> {
        self.work_repo.list_work().await
    }

    /// List jobs with sorting and enrichment
    ///
    /// This is a **query** - it returns enriched job data for display.
    /// Jobs are sorted according to the specified order and limited to
    /// the requested count.
    ///
    /// # Arguments
    /// * `sort_order` - How to sort the results
    /// * `limit` - Maximum number of jobs to return
    ///
    /// # Returns
    /// Enriched jobs with computed metadata (duration, time ago, etc.)
    pub async fn list_jobs_with_options(
        &self,
        sort_order: JobSortOrder,
        limit: usize,
    ) -> Result<Vec<EnrichedJob>> {
        let mut jobs = self.work_repo.list_work().await?;

        // Apply sorting
        self.sort_jobs(&mut jobs, sort_order);

        // Take limited subset
        let jobs: Vec<_> = jobs.into_iter().take(limit).collect();

        // Enrich with computed metadata
        let now = Self::current_timestamp();
        let enriched = jobs
            .into_iter()
            .map(|job| self.enrich_job(job, now))
            .collect();

        Ok(enriched)
    }

    /// Get job by ID
    ///
    /// This is a **query** - it retrieves a single job by identifier.
    /// Uses repository-level filtering for efficient data access.
    ///
    /// # Arguments
    /// * `job_id` - The job identifier to look up
    ///
    /// # Returns
    /// The job if found, None otherwise
    pub async fn get_job(&self, job_id: &str) -> Result<Option<Job>> {
        // Use repository-level filtering instead of loading all jobs
        self.work_repo.find_by_id(job_id).await
    }

    /// Get queue statistics
    ///
    /// This is a **query** - it computes aggregate statistics.
    /// Returns counts of jobs in each status.
    ///
    /// # Returns
    /// Queue statistics (total, pending, in_progress, completed, failed)
    pub async fn get_queue_stats(&self) -> QueueStats {
        self.work_repo.stats().await
    }

    /// Count jobs by status
    ///
    /// This is a **query** - it filters and counts jobs.
    /// Uses repository-level filtering for efficient data access.
    ///
    /// # Arguments
    /// * `status` - The status to count
    ///
    /// # Returns
    /// Number of jobs with the specified status
    pub async fn count_jobs_by_status(
        &self,
        status: crate::domain::types::JobStatus,
    ) -> Result<usize> {
        // Use repository-level filtering instead of loading all jobs
        let jobs = self.work_repo.find_by_status(status).await?;
        Ok(jobs.len())
    }

    /// Get jobs claimed by a specific worker
    ///
    /// This is a **query** - it filters jobs by worker.
    /// Uses repository-level filtering for efficient data access.
    ///
    /// # Arguments
    /// * `worker_id` - The worker node ID
    ///
    /// # Returns
    /// All jobs claimed by the specified worker
    pub async fn get_jobs_by_worker(&self, worker_id: &str) -> Result<Vec<Job>> {
        // Use repository-level filtering instead of loading all jobs
        self.work_repo.find_by_worker(worker_id).await
    }

    /// Get paginated jobs (most recent first)
    ///
    /// This is a **query** - it retrieves a paginated subset of jobs.
    /// Uses repository-level pagination for efficient data access with large result sets.
    ///
    /// # Arguments
    /// * `offset` - Number of items to skip (0-based)
    /// * `limit` - Maximum number of items to return
    ///
    /// # Returns
    /// Paginated slice of jobs (ordered by update time, most recent first)
    pub async fn get_jobs_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>> {
        // Use repository-level pagination instead of loading all jobs
        self.work_repo.find_paginated(offset, limit).await
    }

    /// Get jobs by status for a specific worker (composite query)
    ///
    /// This is a **composite query** - it filters by multiple criteria.
    /// Uses repository-level filtering for efficient data access.
    ///
    /// # Arguments
    /// * `status` - The status to filter by
    /// * `worker_id` - The worker node ID to filter by
    ///
    /// # Returns
    /// All jobs matching both criteria
    pub async fn get_jobs_by_status_and_worker(
        &self,
        status: crate::domain::types::JobStatus,
        worker_id: &str,
    ) -> Result<Vec<Job>> {
        // Use repository-level composite filtering
        self.work_repo.find_by_status_and_worker(status, worker_id).await
    }

    /// Sort jobs according to specified order
    fn sort_jobs(&self, jobs: &mut Vec<Job>, sort_order: JobSortOrder) {
        match sort_order {
            JobSortOrder::Status => {
                jobs.sort_by(|a, b| {
                    match format!("{:?}", a.status).cmp(&format!("{:?}", b.status)) {
                        std::cmp::Ordering::Equal => b.updated_at.cmp(&a.updated_at),
                        other => other,
                    }
                });
            }
            JobSortOrder::JobId => {
                jobs.sort_by(|a, b| a.id.cmp(&b.id));
            }
            JobSortOrder::Time => {
                // Most recent first
                jobs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
        }
    }

    /// Enrich a job with computed metadata
    fn enrich_job(&self, job: Job, now: i64) -> EnrichedJob {
        let url = job.url().unwrap_or("-").to_string();

        let duration_seconds = job.duration_seconds();
        let time_ago_seconds = now - job.updated_at;

        EnrichedJob {
            job_id: job.id,
            status: job.status,
            url,
            duration_seconds,
            time_ago_seconds,
            claimed_by: job.claimed_by,
        }
    }

    /// Get current Unix timestamp
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::MockWorkRepository;
    use crate::domain::types::{Job, JobStatus};

    #[tokio::test]
    async fn test_list_all_jobs_returns_all() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({"url": "https://example.com"}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({"url": "https://example.org"}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let jobs = service.list_all_jobs().await.unwrap();

        // Assert
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_list_jobs_with_options_sorts_correctly() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000, // Older
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: None,
                completed_by: None,
                created_at: 2000,
                updated_at: 2000, // Newer
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act - sort by time (most recent first)
        let jobs = service
            .list_jobs_with_options(JobSortOrder::Time, 10)
            .await
            .unwrap();

        // Assert
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].job_id, "job-2"); // Newer job first
        assert_eq!(jobs[1].job_id, "job-1");
    }

    #[tokio::test]
    async fn test_list_jobs_with_options_limits_results() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1020,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act - limit to 2 results
        let jobs = service
            .list_jobs_with_options(JobSortOrder::Time, 2)
            .await
            .unwrap();

        // Assert
        assert_eq!(jobs.len(), 2); // Limited to 2
    }

    #[tokio::test]
    async fn test_get_job_finds_by_id() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![Job {
            id: "target-job".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1000,
            payload: serde_json::json!({}),
            started_at: None,
            error_message: None,
            retry_count: 0,
        }]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let job = service.get_job("target-job").await.unwrap();

        // Assert
        assert!(job.is_some());
        assert_eq!(job.unwrap().id, "target-job");
    }

    #[tokio::test]
    async fn test_get_job_returns_none_when_not_found() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        let service = JobQueryService::new(mock_repo);

        // Act
        let job = service.get_job("nonexistent").await.unwrap();

        // Assert
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_count_jobs_by_status() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1020,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let pending_count = service
            .count_jobs_by_status(JobStatus::Pending)
            .await
            .unwrap();

        // Assert
        assert_eq!(pending_count, 2);
    }

    #[tokio::test]
    async fn test_get_jobs_by_worker() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-2".to_string()),
                completed_by: Some("worker-2".to_string()),
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1020,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let worker1_jobs = service.get_jobs_by_worker("worker-1").await.unwrap();

        // Assert
        assert_eq!(worker1_jobs.len(), 2);
        assert!(worker1_jobs.iter().all(|job| job.claimed_by.as_deref() == Some("worker-1")));
    }
}

// Tests for repository-level filtering methods
#[cfg(test)]
mod repository_filtering_tests {
    use super::*;
    use crate::repositories::mocks::MockWorkRepository;
    use crate::domain::types::{Job, JobStatus};

    #[tokio::test]
    async fn test_find_by_id_returns_job_when_exists() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({"url": "https://example.com"}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let result = mock_repo.find_by_id("job-1").await.unwrap();

        // Assert
        assert!(result.is_some());
        let job = result.unwrap();
        assert_eq!(job.id, "job-1");
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_find_by_id_returns_none_when_not_exists() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let result = mock_repo.find_by_id("nonexistent").await.unwrap();

        // Assert
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_status_returns_matching_jobs() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1200,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-4".to_string(),
                status: JobStatus::Failed,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1300,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let pending_jobs = mock_repo.find_by_status(JobStatus::Pending).await.unwrap();
        let completed_jobs = mock_repo.find_by_status(JobStatus::Completed).await.unwrap();
        let failed_jobs = mock_repo.find_by_status(JobStatus::Failed).await.unwrap();

        // Assert
        assert_eq!(pending_jobs.len(), 2);
        assert!(pending_jobs.iter().all(|job| job.status == JobStatus::Pending));
        assert_eq!(pending_jobs[0].id, "job-1");
        assert_eq!(pending_jobs[1].id, "job-3");

        assert_eq!(completed_jobs.len(), 1);
        assert_eq!(completed_jobs[0].id, "job-2");

        assert_eq!(failed_jobs.len(), 1);
        assert_eq!(failed_jobs[0].id, "job-4");
    }

    #[tokio::test]
    async fn test_find_by_status_returns_empty_when_no_matches() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let completed_jobs = mock_repo.find_by_status(JobStatus::Completed).await.unwrap();

        // Assert
        assert_eq!(completed_jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_query_service_uses_find_by_id() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "target-job".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({"url": "https://example.com"}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "other-job".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let result = service.get_job("target-job").await.unwrap();

        // Assert - should use repository filtering, not load all jobs
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "target-job");
    }

    #[tokio::test]
    async fn test_query_service_uses_find_by_status_for_count() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1020,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let pending_count = service.count_jobs_by_status(JobStatus::Pending).await.unwrap();
        let completed_count = service.count_jobs_by_status(JobStatus::Completed).await.unwrap();

        // Assert - should use repository filtering for efficiency
        assert_eq!(pending_count, 2);
        assert_eq!(completed_count, 1);
    }

    #[tokio::test]
    async fn test_find_by_worker_returns_matching_jobs() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-2".to_string()),
                completed_by: Some("worker-2".to_string()),
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1200,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let worker1_jobs = mock_repo.find_by_worker("worker-1").await.unwrap();
        let worker2_jobs = mock_repo.find_by_worker("worker-2").await.unwrap();

        // Assert
        assert_eq!(worker1_jobs.len(), 2);
        assert!(worker1_jobs.iter().all(|job| job.claimed_by.as_deref() == Some("worker-1")));
        assert_eq!(worker1_jobs[0].id, "job-1");
        assert_eq!(worker1_jobs[1].id, "job-3");

        assert_eq!(worker2_jobs.len(), 1);
        assert_eq!(worker2_jobs[0].id, "job-2");
    }

    #[tokio::test]
    async fn test_find_by_worker_returns_empty_when_no_matches() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let result = mock_repo.find_by_worker("nonexistent-worker").await.unwrap();

        // Assert
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_find_paginated_returns_correct_page() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,  // Oldest
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1200,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-4".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1300,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-5".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1400,  // Newest
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act - First page (offset 0, limit 2)
        let page1 = mock_repo.find_paginated(0, 2).await.unwrap();

        // Act - Second page (offset 2, limit 2)
        let page2 = mock_repo.find_paginated(2, 2).await.unwrap();

        // Act - Third page (offset 4, limit 2)
        let page3 = mock_repo.find_paginated(4, 2).await.unwrap();

        // Assert - Pages should be ordered by updated_at (most recent first)
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].id, "job-5");  // Newest
        assert_eq!(page1[1].id, "job-4");

        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].id, "job-3");
        assert_eq!(page2[1].id, "job-2");

        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].id, "job-1");  // Oldest
    }

    #[tokio::test]
    async fn test_find_paginated_returns_empty_when_offset_exceeds_total() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let result = mock_repo.find_paginated(10, 5).await.unwrap();

        // Assert
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_find_by_status_and_worker_returns_matching_jobs() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-2".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1200,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-4".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-2".to_string()),
                completed_by: Some("worker-2".to_string()),
                created_at: 1000,
                updated_at: 1300,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act
        let worker1_in_progress = mock_repo
            .find_by_status_and_worker(JobStatus::InProgress, "worker-1")
            .await
            .unwrap();

        let worker1_completed = mock_repo
            .find_by_status_and_worker(JobStatus::Completed, "worker-1")
            .await
            .unwrap();

        let worker2_in_progress = mock_repo
            .find_by_status_and_worker(JobStatus::InProgress, "worker-2")
            .await
            .unwrap();

        // Assert
        assert_eq!(worker1_in_progress.len(), 1);
        assert_eq!(worker1_in_progress[0].id, "job-1");

        assert_eq!(worker1_completed.len(), 1);
        assert_eq!(worker1_completed[0].id, "job-2");

        assert_eq!(worker2_in_progress.len(), 1);
        assert_eq!(worker2_in_progress[0].id, "job-3");
    }

    #[tokio::test]
    async fn test_find_by_status_and_worker_returns_empty_when_no_matches() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        // Act - Wrong status
        let result1 = mock_repo
            .find_by_status_and_worker(JobStatus::Completed, "worker-1")
            .await
            .unwrap();

        // Act - Wrong worker
        let result2 = mock_repo
            .find_by_status_and_worker(JobStatus::InProgress, "worker-2")
            .await
            .unwrap();

        // Assert
        assert_eq!(result1.len(), 0);
        assert_eq!(result2.len(), 0);
    }

    #[tokio::test]
    async fn test_query_service_uses_find_by_worker() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-2".to_string()),
                completed_by: Some("worker-2".to_string()),
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let worker1_jobs = service.get_jobs_by_worker("worker-1").await.unwrap();

        // Assert - should use repository filtering, not load all jobs
        assert_eq!(worker1_jobs.len(), 1);
        assert_eq!(worker1_jobs[0].id, "job-1");
    }

    #[tokio::test]
    async fn test_query_service_get_jobs_paginated() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1200,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let page1 = service.get_jobs_paginated(0, 2).await.unwrap();
        let page2 = service.get_jobs_paginated(2, 2).await.unwrap();

        // Assert - should use repository pagination
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].id, "job-3");  // Most recent first
        assert_eq!(page1[1].id, "job-2");

        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].id, "job-1");  // Oldest
    }

    #[tokio::test]
    async fn test_query_service_get_jobs_by_status_and_worker() {
        // Arrange
        let mock_repo = Arc::new(MockWorkRepository::new());
        mock_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1100,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
            },
        ]).await;

        let service = JobQueryService::new(mock_repo);

        // Act
        let in_progress_jobs = service
            .get_jobs_by_status_and_worker(JobStatus::InProgress, "worker-1")
            .await
            .unwrap();

        // Assert - should use repository composite filtering
        assert_eq!(in_progress_jobs.len(), 1);
        assert_eq!(in_progress_jobs[0].id, "job-1");
    }
}
