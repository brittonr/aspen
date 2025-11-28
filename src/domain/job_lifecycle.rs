//! Job lifecycle business logic (Facade over Command/Query services)
//!
//! **DEPRECATED:** This service is a useless middleman that just delegates to
//! JobCommandService and JobQueryService. It violates the principle of avoiding
//! unnecessary abstraction layers.
//!
//! **DO NOT USE THIS IN NEW CODE!** Use JobCommandService and JobQueryService directly:
//! - JobCommandService for mutations (submit, claim, update status)
//! - JobQueryService for reads (list jobs, get stats, search)
//!
//! This service exists only for backward compatibility during migration.
//! It will be removed in a future version.

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
/// **DEPRECATED:** This is a useless middleman layer that adds no value.
/// It simply delegates to JobCommandService and JobQueryService.
///
/// **Use JobCommandService and JobQueryService directly instead!**
///
/// This service will be removed in a future version.
#[deprecated(since = "0.2.0", note = "Use JobCommandService and JobQueryService directly")]
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
    ///
    /// # Arguments
    /// * `worker_id` - Optional worker ID to assign the job to
    /// * `worker_type` - Optional worker type for filtering compatible jobs
    pub async fn claim_work(&self, worker_id: Option<&str>, worker_type: Option<crate::domain::types::WorkerType>) -> Result<Option<Job>> {
        match self.commands.claim_job(worker_id, worker_type).await? {
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
