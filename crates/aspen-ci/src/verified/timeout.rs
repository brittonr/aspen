//! Pure timeout and deadline calculation functions.
//!
//! These functions compute timeouts and deadlines without accessing
//! system time. Time is passed as an explicit parameter.
//!
//! # Tiger Style
//!
//! - All time values in milliseconds (u64)
//! - Saturating arithmetic prevents overflow
//! - No I/O or system calls

/// Compute the deadline for a step/job given start time and timeout.
///
/// # Arguments
///
/// * `start_time_ms` - When the step started (Unix timestamp in milliseconds)
/// * `timeout_secs` - Timeout duration in seconds
///
/// # Returns
///
/// The deadline in milliseconds (Unix timestamp).
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_deadline_ms;
///
/// // Started at 1000ms, 30 second timeout
/// assert_eq!(compute_deadline_ms(1000, 30), 31000);
///
/// // Overflow protection
/// assert_eq!(compute_deadline_ms(u64::MAX - 1000, 30), u64::MAX);
/// ```
#[inline]
pub fn compute_deadline_ms(start_time_ms: u64, timeout_secs: u64) -> u64 {
    let timeout_ms = timeout_secs.saturating_mul(1000);
    start_time_ms.saturating_add(timeout_ms)
}

/// Check if a deadline has been exceeded.
///
/// # Arguments
///
/// * `deadline_ms` - The deadline (Unix timestamp in milliseconds)
/// * `now_ms` - Current time (Unix timestamp in milliseconds)
///
/// # Returns
///
/// `true` if the deadline has passed.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::is_deadline_exceeded;
///
/// assert!(is_deadline_exceeded(1000, 2000));
/// assert!(!is_deadline_exceeded(2000, 1000));
/// assert!(is_deadline_exceeded(1000, 1000)); // Exact match counts as exceeded
/// ```
#[inline]
pub const fn is_deadline_exceeded(deadline_ms: u64, now_ms: u64) -> bool {
    now_ms >= deadline_ms
}

/// Compute remaining time until deadline.
///
/// # Arguments
///
/// * `deadline_ms` - The deadline (Unix timestamp in milliseconds)
/// * `now_ms` - Current time (Unix timestamp in milliseconds)
///
/// # Returns
///
/// Remaining time in milliseconds, or 0 if deadline passed.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::remaining_time_ms;
///
/// assert_eq!(remaining_time_ms(5000, 3000), 2000);
/// assert_eq!(remaining_time_ms(3000, 5000), 0);
/// assert_eq!(remaining_time_ms(3000, 3000), 0);
/// ```
#[inline]
pub const fn remaining_time_ms(deadline_ms: u64, now_ms: u64) -> u64 {
    if now_ms >= deadline_ms {
        0
    } else {
        deadline_ms.saturating_sub(now_ms)
    }
}

/// Compute effective timeout from optional user timeout and default.
///
/// If user provides a timeout, use it (capped at max). Otherwise use default.
///
/// # Arguments
///
/// * `user_timeout_secs` - User-specified timeout (if any)
/// * `default_timeout_secs` - Default timeout
/// * `max_timeout_secs` - Maximum allowed timeout
///
/// # Returns
///
/// The effective timeout in seconds.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_effective_timeout_secs;
///
/// // User timeout within limits
/// assert_eq!(compute_effective_timeout_secs(Some(60), 30, 3600), 60);
///
/// // User timeout exceeds max, capped
/// assert_eq!(compute_effective_timeout_secs(Some(7200), 30, 3600), 3600);
///
/// // No user timeout, use default
/// assert_eq!(compute_effective_timeout_secs(None, 30, 3600), 30);
/// ```
#[inline]
pub const fn compute_effective_timeout_secs(
    user_timeout_secs: Option<u64>,
    default_timeout_secs: u64,
    max_timeout_secs: u64,
) -> u64 {
    match user_timeout_secs {
        Some(timeout) => {
            if timeout > max_timeout_secs {
                max_timeout_secs
            } else {
                timeout
            }
        }
        None => default_timeout_secs,
    }
}

/// Compute pipeline timeout from stage timeouts.
///
/// Pipeline timeout is the sum of all stage timeouts (sequential execution)
/// or the max (parallel execution).
///
/// # Arguments
///
/// * `stage_timeouts_secs` - Timeout for each stage in seconds
/// * `parallel` - Whether stages run in parallel
///
/// # Returns
///
/// Total pipeline timeout in seconds.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_pipeline_timeout_secs;
///
/// // Sequential: sum of timeouts
/// assert_eq!(compute_pipeline_timeout_secs(&[60, 120, 180], false), 360);
///
/// // Parallel: max of timeouts
/// assert_eq!(compute_pipeline_timeout_secs(&[60, 120, 180], true), 180);
///
/// // Empty
/// assert_eq!(compute_pipeline_timeout_secs(&[], false), 0);
/// ```
#[inline]
pub fn compute_pipeline_timeout_secs(stage_timeouts_secs: &[u64], parallel: bool) -> u64 {
    if stage_timeouts_secs.is_empty() {
        return 0;
    }

    if parallel {
        stage_timeouts_secs.iter().copied().max().unwrap_or(0)
    } else {
        stage_timeouts_secs.iter().fold(0u64, |acc, &t| acc.saturating_add(t))
    }
}

