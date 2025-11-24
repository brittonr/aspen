//! Domain types for job management
//!
//! These types are owned by the domain layer and independent of infrastructure.
//! They represent the business concepts without coupling to storage or transport.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Job status in the workflow lifecycle
///
/// This is the domain representation of job status, independent of how
/// it's stored or transmitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is available for claiming
    Pending,
    /// Job has been claimed by a worker
    Claimed,
    /// Job is being executed
    InProgress,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "Pending"),
            JobStatus::Claimed => write!(f, "Claimed"),
            JobStatus::InProgress => write!(f, "InProgress"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Job representing work in the distributed queue
///
/// This is the domain representation of a job, containing only business-relevant
/// information. Infrastructure concerns (like gossip topics, network addresses)
/// are kept in the infrastructure layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: String,
    /// Current status of the job
    pub status: JobStatus,
    /// Worker node that claimed this job (if any)
    pub claimed_by: Option<String>,
    /// Worker node that completed this job (if any)
    pub completed_by: Option<String>,
    /// Timestamp when job was created (Unix epoch seconds)
    pub created_at: i64,
    /// Timestamp when job was last updated (Unix epoch seconds)
    pub updated_at: i64,
    /// Timestamp when job execution started (Unix epoch seconds)
    pub started_at: Option<i64>,
    /// Error message if job failed
    pub error_message: Option<String>,
    /// Number of times this job has been retried
    pub retry_count: u32,
    /// Job payload (application-specific data)
    pub payload: JsonValue,
}

impl Job {
    /// Get the URL from the job payload (convenience method)
    pub fn url(&self) -> Option<&str> {
        self.payload.get("url")?.as_str()
    }

    /// Calculate duration of job execution in seconds
    ///
    /// Returns the time from when the job started executing (InProgress)
    /// to when it was last updated. Returns 0 if job hasn't started yet.
    pub fn duration_seconds(&self) -> i64 {
        match self.started_at {
            Some(start) => self.updated_at - start,
            None => 0,
        }
    }

    /// Calculate duration of job execution in milliseconds
    ///
    /// Returns the time from when the job started executing (InProgress)
    /// to when it was last updated. Returns 0 if job hasn't started yet.
    pub fn duration_ms(&self) -> i64 {
        self.duration_seconds() * 1000
    }

    /// Calculate time since last update in seconds
    pub fn time_since_update_seconds(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        now - self.updated_at
    }

    /// Check if job is in a terminal state (completed or failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, JobStatus::Completed | JobStatus::Failed)
    }

    /// Check if job is claimable (pending status)
    pub fn is_claimable(&self) -> bool {
        matches!(self.status, JobStatus::Pending)
    }

    /// Check if job is actively running
    pub fn is_running(&self) -> bool {
        matches!(self.status, JobStatus::InProgress)
    }
}

/// Aggregate statistics for the job queue
///
/// Domain representation of queue health and activity metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total number of jobs in the queue
    pub total: usize,
    /// Number of jobs pending (available for claim)
    pub pending: usize,
    /// Number of jobs claimed by workers
    pub claimed: usize,
    /// Number of jobs in progress
    pub in_progress: usize,
    /// Number of completed jobs
    pub completed: usize,
    /// Number of failed jobs
    pub failed: usize,
}

impl QueueStats {
    /// Create empty statistics
    pub fn empty() -> Self {
        Self {
            total: 0,
            pending: 0,
            claimed: 0,
            in_progress: 0,
            completed: 0,
            failed: 0,
        }
    }

    /// Calculate from a collection of jobs
    pub fn from_jobs(jobs: &[Job]) -> Self {
        let mut stats = Self::empty();
        stats.total = jobs.len();

        for job in jobs {
            match job.status {
                JobStatus::Pending => stats.pending += 1,
                JobStatus::Claimed => stats.claimed += 1,
                JobStatus::InProgress => stats.in_progress += 1,
                JobStatus::Completed => stats.completed += 1,
                JobStatus::Failed => stats.failed += 1,
            }
        }

        stats
    }
}

/// Database cluster health status
///
/// Domain representation of distributed database health, independent of
/// infrastructure implementation (hiqlite, postgres, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthStatus {
    /// Whether the database cluster is operating normally
    pub is_healthy: bool,
    /// Number of nodes in the database cluster
    pub node_count: usize,
    /// Whether the cluster has an elected leader
    pub has_leader: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_display() {
        assert_eq!(JobStatus::Pending.to_string(), "Pending");
        assert_eq!(JobStatus::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_job_url_extraction() {
        let job = Job {
            id: "job-1".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1000,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({
                "url": "https://example.com"
            }),
        };

        assert_eq!(job.url(), Some("https://example.com"));
    }

    #[test]
    fn test_job_duration() {
        let job = Job {
            id: "job-1".to_string(),
            status: JobStatus::Completed,
            claimed_by: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1030,
            started_at: Some(1000),
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({}),
        };

        assert_eq!(job.duration_seconds(), 30);
        assert_eq!(job.duration_ms(), 30000);
    }

    #[test]
    fn test_job_duration_without_start() {
        let job = Job {
            id: "job-1".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1030,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({}),
        };

        assert_eq!(job.duration_seconds(), 0);
        assert_eq!(job.duration_ms(), 0);
    }

    #[test]
    fn test_job_state_checks() {
        let pending = Job {
            id: "job-1".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1000,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({}),
        };

        assert!(pending.is_claimable());
        assert!(!pending.is_terminal());
        assert!(!pending.is_running());

        let completed = Job {
            status: JobStatus::Completed,
            ..pending.clone()
        };

        assert!(!completed.is_claimable());
        assert!(completed.is_terminal());
        assert!(!completed.is_running());

        let in_progress = Job {
            status: JobStatus::InProgress,
            ..pending.clone()
        };

        assert!(!in_progress.is_claimable());
        assert!(!in_progress.is_terminal());
        assert!(in_progress.is_running());
    }

    #[test]
    fn test_queue_stats_calculation() {
        let jobs = vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None,
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                started_at: None,
                error_message: None,
                retry_count: 0,
                payload: serde_json::json!({}),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                started_at: Some(1005),
                error_message: None,
                retry_count: 0,
                payload: serde_json::json!({}),
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1020,
                started_at: Some(1005),
                error_message: None,
                retry_count: 0,
                payload: serde_json::json!({}),
            },
        ];

        let stats = QueueStats::from_jobs(&jobs);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.claimed, 0);
        assert_eq!(stats.failed, 0);
    }
}
