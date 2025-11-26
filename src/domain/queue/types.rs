//! Queue domain types
//!
//! Core types for queue statistics and health monitoring.

use serde::{Deserialize, Serialize};
use crate::domain::job::{Job, JobStatus};

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