/// Convert seconds to milliseconds with overflow protection.
///
/// # Arguments
///
/// * `secs` - Duration in seconds
///
/// # Returns
///
/// Duration in milliseconds (saturates at u64::MAX).
///
/// # Example
///
/// ```
/// use aspen_ci::verified::secs_to_ms;
///
/// assert_eq!(secs_to_ms(30), 30000);
/// assert_eq!(secs_to_ms(0), 0);
/// ```
#[inline]
pub const fn secs_to_ms(secs: u64) -> u64 {
    secs.saturating_mul(1000)
}

/// Convert milliseconds to seconds (truncating).
///
/// # Arguments
///
/// * `ms` - Duration in milliseconds
///
/// # Returns
///
/// Duration in seconds (truncated).
///
/// # Example
///
/// ```
/// use aspen_ci::verified::ms_to_secs;
///
/// assert_eq!(ms_to_secs(30000), 30);
/// assert_eq!(ms_to_secs(30500), 30);
/// assert_eq!(ms_to_secs(500), 0);
/// ```
#[inline]
pub const fn ms_to_secs(ms: u64) -> u64 {
    ms / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // compute_deadline_ms tests
    // ========================================================================

    #[test]
    fn test_deadline_basic() {
        assert_eq!(compute_deadline_ms(1000, 30), 31000);
    }

    #[test]
    fn test_deadline_overflow() {
        assert_eq!(compute_deadline_ms(u64::MAX - 1000, 30), u64::MAX);
    }

    #[test]
    fn test_deadline_zero_timeout() {
        assert_eq!(compute_deadline_ms(1000, 0), 1000);
    }

    // ========================================================================
    // is_deadline_exceeded tests
    // ========================================================================

    #[test]
    fn test_exceeded() {
        assert!(is_deadline_exceeded(1000, 2000));
    }

    #[test]
    fn test_not_exceeded() {
        assert!(!is_deadline_exceeded(2000, 1000));
    }

    #[test]
    fn test_exact_match() {
        assert!(is_deadline_exceeded(1000, 1000));
    }

    // ========================================================================
    // remaining_time_ms tests
    // ========================================================================

    #[test]
    fn test_remaining_positive() {
        assert_eq!(remaining_time_ms(5000, 3000), 2000);
    }

    #[test]
    fn test_remaining_zero_passed() {
        assert_eq!(remaining_time_ms(3000, 5000), 0);
    }

    #[test]
    fn test_remaining_zero_exact() {
        assert_eq!(remaining_time_ms(3000, 3000), 0);
    }

    // ========================================================================
    // compute_effective_timeout_secs tests
    // ========================================================================

    #[test]
    fn test_user_timeout_within_limits() {
        assert_eq!(compute_effective_timeout_secs(Some(60), 30, 3600), 60);
    }

    #[test]
    fn test_user_timeout_exceeds_max() {
        assert_eq!(compute_effective_timeout_secs(Some(7200), 30, 3600), 3600);
    }

    #[test]
    fn test_no_user_timeout() {
        assert_eq!(compute_effective_timeout_secs(None, 30, 3600), 30);
    }

    // ========================================================================
    // compute_pipeline_timeout_secs tests
    // ========================================================================

    #[test]
    fn test_sequential_sum() {
        assert_eq!(compute_pipeline_timeout_secs(&[60, 120, 180], false), 360);
    }

    #[test]
    fn test_parallel_max() {
        assert_eq!(compute_pipeline_timeout_secs(&[60, 120, 180], true), 180);
    }

    #[test]
    fn test_empty() {
        assert_eq!(compute_pipeline_timeout_secs(&[], false), 0);
        assert_eq!(compute_pipeline_timeout_secs(&[], true), 0);
    }

    // ========================================================================
    // secs_to_ms tests
    // ========================================================================

    #[test]
    fn test_secs_to_ms() {
        assert_eq!(secs_to_ms(30), 30000);
        assert_eq!(secs_to_ms(0), 0);
    }

    // ========================================================================
    // ms_to_secs tests
    // ========================================================================

    #[test]
    fn test_ms_to_secs() {
        assert_eq!(ms_to_secs(30000), 30);
        assert_eq!(ms_to_secs(30500), 30);
        assert_eq!(ms_to_secs(500), 0);
    }
}
