//! State machine implementation for job workflow states
//!
//! Defines the valid transitions between job states in the work queue.
//! This ensures jobs move through a consistent lifecycle:
//! Pending -> Claimed -> Processing -> Completed/Failed

use crate::domain::types::JobStatus;
use anyhow::{anyhow, Result};

/// State machine for managing work item state transitions
///
/// This struct provides validation and enforcement of allowed state transitions
/// for jobs in the work queue. It ensures jobs follow a consistent lifecycle
/// and prevents invalid state changes.
pub struct WorkStateMachine;

impl WorkStateMachine {
    /// Validate a state transition is allowed
    ///
    /// Returns Ok(()) if the transition is valid, or an error describing why it's invalid.
    ///
    /// # Arguments
    /// * `from` - Current state of the job
    /// * `to` - Desired new state for the job
    pub fn validate_transition(from: &JobStatus, to: &JobStatus) -> Result<()> {
        use JobStatus::*;

        let valid = match (from, to) {
            // From Pending
            (Pending, Claimed) => true,
            (Pending, Failed) => true, // Can fail during validation
            (Pending, Pending) => true, // Idempotent update

            // From Claimed
            (Claimed, InProgress) => true,
            (Claimed, Failed) => true, // Can fail during claim
            (Claimed, Pending) => true, // Unclaim/release
            (Claimed, Claimed) => true, // Idempotent update

            // From InProgress
            (InProgress, Completed) => true,
            (InProgress, Failed) => true,
            (InProgress, InProgress) => true, // Status update

            // From Completed - terminal state
            (Completed, Completed) => true, // Idempotent update only

            // From Failed - terminal state
            (Failed, Failed) => true, // Idempotent update only
            (Failed, Pending) => true, // Retry

            _ => false,
        };

        if valid {
            Ok(())
        } else {
            Err(anyhow!(
                "Invalid state transition from {:?} to {:?}",
                from, to
            ))
        }
    }

    /// Get all valid next states from a current state
    ///
    /// Returns a vector of states that are valid transitions from the current state.
    /// Useful for UI/API to show available actions.
    pub fn valid_transitions(from: &JobStatus) -> Vec<JobStatus> {
        use JobStatus::*;

        match from {
            Pending => vec![Claimed, Failed, Pending],
            Claimed => vec![InProgress, Failed, Pending, Claimed],
            InProgress => vec![Completed, Failed, InProgress],
            Completed => vec![Completed], // Only idempotent update allowed
            Failed => vec![Failed],       // Only idempotent update allowed
        }
    }

    /// Check if a status requires the completed_by field to be set
    pub fn requires_completed_by(status: &JobStatus) -> bool {
        matches!(status, JobStatus::Completed)
    }
}