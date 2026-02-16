//! Pure workflow condition evaluation functions.
//!
//! These functions evaluate workflow transition conditions without
//! side effects. The async shell layer handles actual state transitions
//! and persistence.
//!
//! # Tiger Style
//!
//! - Pure evaluation functions
//! - Deterministic behavior
//! - No I/O or system calls

/// Result of evaluating a workflow transition condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionResult {
    /// Condition is satisfied, transition should occur.
    Satisfied,
    /// Condition is not satisfied.
    NotSatisfied,
    /// Condition cannot be evaluated (missing data).
    Indeterminate,
}

/// Check if all jobs in a workflow step succeeded.
///
/// # Arguments
///
/// * `total_jobs` - Total number of jobs in the step
/// * `succeeded_jobs` - Number of jobs that succeeded
/// * `failed_jobs` - Number of jobs that failed
///
/// # Returns
///
/// Condition result based on job outcomes.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{ConditionResult, check_all_success};
///
/// // All succeeded
/// assert_eq!(check_all_success(3, 3, 0), ConditionResult::Satisfied);
///
/// // Some failed
/// assert_eq!(check_all_success(3, 2, 1), ConditionResult::NotSatisfied);
///
/// // Still running
/// assert_eq!(check_all_success(3, 2, 0), ConditionResult::Indeterminate);
/// ```
#[inline]
pub const fn check_all_success(total_jobs: u32, succeeded_jobs: u32, failed_jobs: u32) -> ConditionResult {
    if failed_jobs > 0 {
        ConditionResult::NotSatisfied
    } else if succeeded_jobs == total_jobs {
        ConditionResult::Satisfied
    } else {
        ConditionResult::Indeterminate
    }
}

/// Check if any job in a workflow step failed.
///
/// # Arguments
///
/// * `total_jobs` - Total number of jobs in the step
/// * `succeeded_jobs` - Number of jobs that succeeded
/// * `failed_jobs` - Number of jobs that failed
///
/// # Returns
///
/// Condition result based on job outcomes.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{ConditionResult, check_any_failed};
///
/// // Some failed
/// assert_eq!(check_any_failed(3, 2, 1), ConditionResult::Satisfied);
///
/// // All succeeded
/// assert_eq!(check_any_failed(3, 3, 0), ConditionResult::NotSatisfied);
///
/// // Still running, no failures yet
/// assert_eq!(check_any_failed(3, 2, 0), ConditionResult::Indeterminate);
/// ```
#[inline]
pub const fn check_any_failed(total_jobs: u32, succeeded_jobs: u32, failed_jobs: u32) -> ConditionResult {
    if failed_jobs > 0 {
        ConditionResult::Satisfied
    } else if succeeded_jobs == total_jobs {
        ConditionResult::NotSatisfied
    } else {
        ConditionResult::Indeterminate
    }
}

/// Check if the success rate meets a threshold.
///
/// # Arguments
///
/// * `total_jobs` - Total number of jobs in the step
/// * `succeeded_jobs` - Number of jobs that succeeded
/// * `completed_jobs` - Number of jobs completed (succeeded + failed)
/// * `threshold` - Required success rate (0.0 to 1.0)
///
/// # Returns
///
/// Condition result based on success rate.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{ConditionResult, check_success_rate};
///
/// // 80% success rate, threshold 70%
/// assert_eq!(check_success_rate(10, 8, 10, 0.7), ConditionResult::Satisfied);
///
/// // 50% success rate, threshold 70%
/// assert_eq!(check_success_rate(10, 5, 10, 0.7), ConditionResult::NotSatisfied);
///
/// // Not all complete yet
/// assert_eq!(check_success_rate(10, 5, 7, 0.7), ConditionResult::Indeterminate);
/// ```
#[inline]
pub fn check_success_rate(
    total_jobs: u32,
    succeeded_jobs: u32,
    completed_jobs: u32,
    threshold: f32,
) -> ConditionResult {
    if completed_jobs < total_jobs {
        return ConditionResult::Indeterminate;
    }

    if total_jobs == 0 {
        return ConditionResult::Satisfied;
    }

    let rate = succeeded_jobs as f32 / total_jobs as f32;
    if rate >= threshold {
        ConditionResult::Satisfied
    } else {
        ConditionResult::NotSatisfied
    }
}

