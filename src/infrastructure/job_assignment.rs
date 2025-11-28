//! Job assignment infrastructure types
//!
//! This module defines infrastructure types for tracking job assignment
//! to workers, separate from the domain Job type.

use serde::{Deserialize, Serialize};

/// Job assignment tracks which worker is assigned to execute a job
///
/// This is an infrastructure concern separate from the domain Job.
/// It represents the operational aspects of job execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobAssignment {
    /// ID of the job being assigned
    pub job_id: String,
    /// Worker ID from the worker registry (if assigned)
    pub assigned_worker_id: Option<String>,
    /// Node ID that claimed this job (for distributed claim tracking)
    pub claimed_by: Option<String>,
    /// Node ID that completed this job (for distributed completion tracking)
    pub completed_by: Option<String>,
    /// Timestamp when job was assigned (Unix epoch seconds)
    pub assigned_at: Option<i64>,
    /// Strategy used for assignment
    pub assignment_strategy: AssignmentStrategy,
}

impl JobAssignment {
    /// Create a new unassigned job assignment
    pub fn new(job_id: String) -> Self {
        Self {
            job_id,
            assigned_worker_id: None,
            claimed_by: None,
            completed_by: None,
            assigned_at: None,
            assignment_strategy: AssignmentStrategy::FirstAvailable,
        }
    }

    /// Create an assignment to a specific worker
    pub fn to_worker(job_id: String, worker_id: String) -> Self {
        let now = current_timestamp();
        Self {
            job_id,
            assigned_worker_id: Some(worker_id),
            claimed_by: None,
            completed_by: None,
            assigned_at: Some(now),
            assignment_strategy: AssignmentStrategy::Manual,
        }
    }

    /// Assign to a worker
    pub fn assign_to(&mut self, worker_id: String) {
        self.assigned_worker_id = Some(worker_id);
        self.assigned_at = Some(current_timestamp());
    }

    /// Mark as claimed by a node
    pub fn claim_by(&mut self, node_id: String) {
        self.claimed_by = Some(node_id);
    }

    /// Mark as completed by a node
    pub fn complete_by(&mut self, node_id: String) {
        self.completed_by = Some(node_id);
    }

    /// Check if assigned to a worker
    pub fn is_assigned(&self) -> bool {
        self.assigned_worker_id.is_some()
    }

    /// Check if claimed by a node
    pub fn is_claimed(&self) -> bool {
        self.claimed_by.is_some()
    }

    /// Check if completed
    pub fn is_completed(&self) -> bool {
        self.completed_by.is_some()
    }
}

/// Strategy used for assigning jobs to workers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStrategy {
    /// First available worker (default)
    FirstAvailable,
    /// Round-robin across workers
    RoundRobin,
    /// Load-balanced based on worker capacity
    LoadBalanced,
    /// Manual assignment by administrator
    Manual,
    /// Affinity-based (prefer same worker for related jobs)
    Affinity,
}

impl Default for AssignmentStrategy {
    fn default() -> Self {
        Self::FirstAvailable
    }
}

impl std::fmt::Display for AssignmentStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentStrategy::FirstAvailable => write!(f, "first_available"),
            AssignmentStrategy::RoundRobin => write!(f, "round_robin"),
            AssignmentStrategy::LoadBalanced => write!(f, "load_balanced"),
            AssignmentStrategy::Manual => write!(f, "manual"),
            AssignmentStrategy::Affinity => write!(f, "affinity"),
        }
    }
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> i64 {
    crate::common::current_timestamp_or_zero()
}
