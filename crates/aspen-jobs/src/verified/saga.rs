//! Pure saga state machine functions.
//!
//! These functions handle saga state transitions and phase computations
//! without side effects. The async shell layer manages persistence and
//! async execution.
//!
//! # Tiger Style
//!
//! - State transitions are explicit and validated
//! - No I/O or system calls
//! - Deterministic behavior

/// Saga execution phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaPhase {
    /// Saga has not started.
    NotStarted,
    /// Executing forward at the given step index.
    Executing(usize),
    /// All steps completed successfully.
    Completed,
    /// Compensating at the given step index (counting down).
    Compensating(usize),
    /// All compensations completed.
    CompensationCompleted,
    /// Compensation failed at the given step.
    CompensationFailed(usize),
}

/// Compute the next phase after a successful step execution.
///
/// # Arguments
///
/// * `current_step` - Index of the step that just completed (0-indexed)
/// * `total_steps` - Total number of steps in the saga
///
/// # Returns
///
/// The next saga phase.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{SagaPhase, compute_next_phase_after_success};
///
/// // Completed step 0 of 3 -> move to step 1
/// assert_eq!(
///     compute_next_phase_after_success(0, 3),
///     SagaPhase::Executing(1)
/// );
///
/// // Completed step 2 (last) of 3 -> saga completed
/// assert_eq!(
///     compute_next_phase_after_success(2, 3),
///     SagaPhase::Completed
/// );
/// ```
#[inline]
pub const fn compute_next_phase_after_success(current_step: usize, total_steps: usize) -> SagaPhase {
    let next_step = current_step + 1;
    if next_step >= total_steps {
        SagaPhase::Completed
    } else {
        SagaPhase::Executing(next_step)
    }
}

/// Compute the initial compensation phase after a step failure.
///
/// Compensation starts from the step before the failed step (LIFO order).
///
/// # Arguments
///
/// * `failed_step` - Index of the step that failed (0-indexed)
///
/// # Returns
///
/// The initial compensation phase, or `CompensationCompleted` if no
/// previous steps to compensate.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{SagaPhase, compute_initial_compensation_phase};
///
/// // Failed at step 3 -> start compensating at step 2
/// assert_eq!(
///     compute_initial_compensation_phase(3),
///     SagaPhase::Compensating(2)
/// );
///
/// // Failed at step 0 -> nothing to compensate
/// assert_eq!(
///     compute_initial_compensation_phase(0),
///     SagaPhase::CompensationCompleted
/// );
/// ```
#[inline]
pub const fn compute_initial_compensation_phase(failed_step: usize) -> SagaPhase {
    if failed_step == 0 {
        SagaPhase::CompensationCompleted
    } else {
        SagaPhase::Compensating(failed_step - 1)
    }
}

/// Compute the next compensation phase after a successful compensation.
///
/// # Arguments
///
/// * `current_compensation_step` - Index of the step just compensated
///
/// # Returns
///
/// The next compensation phase.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{SagaPhase, compute_next_compensation_phase};
///
/// // Compensated step 2 -> move to step 1
/// assert_eq!(
///     compute_next_compensation_phase(2),
///     SagaPhase::Compensating(1)
/// );
///
/// // Compensated step 0 (first) -> compensation completed
/// assert_eq!(
///     compute_next_compensation_phase(0),
///     SagaPhase::CompensationCompleted
/// );
/// ```
#[inline]
pub const fn compute_next_compensation_phase(current_compensation_step: usize) -> SagaPhase {
    if current_compensation_step == 0 {
        SagaPhase::CompensationCompleted
    } else {
        SagaPhase::Compensating(current_compensation_step - 1)
    }
}