/// Check if a specific job succeeded.
///
/// # Arguments
///
/// * `job_succeeded` - Whether the specific job succeeded
/// * `job_completed` - Whether the specific job has completed
///
/// # Returns
///
/// Condition result based on job outcome.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{ConditionResult, check_job_success};
///
/// assert_eq!(check_job_success(true, true), ConditionResult::Satisfied);
/// assert_eq!(check_job_success(false, true), ConditionResult::NotSatisfied);
/// assert_eq!(check_job_success(false, false), ConditionResult::Indeterminate);
/// ```
#[inline]
pub const fn check_job_success(job_succeeded: bool, job_completed: bool) -> ConditionResult {
    if !job_completed {
        ConditionResult::Indeterminate
    } else if job_succeeded {
        ConditionResult::Satisfied
    } else {
        ConditionResult::NotSatisfied
    }
}

/// Check if a specific job failed.
///
/// # Arguments
///
/// * `job_failed` - Whether the specific job failed
/// * `job_completed` - Whether the specific job has completed
///
/// # Returns
///
/// Condition result based on job outcome.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{ConditionResult, check_job_failed};
///
/// assert_eq!(check_job_failed(true, true), ConditionResult::Satisfied);
/// assert_eq!(check_job_failed(false, true), ConditionResult::NotSatisfied);
/// assert_eq!(check_job_failed(false, false), ConditionResult::Indeterminate);
/// ```
#[inline]
pub const fn check_job_failed(job_failed: bool, job_completed: bool) -> ConditionResult {
    if !job_completed {
        ConditionResult::Indeterminate
    } else if job_failed {
        ConditionResult::Satisfied
    } else {
        ConditionResult::NotSatisfied
    }
}

/// Compute the trimmed history size after removing oldest entries.
///
/// When history exceeds max size, remove oldest 10% to make room.
///
/// # Arguments
///
/// * `current_size` - Current number of history entries
/// * `max_size` - Maximum allowed history entries
///
/// # Returns
///
/// Number of entries to keep (or `current_size` if within limit).
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_history_trim_size;
///
/// // Under limit, keep all
/// assert_eq!(compute_history_trim_size(50, 100), 50);
///
/// // At limit, trim to 90%
/// assert_eq!(compute_history_trim_size(100, 100), 90);
///
/// // Over limit, trim to 90% of max
/// assert_eq!(compute_history_trim_size(150, 100), 90);
/// ```
#[inline]
pub const fn compute_history_trim_size(current_size: u32, max_size: u32) -> u32 {
    if current_size < max_size {
        current_size
    } else {
        // Keep 90% of max
        (max_size * 9) / 10
    }
}

