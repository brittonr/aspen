//! Job State Machine - Domain Business Rules
//!
//! This module implements state transition validation for jobs in the domain layer.
//! It enforces business rules about valid state transitions, preventing invalid
//! operations like transitioning from Completed back to Pending.
//!
//! State Machine:
//! ```text
//!   Pending → Claimed → InProgress → Completed
//!      ↓         ↓           ↓
//!      └─────────┴───────────┴──────> Failed
//!
//!   Failed ────> Pending (retry allowed)
//! ```
//!
//! Terminal State: Completed (cannot transition except to itself)
//! Retriable State: Failed (can be reset to Pending for retry)

use crate::domain::types::JobStatus;

/// Domain error for invalid state transitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionError {
    pub from: JobStatus,
    pub to: JobStatus,
    pub reason: String,
}

impl std::fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid state transition from {} to {}: {}",
            self.from, self.to, self.reason
        )
    }
}

impl std::error::Error for StateTransitionError {}

/// Result type for state machine operations
pub type StateResult<T> = Result<T, StateTransitionError>;

/// Pure business logic for job state transitions
///
/// This is a stateless validator with no side effects. All methods are pure
/// functions that enforce domain business rules.
pub struct JobStateMachine;

impl JobStateMachine {
    /// Validate a state transition
    ///
    /// Returns `Ok(())` if the transition is valid according to business rules.
    ///
    /// # Business Rules
    ///
    /// **Forward transitions (happy path):**
    /// - Pending → Claimed (worker takes ownership)
    /// - Claimed → InProgress (worker starts execution)
    /// - InProgress → Completed (execution succeeds)
    ///
    /// **Failure transitions (error handling):**
    /// - Pending → Failed (validation failure before claiming)
    /// - Claimed → Failed (worker decides not to process)
    /// - InProgress → Failed (execution error)
    ///
    /// **Retry transitions:**
    /// - Failed → Pending (reset for retry)
    ///
    /// **Idempotent transitions:**
    /// - Any state to itself (safe to call multiple times)
    ///
    /// **Terminal state:**
    /// - Completed cannot transition to any other state
    ///   (immutable success record)
    ///
    /// # Examples
    ///
    /// ```
    /// use mvm_ci::domain::state_machine::JobStateMachine;
    /// use mvm_ci::domain::types::JobStatus;
    ///
    /// // Valid transition
    /// assert!(JobStateMachine::validate_transition(
    ///     JobStatus::Pending,
    ///     JobStatus::Claimed
    /// ).is_ok());
    ///
    /// // Invalid transition
    /// assert!(JobStateMachine::validate_transition(
    ///     JobStatus::Completed,
    ///     JobStatus::Pending
    /// ).is_err());
    /// ```
    pub fn validate_transition(from: JobStatus, to: JobStatus) -> StateResult<()> {
        use JobStatus::*;

        match (from, to) {
            // Idempotent updates (same state) are always allowed
            (current, next) if current == next => Ok(()),

            // Terminal state: Completed cannot transition to other states
            (Completed, _) => Err(StateTransitionError {
                from,
                to,
                reason: "Completed jobs are immutable (terminal state)".to_string(),
            }),

            // Valid forward transitions (happy path)
            (Pending, Claimed) => Ok(()),
            (Claimed, InProgress) => Ok(()),
            (InProgress, Completed) => Ok(()),

            // Valid failure transitions (can fail from any non-terminal state)
            (Pending, Failed) => Ok(()),
            (Claimed, Failed) => Ok(()),
            (InProgress, Failed) => Ok(()),

            // Valid retry transition
            (Failed, Pending) => Ok(()),

            // Invalid backward transitions
            (Claimed, Pending) => Err(StateTransitionError {
                from,
                to,
                reason: "Cannot unclaim a job (backward transition)".to_string(),
            }),
            (InProgress, Claimed) | (InProgress, Pending) => Err(StateTransitionError {
                from,
                to,
                reason: "Cannot revert job that is in progress (backward transition)".to_string(),
            }),

            // Invalid skip transitions
            (Pending, InProgress) => Err(StateTransitionError {
                from,
                to,
                reason: "Must claim job before starting work (skipped Claimed state)".to_string(),
            }),
            (Pending, Completed) => Err(StateTransitionError {
                from,
                to,
                reason: "Must claim and start job before completing (skipped states)"
                    .to_string(),
            }),
            (Claimed, Completed) => Err(StateTransitionError {
                from,
                to,
                reason: "Must start job before completing (skipped InProgress state)".to_string(),
            }),

            // Invalid transitions from Failed (except retry)
            (Failed, Claimed) | (Failed, InProgress) | (Failed, Completed) => {
                Err(StateTransitionError {
                    from,
                    to,
                    reason: "Failed jobs can only be retried (set to Pending) or remain Failed"
                        .to_string(),
                })
            }

            // Catch-all for any remaining invalid transitions
            // This shouldn't be reached given the exhaustive cases above,
            // but Rust requires it for completeness
            (from_state, to_state) => {
                Err(StateTransitionError {
                    from: from_state,
                    to: to_state,
                    reason: format!("Invalid transition from {:?} to {:?}", from_state, to_state),
                })
            }
        }
    }

