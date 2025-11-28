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
    pub payload_summary: String,
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
                        std::cmp::Ordering::Equal => b.updated_at().cmp(&a.updated_at()),
                        other => other,
                    }
                });
            }
            JobSortOrder::JobId => {
                jobs.sort_by(|a, b| a.id.cmp(&b.id));
            }
            JobSortOrder::Time => {
                // Most recent first
                jobs.sort_by(|a, b| b.updated_at().cmp(&a.updated_at()));
            }
        }
    }

    /// Enrich a job with computed metadata
    fn enrich_job(&self, job: Job, now: i64) -> EnrichedJob {
        // Generate a summary of the payload for display
        let payload_summary = if let Some(url) = job.payload.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else if let Some(job_type) = job.payload.get("type").and_then(|v| v.as_str()) {
            format!("Type: {}", job_type)
        } else if let Some(name) = job.payload.get("name").and_then(|v| v.as_str()) {
            format!("Name: {}", name)
        } else {
            format!("Payload with {} fields", job.payload.as_object().map(|o| o.len()).unwrap_or(0))
        };

        let duration_seconds = job.duration_seconds();
        let time_ago_seconds = now - job.updated_at();

        EnrichedJob {
            job_id: job.id,
            status: job.status,
            payload_summary,
            duration_seconds,
            time_ago_seconds,
            claimed_by: job.claimed_by,
        }
    }

    /// Get current Unix timestamp
    fn current_timestamp() -> i64 {
        crate::common::current_timestamp_or_zero()
    }
}
