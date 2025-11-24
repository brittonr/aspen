//! Work State Machine - Pure Business Logic
//!
//! This module implements the state transition rules for workflow jobs.
//! It contains no infrastructure dependencies - only business rules that
//! determine valid state transitions.
//!
//! State Machine:
//! ```text
//!   Pending → Claimed → InProgress → Completed/Failed
//!      ↓                                  ↑
//!      └──────────────────────────────────┘
//!           (can also fail directly)
//! ```
//!
//! Terminal States: Completed, Failed (cannot transition out)

use crate::domain::types::JobStatus;

/// Pure business logic for workflow state transitions
///
/// This is a stateless validator with no side effects. All methods are pure functions
/// that take current state and return validation results.
pub struct WorkStateMachine;

impl WorkStateMachine {
    /// Validate a state transition
    ///
    /// Returns `Ok(())` if the transition is valid, `Err(reason)` otherwise.
    ///
    /// # Rules
    /// - Cannot transition from terminal states (Completed/Failed) except idempotent updates
    /// - Pending can go to Claimed or Failed
    /// - Claimed can go to InProgress or Failed
    /// - InProgress can go to Completed or Failed
    /// - Terminal states can only transition to themselves (idempotent)
    #[allow(dead_code)] // Used in tests
    pub fn validate_transition(from: &JobStatus, to: &JobStatus) -> Result<(), String> {
        use JobStatus::*;

        match (from, to) {
            // Idempotent updates (same state) are always allowed
            (current, next) if current == next => Ok(()),

            // Terminal states cannot transition to different states
            (Completed, _) | (Failed, _) => {
                Err(format!("Cannot transition from terminal state {:?}", from))
            }

            // Valid forward transitions
            (Pending, Claimed) => Ok(()),
            (Pending, Failed) => Ok(()), // Can fail without claiming
            (Claimed, InProgress) => Ok(()),
            (Claimed, Failed) => Ok(()), // Can fail after claiming
            (InProgress, Completed) => Ok(()),
            (InProgress, Failed) => Ok(()),

            // Invalid backward/skip transitions
            (current, next) => Err(format!(
                "Invalid state transition: {:?} → {:?}",
                current, next
            )),
        }
    }

    /// Check if a status is a terminal state
    ///
    /// Terminal states cannot transition to other states.
    pub fn is_terminal(status: &JobStatus) -> bool {
        matches!(status, JobStatus::Completed | JobStatus::Failed)
    }

    /// Check if a status requires setting `completed_by` field
    ///
    /// Only terminal states track which node completed the work.
    pub fn requires_completed_by(status: &JobStatus) -> bool {
        Self::is_terminal(status)
    }

    /// Get the next valid states from a given state
    ///
    /// Returns a list of states that are valid transitions from the current state.
    #[allow(dead_code)] // Used in tests
    pub fn next_valid_states(from: &JobStatus) -> Vec<JobStatus> {
        use JobStatus::*;

        match from {
            Pending => vec![Claimed, Failed],
            Claimed => vec![InProgress, Failed],
            InProgress => vec![Completed, Failed],
            Completed => vec![Completed], // Only idempotent update allowed
            Failed => vec![Failed],       // Only idempotent update allowed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_forward_transitions() {
        // Happy path: Pending → Claimed → InProgress → Completed
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Pending,
            &JobStatus::Claimed
        )
        .is_ok());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Claimed,
            &JobStatus::InProgress
        )
        .is_ok());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::InProgress,
            &JobStatus::Completed
        )
        .is_ok());
    }

    #[test]
    fn test_failure_transitions() {
        // Can fail from any non-terminal state
        assert!(
            WorkStateMachine::validate_transition(&JobStatus::Pending, &JobStatus::Failed)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&JobStatus::Claimed, &JobStatus::Failed)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&JobStatus::InProgress, &JobStatus::Failed)
                .is_ok()
        );
    }

    #[test]
    fn test_idempotent_updates() {
        // Same state transitions are always allowed
        assert!(
            WorkStateMachine::validate_transition(&JobStatus::Pending, &JobStatus::Pending)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&JobStatus::Completed, &JobStatus::Completed)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&JobStatus::Failed, &JobStatus::Failed)
                .is_ok()
        );
    }

    #[test]
    fn test_terminal_state_rejection() {
        // Cannot transition from terminal states to different states
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Completed,
            &JobStatus::Pending
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Failed,
            &JobStatus::Pending
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Completed,
            &JobStatus::InProgress
        )
        .is_err());
    }

    #[test]
    fn test_invalid_backward_transitions() {
        // Cannot go backwards in the state machine
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Claimed,
            &JobStatus::Pending
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::InProgress,
            &JobStatus::Claimed
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::InProgress,
            &JobStatus::Pending
        )
        .is_err());
    }

    #[test]
    fn test_invalid_skip_transitions() {
        // Cannot skip states
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Pending,
            &JobStatus::InProgress
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Pending,
            &JobStatus::Completed
        )
        .is_err());
    }

    #[test]
    fn test_is_terminal() {
        assert!(!WorkStateMachine::is_terminal(&JobStatus::Pending));
        assert!(!WorkStateMachine::is_terminal(&JobStatus::Claimed));
        assert!(!WorkStateMachine::is_terminal(&JobStatus::InProgress));
        assert!(WorkStateMachine::is_terminal(&JobStatus::Completed));
        assert!(WorkStateMachine::is_terminal(&JobStatus::Failed));
    }

    #[test]
    fn test_requires_completed_by() {
        assert!(!WorkStateMachine::requires_completed_by(&JobStatus::Pending));
        assert!(!WorkStateMachine::requires_completed_by(&JobStatus::Claimed));
        assert!(!WorkStateMachine::requires_completed_by(
            &JobStatus::InProgress
        ));
        assert!(WorkStateMachine::requires_completed_by(&JobStatus::Completed));
        assert!(WorkStateMachine::requires_completed_by(&JobStatus::Failed));
    }

    #[test]
    fn test_next_valid_states() {
        assert_eq!(
            WorkStateMachine::next_valid_states(&JobStatus::Pending),
            vec![JobStatus::Claimed, JobStatus::Failed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&JobStatus::Claimed),
            vec![JobStatus::InProgress, JobStatus::Failed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&JobStatus::InProgress),
            vec![JobStatus::Completed, JobStatus::Failed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&JobStatus::Completed),
            vec![JobStatus::Completed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&JobStatus::Failed),
            vec![JobStatus::Failed]
        );
    }
}