    /// Check if a status is terminal (immutable)
    ///
    /// Terminal states represent final outcomes that cannot be changed.
    /// Currently only `Completed` is terminal - failed jobs can be retried.
    pub fn is_terminal(status: JobStatus) -> bool {
        matches!(status, JobStatus::Completed)
    }

    /// Check if a status is retriable
    ///
    /// Retriable states can be reset to Pending to try again.
    pub fn is_retriable(status: JobStatus) -> bool {
        matches!(status, JobStatus::Failed)
    }

    /// Check if a status requires a worker assignment
    ///
    /// Claimed, InProgress, Completed, and Failed states should track
    /// which worker is responsible.
    pub fn requires_worker(status: JobStatus) -> bool {
        matches!(
            status,
            JobStatus::Claimed | JobStatus::InProgress | JobStatus::Completed | JobStatus::Failed
        )
    }

    /// Get the next valid states from a given state
    ///
    /// Returns all states that are valid transitions from the current state.
    /// Useful for UI state machines, validation, and debugging.
    pub fn next_valid_states(from: JobStatus) -> Vec<JobStatus> {
        use JobStatus::*;

        match from {
            Pending => vec![Pending, Claimed, Failed],
            Claimed => vec![Claimed, InProgress, Failed],
            InProgress => vec![InProgress, Completed, Failed],
            Completed => vec![Completed], // Terminal - only idempotent
            Failed => vec![Failed, Pending], // Can retry
        }
    }

    /// Check if a transition is a retry operation
    pub fn is_retry_transition(from: JobStatus, to: JobStatus) -> bool {
        matches!((from, to), (JobStatus::Failed, JobStatus::Pending))
    }

    /// Check if a transition represents forward progress
    pub fn is_forward_transition(from: JobStatus, to: JobStatus) -> bool {
        matches!(
            (from, to),
            (JobStatus::Pending, JobStatus::Claimed)
                | (JobStatus::Claimed, JobStatus::InProgress)
                | (JobStatus::InProgress, JobStatus::Completed)
        )
    }

