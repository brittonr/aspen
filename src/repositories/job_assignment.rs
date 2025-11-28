//! Job assignment tracking (infrastructure concern)
//!
//! This module defines types for tracking which workers/nodes are assigned
//! to execute jobs. These are infrastructure concerns separate from the
//! domain Job entity.

use serde::{Deserialize, Serialize};

/// Job assignment metadata tracked at the infrastructure layer
///
/// Separates infrastructure concerns (which worker/node is executing)
/// from domain concerns (job business logic and status).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobAssignment {
    /// Job ID this assignment is for
    pub job_id: String,
    /// Worker node that claimed this job (from cluster)
    pub claimed_by_node: Option<String>,
    /// Worker ID from worker registry
    pub assigned_worker_id: Option<String>,
    /// Worker node that completed this job
    pub completed_by_node: Option<String>,
    /// Timestamp when job was claimed
    pub claimed_at: Option<i64>,
    /// Timestamp when job execution started
    pub started_at: Option<i64>,
    /// Timestamp when job was completed
    pub completed_at: Option<i64>,
}

impl JobAssignment {
    /// Create a new unassigned job assignment record
    pub fn new(job_id: String) -> Self {
        Self {
            job_id,
            claimed_by_node: None,
            assigned_worker_id: None,
            completed_by_node: None,
            claimed_at: None,
            started_at: None,
            completed_at: None,
        }
    }

    /// Mark job as claimed by a worker/node
    pub fn claim(&mut self, node_id: String, worker_id: Option<String>, timestamp: i64) {
        self.claimed_by_node = Some(node_id);
        self.assigned_worker_id = worker_id;
        self.claimed_at = Some(timestamp);
    }

    /// Mark job as started
    pub fn mark_started(&mut self, timestamp: i64) {
        self.started_at = Some(timestamp);
    }

    /// Mark job as completed
    pub fn complete(&mut self, node_id: String, timestamp: i64) {
        self.completed_by_node = Some(node_id);
        self.completed_at = Some(timestamp);
    }

    /// Check if job has been claimed
    pub fn is_claimed(&self) -> bool {
        self.claimed_by_node.is_some()
    }

    /// Check if job has started execution
    pub fn is_started(&self) -> bool {
        self.started_at.is_some()
    }

    /// Check if job has completed
    pub fn is_completed(&self) -> bool {
        self.completed_by_node.is_some()
    }
}

impl From<hiqlite::Row<'static>> for JobAssignment {
    fn from(mut row: hiqlite::Row) -> Self {
        Self {
            job_id: row.get("job_id"),
            claimed_by_node: row.get("claimed_by_node"),
            assigned_worker_id: row.get("assigned_worker_id"),
            completed_by_node: row.get("completed_by_node"),
            claimed_at: row.get("claimed_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
        }
    }
}

impl Default for JobAssignment {
    fn default() -> Self {
        Self::new(String::new())
    }
}
