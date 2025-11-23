//! Job lifecycle business logic
//!
//! Encapsulates job creation, status transitions, validation, and querying.
//! Enforces business rules and state machine constraints.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Result;

use crate::repositories::WorkRepository;
use crate::work_queue::{WorkItem, WorkStatus, WorkQueueStats};

/// Job submission parameters
#[derive(Debug, Clone)]
pub struct JobSubmission {
    pub url: String,
}

/// Job with enriched metadata for display
#[derive(Debug, Clone)]
pub struct EnrichedJob {
    pub job_id: String,
    pub status: WorkStatus,
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

/// Domain service for job lifecycle operations
pub struct JobLifecycleService {
    work_repo: Arc<dyn WorkRepository>,
    job_counter: Arc<AtomicUsize>,
}

impl JobLifecycleService {
    /// Create a new job lifecycle service
    pub fn new(work_repo: Arc<dyn WorkRepository>) -> Self {
        Self {
            work_repo,
            job_counter: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Submit a new job to the queue
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
            "url": submission.url,
        });

        // Publish to distributed queue
        self.work_repo.publish_work(job_id.clone(), payload).await?;

        tracing::info!(job_id = %job_id, url = %submission.url, "Job submitted");

        Ok(job_id)
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> WorkQueueStats {
        self.work_repo.stats().await
    }

    /// List jobs with optional sorting and enrichment
    pub async fn list_jobs(
        &self,
        sort_order: JobSortOrder,
        limit: usize,
    ) -> Result<Vec<EnrichedJob>> {
        let mut work_items = self.work_repo.list_work().await?;

        // Apply sorting
        self.sort_jobs(&mut work_items, sort_order);

        // Take limited subset
        let work_items: Vec<_> = work_items.into_iter().take(limit).collect();

        // Enrich with computed metadata
        let now = Self::current_timestamp();
        let enriched = work_items
            .into_iter()
            .map(|item| self.enrich_job(item, now))
            .collect();

        Ok(enriched)
    }

    /// Sort jobs according to specified order
    fn sort_jobs(&self, jobs: &mut Vec<WorkItem>, sort_order: JobSortOrder) {
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
                jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));
            }
            JobSortOrder::Time => {
                // Most recent first
                jobs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
        }
    }

    /// Enrich a work item with computed metadata
    fn enrich_job(&self, item: WorkItem, now: i64) -> EnrichedJob {
        let url = item
            .payload
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("-")
            .to_string();

        let duration_seconds = item.updated_at - item.created_at;
        let time_ago_seconds = now - item.updated_at;

        EnrichedJob {
            job_id: item.job_id,
            status: item.status,
            url,
            duration_seconds,
            time_ago_seconds,
            claimed_by: item.claimed_by,
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
