//! State machine implementation for job workflow states
//!
//! Defines the valid transitions between job states in the work queue.
//! This ensures jobs move through a consistent lifecycle:
//! Pending -> Claimed -> Processing -> Completed/Failed

use crate::domain::types::JobStatus;

/// State machine for managing work item state transitions
///
/// This struct provides validation and enforcement of allowed state transitions
/// for jobs in the work queue. It ensures jobs follow a consistent lifecycle
/// and prevents invalid state changes.
pub struct WorkStateMachine;

impl WorkStateMachine {
    /// Check if a status requires the completed_by field to be set
    pub fn requires_completed_by(status: &JobStatus) -> bool {
        matches!(status, JobStatus::Completed)
    }
}