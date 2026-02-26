//! Pure retry delay calculation functions.
//!
//! These functions compute retry delays without accessing system time.
//! The async shell layer provides the current time and combines with
//! these calculations.
//!
//! # Tiger Style
//!
//! - Saturating arithmetic prevents overflow
//! - All time values in milliseconds (u64)
//! - No I/O or system calls

/// Compute exponential backoff delay for a retry attempt.
///
/// Calculates `initial_delay_ms * (multiplier ^ (attempt - 1))`, capped
/// by `max_delay_ms` if provided.
///
/// # Arguments
///
/// * `initial_delay_ms` - Base delay in milliseconds
/// * `multiplier` - Multiplier for each attempt (e.g., 2.0 for doubling)
/// * `attempt` - Current attempt number (1-indexed)
/// * `max_delay_ms` - Optional maximum delay cap
///
/// # Returns
///
/// The delay in milliseconds for this retry attempt.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_exponential_retry_delay_ms;
///
/// // First attempt: 100ms
/// assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 1, None), 100);
///
/// // Second attempt: 100 * 2 = 200ms
/// assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 2, None), 200);
///
/// // Third attempt: 100 * 2 * 2 = 400ms
/// assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 3, None), 400);
///
/// // With max cap: min(800, 500) = 500ms
/// assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 4, Some(500)), 500);
/// ```
#[inline]
pub fn compute_exponential_retry_delay_ms(
    initial_delay_ms: u64,
    multiplier: f64,
    attempt: u32,
    max_delay_ms: Option<u64>,
) -> u64 {
    // Tiger Style: preconditions
    debug_assert!(initial_delay_ms > 0, "RETRY_DELAY: initial_delay_ms must be > 0");
    debug_assert!(multiplier > 0.0, "RETRY_DELAY: multiplier must be > 0");
    debug_assert!(!multiplier.is_nan(), "RETRY_DELAY: multiplier must not be NaN");

    if attempt == 0 {
        return initial_delay_ms;
    }

    // Calculate multiplier^(attempt-1) iteratively to avoid floating point issues
    let mut delay_f64 = initial_delay_ms as f64;
    for _ in 1..attempt {
        delay_f64 *= multiplier;

        // Apply max cap during iteration to prevent intermediate overflow
        if let Some(max) = max_delay_ms {
            delay_f64 = delay_f64.min(max as f64);
        }
    }

    // Convert to u64 with saturation
    let delay = if delay_f64 >= u64::MAX as f64 {
        u64::MAX
    } else if delay_f64 <= 0.0 {
        0
    } else {
        delay_f64 as u64
    };

    // Apply final max cap
    let result = match max_delay_ms {
        Some(max) => delay.min(max),
        None => delay,
    };

    // Tiger Style: postcondition
    debug_assert!(
        result >= initial_delay_ms || max_delay_ms.is_some_and(|m| m < initial_delay_ms),
        "RETRY_DELAY: result {} < initial {} without max cap",
        result,
        initial_delay_ms
    );
    result
}

/// Compute fixed retry delay.
///
/// Simply returns the fixed delay, but validates the attempt is within bounds.
///
/// # Arguments
///
/// * `delay_ms` - Fixed delay in milliseconds
/// * `attempt` - Current attempt number (1-indexed)
/// * `max_attempts` - Maximum number of attempts allowed
///
/// # Returns
///
/// `Some(delay_ms)` if attempt is within bounds, `None` if exceeded.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_fixed_retry_delay_ms;
///
/// // Within bounds
/// assert_eq!(compute_fixed_retry_delay_ms(1000, 1, 3), Some(1000));
/// assert_eq!(compute_fixed_retry_delay_ms(1000, 3, 3), Some(1000));
///
/// // Exceeded max attempts
/// assert_eq!(compute_fixed_retry_delay_ms(1000, 4, 3), None);
/// ```
#[inline]
pub const fn compute_fixed_retry_delay_ms(delay_ms: u64, attempt: u32, max_attempts: u32) -> Option<u64> {
    if attempt > max_attempts { None } else { Some(delay_ms) }
}

/// Get custom retry delay from a delay list.
///
/// Returns the delay for the given attempt from the list, with a fallback
/// for attempts beyond the list length.
///
/// # Arguments
///
/// * `delays_ms` - Slice of delays for each attempt (0-indexed into list)
/// * `attempt` - Current attempt number (1-indexed)
/// * `fallback_ms` - Default delay if attempt exceeds list length
///
/// # Returns
///
/// The delay for this attempt in milliseconds.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::get_custom_retry_delay_ms;
///
/// let delays = &[100, 500, 2000, 10000];
///
/// // First attempt uses first delay
/// assert_eq!(get_custom_retry_delay_ms(delays, 1, 60000), 100);
///
/// // Fourth attempt uses fourth delay
/// assert_eq!(get_custom_retry_delay_ms(delays, 4, 60000), 10000);
///
/// // Fifth attempt exceeds list, uses fallback
/// assert_eq!(get_custom_retry_delay_ms(delays, 5, 60000), 60000);
/// ```
#[inline]
pub fn get_custom_retry_delay_ms(delays_ms: &[u64], attempt: u32, fallback_ms: u64) -> u64 {
    if attempt == 0 {
        return delays_ms.first().copied().unwrap_or(fallback_ms);
    }

    let index = (attempt - 1) as usize;
    delays_ms.get(index).copied().unwrap_or(fallback_ms)
}