/// Check if workflow state is terminal.
///
/// # Arguments
///
/// * `state` - The current state name
/// * `terminal_states` - List of terminal state names
///
/// # Returns
///
/// `true` if the state is terminal.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::is_workflow_terminal;
///
/// let terminals = &["completed", "failed", "cancelled"];
///
/// assert!(is_workflow_terminal("completed", terminals));
/// assert!(is_workflow_terminal("failed", terminals));
/// assert!(!is_workflow_terminal("running", terminals));
/// ```
#[inline]
pub fn is_workflow_terminal(state: &str, terminal_states: &[&str]) -> bool {
    terminal_states.contains(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // check_all_success tests
    // ========================================================================

    #[test]
    fn test_all_success_satisfied() {
        assert_eq!(check_all_success(3, 3, 0), ConditionResult::Satisfied);
    }

    #[test]
    fn test_all_success_failed() {
        assert_eq!(check_all_success(3, 2, 1), ConditionResult::NotSatisfied);
    }

    #[test]
    fn test_all_success_running() {
        assert_eq!(check_all_success(3, 2, 0), ConditionResult::Indeterminate);
    }

    #[test]
    fn test_all_success_empty() {
        assert_eq!(check_all_success(0, 0, 0), ConditionResult::Satisfied);
    }

    // ========================================================================
    // check_any_failed tests
    // ========================================================================

    #[test]
    fn test_any_failed_satisfied() {
        assert_eq!(check_any_failed(3, 2, 1), ConditionResult::Satisfied);
    }

    #[test]
    fn test_any_failed_not_satisfied() {
        assert_eq!(check_any_failed(3, 3, 0), ConditionResult::NotSatisfied);
    }

    #[test]
    fn test_any_failed_running() {
        assert_eq!(check_any_failed(3, 2, 0), ConditionResult::Indeterminate);
    }

    // ========================================================================
    // check_success_rate tests
    // ========================================================================

    #[test]
    fn test_success_rate_above_threshold() {
        assert_eq!(check_success_rate(10, 8, 10, 0.7), ConditionResult::Satisfied);
    }

    #[test]
    fn test_success_rate_below_threshold() {
        assert_eq!(check_success_rate(10, 5, 10, 0.7), ConditionResult::NotSatisfied);
    }

    #[test]
    fn test_success_rate_incomplete() {
        assert_eq!(check_success_rate(10, 5, 7, 0.7), ConditionResult::Indeterminate);
    }

    #[test]
    fn test_success_rate_empty() {
        assert_eq!(check_success_rate(0, 0, 0, 0.7), ConditionResult::Satisfied);
    }

    #[test]
    fn test_success_rate_exact_threshold() {
        assert_eq!(check_success_rate(10, 7, 10, 0.7), ConditionResult::Satisfied);
    }

    // ========================================================================
    // check_job_success tests
    // ========================================================================

    #[test]
    fn test_job_success_satisfied() {
        assert_eq!(check_job_success(true, true), ConditionResult::Satisfied);
    }

    #[test]
    fn test_job_success_failed() {
        assert_eq!(check_job_success(false, true), ConditionResult::NotSatisfied);
    }

    #[test]
    fn test_job_success_running() {
        assert_eq!(check_job_success(false, false), ConditionResult::Indeterminate);
    }

    // ========================================================================
    // check_job_failed tests
    // ========================================================================

    #[test]
    fn test_job_failed_satisfied() {
        assert_eq!(check_job_failed(true, true), ConditionResult::Satisfied);
    }

    #[test]
    fn test_job_failed_not_satisfied() {
        assert_eq!(check_job_failed(false, true), ConditionResult::NotSatisfied);
    }

    #[test]
    fn test_job_failed_running() {
        assert_eq!(check_job_failed(false, false), ConditionResult::Indeterminate);
    }

    // ========================================================================
    // compute_history_trim_size tests
    // ========================================================================

    #[test]
    fn test_trim_under_limit() {
        assert_eq!(compute_history_trim_size(50, 100), 50);
    }

    #[test]
    fn test_trim_at_limit() {
        assert_eq!(compute_history_trim_size(100, 100), 90);
    }

    #[test]
    fn test_trim_over_limit() {
        assert_eq!(compute_history_trim_size(150, 100), 90);
    }

    // ========================================================================
    // is_workflow_terminal tests
    // ========================================================================

    #[test]
    fn test_terminal_true() {
        let terminals = &["completed", "failed", "cancelled"];
        assert!(is_workflow_terminal("completed", terminals));
        assert!(is_workflow_terminal("failed", terminals));
    }

    #[test]
    fn test_terminal_false() {
        let terminals = &["completed", "failed", "cancelled"];
        assert!(!is_workflow_terminal("running", terminals));
        assert!(!is_workflow_terminal("pending", terminals));
    }

    #[test]
    fn test_terminal_empty_list() {
        let terminals: &[&str] = &[];
        assert!(!is_workflow_terminal("completed", terminals));
    }
}