/// Check if a saga phase is terminal (no more transitions possible).
///
/// # Arguments
///
/// * `phase` - The current saga phase
///
/// # Returns
///
/// `true` if the phase is terminal.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{SagaPhase, is_saga_phase_terminal};
///
/// assert!(is_saga_phase_terminal(SagaPhase::Completed));
/// assert!(is_saga_phase_terminal(SagaPhase::CompensationCompleted));
/// assert!(is_saga_phase_terminal(SagaPhase::CompensationFailed(2)));
///
/// assert!(!is_saga_phase_terminal(SagaPhase::NotStarted));
/// assert!(!is_saga_phase_terminal(SagaPhase::Executing(1)));
/// assert!(!is_saga_phase_terminal(SagaPhase::Compensating(1)));
/// ```
#[inline]
pub const fn is_saga_phase_terminal(phase: SagaPhase) -> bool {
    matches!(phase, SagaPhase::Completed | SagaPhase::CompensationCompleted | SagaPhase::CompensationFailed(_))
}

/// Check if a saga is in a successful terminal state.
///
/// # Arguments
///
/// * `phase` - The current saga phase
///
/// # Returns
///
/// `true` if the saga completed successfully (all steps executed).
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{SagaPhase, is_saga_successful};
///
/// assert!(is_saga_successful(SagaPhase::Completed));
/// assert!(!is_saga_successful(SagaPhase::CompensationCompleted));
/// assert!(!is_saga_successful(SagaPhase::Executing(1)));
/// ```
#[inline]
pub const fn is_saga_successful(phase: SagaPhase) -> bool {
    matches!(phase, SagaPhase::Completed)
}

/// Compute compensation retry delay using exponential backoff.
///
/// # Arguments
///
/// * `attempt` - Current retry attempt (0-indexed)
/// * `base_delay_ms` - Base delay in milliseconds
/// * `max_attempts` - Maximum number of retry attempts
///
/// # Returns
///
/// `Some(delay_ms)` if retry should be attempted, `None` if max reached.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_compensation_retry_delay_ms;
///
/// // First retry: 100ms
/// assert_eq!(compute_compensation_retry_delay_ms(0, 100, 5), Some(100));
///
/// // Second retry: 200ms
/// assert_eq!(compute_compensation_retry_delay_ms(1, 100, 5), Some(200));
///
/// // Exceeded max attempts
/// assert_eq!(compute_compensation_retry_delay_ms(5, 100, 5), None);
/// ```
#[inline]
pub fn compute_compensation_retry_delay_ms(attempt: u32, base_delay_ms: u64, max_attempts: u32) -> Option<u64> {
    if attempt >= max_attempts {
        return None;
    }

    // Exponential backoff: base * 2^attempt
    let multiplier = 1u64.checked_shl(attempt)?;
    Some(base_delay_ms.saturating_mul(multiplier))
}

