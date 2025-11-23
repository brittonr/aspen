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

use crate::work_queue::WorkStatus;

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
    pub fn validate_transition(from: &WorkStatus, to: &WorkStatus) -> Result<(), String> {
        use WorkStatus::*;

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
    pub fn is_terminal(status: &WorkStatus) -> bool {
        matches!(status, WorkStatus::Completed | WorkStatus::Failed)
    }

    /// Check if a status requires setting `completed_by` field
    ///
    /// Only terminal states track which node completed the work.
    pub fn requires_completed_by(status: &WorkStatus) -> bool {
        Self::is_terminal(status)
    }

    /// Get the next valid states from a given state
    ///
    /// Returns a list of states that are valid transitions from the current state.
    pub fn next_valid_states(from: &WorkStatus) -> Vec<WorkStatus> {
        use WorkStatus::*;

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
    use crate::work_queue::WorkStatus;

    #[test]
    fn test_valid_forward_transitions() {
        // Happy path: Pending → Claimed → InProgress → Completed
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Pending,
            &WorkStatus::Claimed
        )
        .is_ok());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Claimed,
            &WorkStatus::InProgress
        )
        .is_ok());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::InProgress,
            &WorkStatus::Completed
        )
        .is_ok());
    }

    #[test]
    fn test_failure_transitions() {
        // Can fail from any non-terminal state
        assert!(
            WorkStateMachine::validate_transition(&WorkStatus::Pending, &WorkStatus::Failed)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&WorkStatus::Claimed, &WorkStatus::Failed)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&WorkStatus::InProgress, &WorkStatus::Failed)
                .is_ok()
        );
    }

    #[test]
    fn test_idempotent_updates() {
        // Same state transitions are always allowed
        assert!(
            WorkStateMachine::validate_transition(&WorkStatus::Pending, &WorkStatus::Pending)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&WorkStatus::Completed, &WorkStatus::Completed)
                .is_ok()
        );
        assert!(
            WorkStateMachine::validate_transition(&WorkStatus::Failed, &WorkStatus::Failed)
                .is_ok()
        );
    }

    #[test]
    fn test_terminal_state_rejection() {
        // Cannot transition from terminal states to different states
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Completed,
            &WorkStatus::Pending
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Failed,
            &WorkStatus::Pending
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Completed,
            &WorkStatus::InProgress
        )
        .is_err());
    }

    #[test]
    fn test_invalid_backward_transitions() {
        // Cannot go backwards in the state machine
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Claimed,
            &WorkStatus::Pending
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::InProgress,
            &WorkStatus::Claimed
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::InProgress,
            &WorkStatus::Pending
        )
        .is_err());
    }

    #[test]
    fn test_invalid_skip_transitions() {
        // Cannot skip states
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Pending,
            &WorkStatus::InProgress
        )
        .is_err());
        assert!(WorkStateMachine::validate_transition(
            &WorkStatus::Pending,
            &WorkStatus::Completed
        )
        .is_err());
    }

    #[test]
    fn test_is_terminal() {
        assert!(!WorkStateMachine::is_terminal(&WorkStatus::Pending));
        assert!(!WorkStateMachine::is_terminal(&WorkStatus::Claimed));
        assert!(!WorkStateMachine::is_terminal(&WorkStatus::InProgress));
        assert!(WorkStateMachine::is_terminal(&WorkStatus::Completed));
        assert!(WorkStateMachine::is_terminal(&WorkStatus::Failed));
    }

    #[test]
    fn test_requires_completed_by() {
        assert!(!WorkStateMachine::requires_completed_by(&WorkStatus::Pending));
        assert!(!WorkStateMachine::requires_completed_by(&WorkStatus::Claimed));
        assert!(!WorkStateMachine::requires_completed_by(
            &WorkStatus::InProgress
        ));
        assert!(WorkStateMachine::requires_completed_by(&WorkStatus::Completed));
        assert!(WorkStateMachine::requires_completed_by(&WorkStatus::Failed));
    }

    #[test]
    fn test_next_valid_states() {
        assert_eq!(
            WorkStateMachine::next_valid_states(&WorkStatus::Pending),
            vec![WorkStatus::Claimed, WorkStatus::Failed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&WorkStatus::Claimed),
            vec![WorkStatus::InProgress, WorkStatus::Failed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&WorkStatus::InProgress),
            vec![WorkStatus::Completed, WorkStatus::Failed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&WorkStatus::Completed),
            vec![WorkStatus::Completed]
        );
        assert_eq!(
            WorkStateMachine::next_valid_states(&WorkStatus::Failed),
            vec![WorkStatus::Failed]
        );
    }
}