/// Check if retry attempts have been exceeded.
///
/// # Arguments
///
/// * `attempts` - Number of attempts made so far
/// * `max_attempts` - Maximum number of attempts allowed
///
/// # Returns
///
/// `true` if attempts have exceeded the limit.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::has_exceeded_retry_limit;
///
/// assert!(!has_exceeded_retry_limit(1, 3));
/// assert!(!has_exceeded_retry_limit(3, 3));
/// assert!(has_exceeded_retry_limit(4, 3));
/// ```
#[inline]
pub const fn has_exceeded_retry_limit(attempts: u32, max_attempts: u32) -> bool {
    attempts > max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // compute_exponential_retry_delay_ms tests
    // ========================================================================

    #[test]
    fn test_exponential_first_attempt() {
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 1, None), 100);
    }

    #[test]
    fn test_exponential_doubling() {
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 2, None), 200);
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 3, None), 400);
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 4, None), 800);
    }

    #[test]
    fn test_exponential_with_max_cap() {
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 4, Some(500)), 500);
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 10, Some(1000)), 1000);
    }

    #[test]
    fn test_exponential_zero_attempt() {
        // Attempt 0 should return initial delay
        assert_eq!(compute_exponential_retry_delay_ms(100, 2.0, 0, None), 100);
    }

    #[test]
    fn test_exponential_fractional_multiplier() {
        // With 1.5x multiplier
        assert_eq!(compute_exponential_retry_delay_ms(1000, 1.5, 1, None), 1000);
        assert_eq!(compute_exponential_retry_delay_ms(1000, 1.5, 2, None), 1500);
        assert_eq!(compute_exponential_retry_delay_ms(1000, 1.5, 3, None), 2250);
    }

    #[test]
    fn test_exponential_large_values() {
        // Should saturate at u64::MAX instead of overflowing
        let result = compute_exponential_retry_delay_ms(u64::MAX / 2, 10.0, 100, None);
        assert!(result > 0); // Should not panic or wrap
    }

    // ========================================================================
    // compute_fixed_retry_delay_ms tests
    // ========================================================================

    #[test]
    fn test_fixed_within_bounds() {
        assert_eq!(compute_fixed_retry_delay_ms(1000, 1, 3), Some(1000));
        assert_eq!(compute_fixed_retry_delay_ms(1000, 3, 3), Some(1000));
    }

    #[test]
    fn test_fixed_exceeded() {
        assert_eq!(compute_fixed_retry_delay_ms(1000, 4, 3), None);
    }

    #[test]
    fn test_fixed_zero_attempts() {
        assert_eq!(compute_fixed_retry_delay_ms(1000, 0, 3), Some(1000));
    }

    // ========================================================================
    // get_custom_retry_delay_ms tests
    // ========================================================================

    #[test]
    fn test_custom_within_list() {
        let delays = &[100, 500, 2000, 10000];
        assert_eq!(get_custom_retry_delay_ms(delays, 1, 60000), 100);
        assert_eq!(get_custom_retry_delay_ms(delays, 2, 60000), 500);
        assert_eq!(get_custom_retry_delay_ms(delays, 4, 60000), 10000);
    }

    #[test]
    fn test_custom_exceeds_list() {
        let delays = &[100, 500];
        assert_eq!(get_custom_retry_delay_ms(delays, 3, 60000), 60000);
    }

    #[test]
    fn test_custom_empty_list() {
        let delays: &[u64] = &[];
        assert_eq!(get_custom_retry_delay_ms(delays, 1, 60000), 60000);
    }

    #[test]
    fn test_custom_zero_attempt() {
        let delays = &[100, 500];
        assert_eq!(get_custom_retry_delay_ms(delays, 0, 60000), 100);
    }

    // ========================================================================
    // has_exceeded_retry_limit tests
    // ========================================================================

    #[test]
    fn test_not_exceeded() {
        assert!(!has_exceeded_retry_limit(1, 3));
        assert!(!has_exceeded_retry_limit(3, 3));
    }

    #[test]
    fn test_exceeded() {
        assert!(has_exceeded_retry_limit(4, 3));
        assert!(has_exceeded_retry_limit(100, 3));
    }

    #[test]
    fn test_zero_max() {
        assert!(has_exceeded_retry_limit(1, 0));
    }
}