/// Count the number of steps requiring compensation.
///
/// # Arguments
///
/// * `executed_steps` - Slice of booleans indicating which steps were executed
/// * `requires_compensation` - Slice of booleans indicating which steps need compensation
///
/// # Returns
///
/// The number of steps that need compensation.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::count_compensatable_steps;
///
/// let executed = &[true, true, true, false];
/// let requires_comp = &[true, false, true, true];
///
/// // Steps 0 and 2 were executed and require compensation
/// assert_eq!(count_compensatable_steps(executed, requires_comp), 2);
/// ```
#[inline]
pub fn count_compensatable_steps(executed_steps: &[bool], requires_compensation: &[bool]) -> usize {
    executed_steps
        .iter()
        .zip(requires_compensation.iter())
        .filter(|(executed, requires)| **executed && **requires)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // compute_next_phase_after_success tests
    // ========================================================================

    #[test]
    fn test_next_phase_middle() {
        assert_eq!(compute_next_phase_after_success(0, 3), SagaPhase::Executing(1));
        assert_eq!(compute_next_phase_after_success(1, 3), SagaPhase::Executing(2));
    }

    #[test]
    fn test_next_phase_last() {
        assert_eq!(compute_next_phase_after_success(2, 3), SagaPhase::Completed);
    }

    #[test]
    fn test_next_phase_single_step() {
        assert_eq!(compute_next_phase_after_success(0, 1), SagaPhase::Completed);
    }

    // ========================================================================
    // compute_initial_compensation_phase tests
    // ========================================================================

    #[test]
    fn test_initial_compensation_from_middle() {
        assert_eq!(compute_initial_compensation_phase(3), SagaPhase::Compensating(2));
    }

    #[test]
    fn test_initial_compensation_from_first() {
        assert_eq!(compute_initial_compensation_phase(0), SagaPhase::CompensationCompleted);
    }

    #[test]
    fn test_initial_compensation_from_second() {
        assert_eq!(compute_initial_compensation_phase(1), SagaPhase::Compensating(0));
    }

    // ========================================================================
    // compute_next_compensation_phase tests
    // ========================================================================

    #[test]
    fn test_next_compensation_middle() {
        assert_eq!(compute_next_compensation_phase(2), SagaPhase::Compensating(1));
    }

    #[test]
    fn test_next_compensation_last() {
        assert_eq!(compute_next_compensation_phase(0), SagaPhase::CompensationCompleted);
    }

    // ========================================================================
    // is_saga_phase_terminal tests
    // ========================================================================

    #[test]
    fn test_terminal_phases() {
        assert!(is_saga_phase_terminal(SagaPhase::Completed));
        assert!(is_saga_phase_terminal(SagaPhase::CompensationCompleted));
        assert!(is_saga_phase_terminal(SagaPhase::CompensationFailed(2)));
    }

    #[test]
    fn test_non_terminal_phases() {
        assert!(!is_saga_phase_terminal(SagaPhase::NotStarted));
        assert!(!is_saga_phase_terminal(SagaPhase::Executing(1)));
        assert!(!is_saga_phase_terminal(SagaPhase::Compensating(1)));
    }

    // ========================================================================
    // is_saga_successful tests
    // ========================================================================

    #[test]
    fn test_successful() {
        assert!(is_saga_successful(SagaPhase::Completed));
    }

    #[test]
    fn test_not_successful() {
        assert!(!is_saga_successful(SagaPhase::CompensationCompleted));
        assert!(!is_saga_successful(SagaPhase::CompensationFailed(0)));
        assert!(!is_saga_successful(SagaPhase::NotStarted));
    }

    // ========================================================================
    // compute_compensation_retry_delay_ms tests
    // ========================================================================

    #[test]
    fn test_compensation_retry_delays() {
        assert_eq!(compute_compensation_retry_delay_ms(0, 100, 5), Some(100));
        assert_eq!(compute_compensation_retry_delay_ms(1, 100, 5), Some(200));
        assert_eq!(compute_compensation_retry_delay_ms(2, 100, 5), Some(400));
    }

    #[test]
    fn test_compensation_retry_exceeded() {
        assert_eq!(compute_compensation_retry_delay_ms(5, 100, 5), None);
        assert_eq!(compute_compensation_retry_delay_ms(10, 100, 5), None);
    }

    // ========================================================================
    // count_compensatable_steps tests
    // ========================================================================

    #[test]
    fn test_count_compensatable() {
        let executed = &[true, true, true, false];
        let requires = &[true, false, true, true];
        assert_eq!(count_compensatable_steps(executed, requires), 2);
    }

    #[test]
    fn test_count_none_executed() {
        let executed = &[false, false, false];
        let requires = &[true, true, true];
        assert_eq!(count_compensatable_steps(executed, requires), 0);
    }

    #[test]
    fn test_count_none_require() {
        let executed = &[true, true, true];
        let requires = &[false, false, false];
        assert_eq!(count_compensatable_steps(executed, requires), 0);
    }

    #[test]
    fn test_count_empty() {
        let executed: &[bool] = &[];
        let requires: &[bool] = &[];
        assert_eq!(count_compensatable_steps(executed, requires), 0);
    }
}