    /// Check if a transition represents a failure
    pub fn is_failure_transition(from: JobStatus, to: JobStatus) -> bool {
        to == JobStatus::Failed && from != JobStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_forward_transitions() {
        // Happy path: Pending → Claimed → InProgress → Completed
        assert!(JobStateMachine::validate_transition(JobStatus::Pending, JobStatus::Claimed)
            .is_ok());
        assert!(JobStateMachine::validate_transition(JobStatus::Claimed, JobStatus::InProgress)
            .is_ok());
        assert!(
            JobStateMachine::validate_transition(JobStatus::InProgress, JobStatus::Completed)
                .is_ok()
        );
    }

    #[test]
    fn test_failure_transitions() {
        // Can fail from any non-terminal state
        assert!(
            JobStateMachine::validate_transition(JobStatus::Pending, JobStatus::Failed).is_ok()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::Claimed, JobStatus::Failed).is_ok()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::InProgress, JobStatus::Failed).is_ok()
        );
    }

    #[test]
    fn test_retry_transition() {
        // Failed jobs can be reset to Pending for retry
        assert!(
            JobStateMachine::validate_transition(JobStatus::Failed, JobStatus::Pending).is_ok()
        );
    }

    #[test]
    fn test_idempotent_updates() {
        // Same state transitions are always allowed
        assert!(
            JobStateMachine::validate_transition(JobStatus::Pending, JobStatus::Pending).is_ok()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::Completed, JobStatus::Completed)
                .is_ok()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::Failed, JobStatus::Failed).is_ok()
        );
    }

    #[test]
    fn test_terminal_state_rejection() {
        // Cannot transition from Completed to any other state
        let result =
            JobStateMachine::validate_transition(JobStatus::Completed, JobStatus::Pending);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .reason
            .contains("immutable (terminal state)"));

        assert!(
            JobStateMachine::validate_transition(JobStatus::Completed, JobStatus::InProgress)
                .is_err()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::Completed, JobStatus::Failed).is_err()
        );
    }

    #[test]
    fn test_invalid_backward_transitions() {
        // Cannot go backwards in the state machine
        let result = JobStateMachine::validate_transition(JobStatus::Claimed, JobStatus::Pending);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("unclaim"));

        let result =
            JobStateMachine::validate_transition(JobStatus::InProgress, JobStatus::Claimed);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("revert"));

        assert!(
            JobStateMachine::validate_transition(JobStatus::InProgress, JobStatus::Pending)
                .is_err()
        );
    }

    #[test]
    fn test_invalid_skip_transitions() {
        // Cannot skip states in forward direction
        let result =
            JobStateMachine::validate_transition(JobStatus::Pending, JobStatus::InProgress);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("skipped Claimed"));

        let result =
            JobStateMachine::validate_transition(JobStatus::Pending, JobStatus::Completed);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("skipped states"));

        let result =
            JobStateMachine::validate_transition(JobStatus::Claimed, JobStatus::Completed);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("skipped InProgress"));
    }

    #[test]
    fn test_invalid_failed_transitions() {
        // Failed can only go to Pending (retry) or stay Failed
        assert!(
            JobStateMachine::validate_transition(JobStatus::Failed, JobStatus::Claimed).is_err()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::Failed, JobStatus::InProgress)
                .is_err()
        );
        assert!(
            JobStateMachine::validate_transition(JobStatus::Failed, JobStatus::Completed).is_err()
        );
    }

    #[test]
    fn test_is_terminal() {
        assert!(!JobStateMachine::is_terminal(JobStatus::Pending));
        assert!(!JobStateMachine::is_terminal(JobStatus::Claimed));
        assert!(!JobStateMachine::is_terminal(JobStatus::InProgress));
        assert!(JobStateMachine::is_terminal(JobStatus::Completed));
        assert!(!JobStateMachine::is_terminal(JobStatus::Failed)); // Failed is retriable
    }

    #[test]
    fn test_is_retriable() {
        assert!(!JobStateMachine::is_retriable(JobStatus::Pending));
        assert!(!JobStateMachine::is_retriable(JobStatus::Completed));
        assert!(JobStateMachine::is_retriable(JobStatus::Failed));
    }

    #[test]
    fn test_requires_worker() {
        assert!(!JobStateMachine::requires_worker(JobStatus::Pending));
        assert!(JobStateMachine::requires_worker(JobStatus::Claimed));
        assert!(JobStateMachine::requires_worker(JobStatus::InProgress));
        assert!(JobStateMachine::requires_worker(JobStatus::Completed));
        assert!(JobStateMachine::requires_worker(JobStatus::Failed));
    }

    #[test]
    fn test_next_valid_states() {
        assert_eq!(
            JobStateMachine::next_valid_states(JobStatus::Pending),
            vec![JobStatus::Pending, JobStatus::Claimed, JobStatus::Failed]
        );
        assert_eq!(
            JobStateMachine::next_valid_states(JobStatus::Claimed),
            vec![
                JobStatus::Claimed,
                JobStatus::InProgress,
                JobStatus::Failed
            ]
        );
        assert_eq!(
            JobStateMachine::next_valid_states(JobStatus::InProgress),
            vec![
                JobStatus::InProgress,
                JobStatus::Completed,
                JobStatus::Failed
            ]
        );
        assert_eq!(
            JobStateMachine::next_valid_states(JobStatus::Completed),
            vec![JobStatus::Completed]
        );
        assert_eq!(
            JobStateMachine::next_valid_states(JobStatus::Failed),
            vec![JobStatus::Failed, JobStatus::Pending]
        );
    }

    #[test]
    fn test_transition_type_checks() {
        // Retry
        assert!(JobStateMachine::is_retry_transition(
            JobStatus::Failed,
            JobStatus::Pending
        ));
        assert!(!JobStateMachine::is_retry_transition(
            JobStatus::Pending,
            JobStatus::Claimed
        ));

        // Forward
        assert!(JobStateMachine::is_forward_transition(
            JobStatus::Pending,
            JobStatus::Claimed
        ));
        assert!(JobStateMachine::is_forward_transition(
            JobStatus::Claimed,
            JobStatus::InProgress
        ));
        assert!(!JobStateMachine::is_forward_transition(
            JobStatus::Pending,
            JobStatus::Failed
        ));

        // Failure
        assert!(JobStateMachine::is_failure_transition(
            JobStatus::Pending,
            JobStatus::Failed
        ));
        assert!(JobStateMachine::is_failure_transition(
            JobStatus::InProgress,
            JobStatus::Failed
        ));
        assert!(!JobStateMachine::is_failure_transition(
            JobStatus::Failed,
            JobStatus::Failed
        ));
    }
}
