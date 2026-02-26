//! Pure scheduling calculation functions.
//!
//! These functions compute next execution times for interval-based schedules
//! without accessing system clocks. Time is passed as an explicit parameter.
//!
//! # Tiger Style
//!
//! - All time values in milliseconds (u64)
//! - Saturating arithmetic prevents overflow
//! - No I/O or system calls

/// Compute the next execution time for an interval schedule.
///
/// Given a start time and interval, calculates when the next execution
/// should occur relative to the current time.
///
/// # Arguments
///
/// * `start_ms` - When the schedule started (Unix timestamp in milliseconds)
/// * `interval_ms` - Interval between executions in milliseconds
/// * `now_ms` - Current time (Unix timestamp in milliseconds)
///
/// # Returns
///
/// The next execution time in milliseconds since Unix epoch.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_next_interval_execution_ms;
///
/// // Started at t=0, interval=1000ms, now=2500ms
/// // Next execution: 3000ms (periods=3, next=3*1000=3000)
/// assert_eq!(compute_next_interval_execution_ms(0, 1000, 2500), 3000);
///
/// // If now is exactly on interval boundary
/// // Next execution: 4000ms (periods=4, next=4*1000=4000)
/// assert_eq!(compute_next_interval_execution_ms(0, 1000, 3000), 4000);
///
/// // If start is in the future, return start time
/// assert_eq!(compute_next_interval_execution_ms(5000, 1000, 2000), 5000);
/// ```
#[inline]
pub fn compute_next_interval_execution_ms(start_ms: u64, interval_ms: u64, now_ms: u64) -> u64 {
    // If start is in the future, return start time
    if start_ms > now_ms {
        return start_ms;
    }

    // Avoid division by zero
    if interval_ms == 0 {
        return now_ms;
    }

    // Calculate elapsed time since start
    let elapsed_ms = now_ms.saturating_sub(start_ms);

    // Calculate number of complete periods, then add 1 for next
    let periods = (elapsed_ms / interval_ms).saturating_add(1);

    // Calculate next execution time
    let offset = periods.saturating_mul(interval_ms);
    start_ms.saturating_add(offset)
}

/// Determine if a one-time scheduled execution should fire.
///
/// # Arguments
///
/// * `scheduled_ms` - When the execution is scheduled (Unix timestamp in milliseconds)
/// * `now_ms` - Current time (Unix timestamp in milliseconds)
///
/// # Returns
///
/// `true` if the scheduled time is in the future and should be waited for.
/// `false` if the scheduled time has passed.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::is_one_time_schedule_pending;
///
/// // Future schedule
/// assert!(is_one_time_schedule_pending(5000, 3000));
///
/// // Past schedule
/// assert!(!is_one_time_schedule_pending(3000, 5000));
///
/// // Exact match (past, should fire now)
/// assert!(!is_one_time_schedule_pending(3000, 3000));
/// ```
#[inline]
pub const fn is_one_time_schedule_pending(scheduled_ms: u64, now_ms: u64) -> bool {
    scheduled_ms > now_ms
}

/// Check if a rate limit period has reset.
///
/// Rate limits reset at the start of each hour.
///
/// # Arguments
///
/// * `current_hour_start_ms` - Start of current tracking hour (Unix timestamp)
/// * `now_hour_start_ms` - Start of the current hour based on now (Unix timestamp)
///
/// # Returns
///
/// `true` if we're in a new hour and the rate limit count should reset.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::should_reset_rate_limit;
///
/// // Same hour
/// assert!(!should_reset_rate_limit(3600000, 3600000));
///
/// // New hour
/// assert!(should_reset_rate_limit(3600000, 7200000));
///
/// // First check (no previous hour)
/// assert!(should_reset_rate_limit(0, 3600000));
/// ```
#[inline]
pub const fn should_reset_rate_limit(current_hour_start_ms: u64, now_hour_start_ms: u64) -> bool {
    current_hour_start_ms < now_hour_start_ms
}

/// Compute when the next rate limit window starts.
///
/// # Arguments
///
/// * `current_hour_start_ms` - Start of current hour (Unix timestamp in milliseconds)
///
/// # Returns
///
/// Start of the next hour in milliseconds.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_next_hour_start_ms;
///
/// const HOUR_MS: u64 = 60 * 60 * 1000;
///
/// assert_eq!(compute_next_hour_start_ms(0), HOUR_MS);
/// assert_eq!(compute_next_hour_start_ms(HOUR_MS), 2 * HOUR_MS);
/// ```
#[inline]
pub const fn compute_next_hour_start_ms(current_hour_start_ms: u64) -> u64 {
    const HOUR_MS: u64 = 60 * 60 * 1000;
    current_hour_start_ms.saturating_add(HOUR_MS)
}

/// Check if rate limit allows execution.
///
/// # Arguments
///
/// * `current_count` - Number of executions in current hour
/// * `max_per_hour` - Maximum allowed executions per hour
///
/// # Returns
///
/// `true` if there's remaining capacity in the current hour.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::rate_limit_has_capacity;
///
/// assert!(rate_limit_has_capacity(5, 10));
/// assert!(!rate_limit_has_capacity(10, 10));
/// assert!(!rate_limit_has_capacity(11, 10));
/// ```
#[inline]
pub const fn rate_limit_has_capacity(current_count: u32, max_per_hour: u32) -> bool {
    current_count < max_per_hour
}

/// Truncate a timestamp to the start of its hour.
///
/// # Arguments
///
/// * `timestamp_ms` - Unix timestamp in milliseconds
///
/// # Returns
///
/// Unix timestamp of the start of that hour.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::truncate_to_hour_ms;
///
/// const HOUR_MS: u64 = 60 * 60 * 1000;
///
/// // Middle of first hour -> start of first hour
/// assert_eq!(truncate_to_hour_ms(HOUR_MS / 2), 0);
///
/// // Middle of second hour -> start of second hour
/// assert_eq!(truncate_to_hour_ms(HOUR_MS + HOUR_MS / 2), HOUR_MS);
/// ```
#[inline]
pub const fn truncate_to_hour_ms(timestamp_ms: u64) -> u64 {
    const HOUR_MS: u64 = 60 * 60 * 1000;
    (timestamp_ms / HOUR_MS) * HOUR_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR_MS: u64 = 60 * 60 * 1000;

    // ========================================================================
    // compute_next_interval_execution_ms tests
    // ========================================================================

    #[test]
    fn test_interval_basic() {
        // Started at 0, 1 second interval, now at 2.5 seconds
        assert_eq!(compute_next_interval_execution_ms(0, 1000, 2500), 3000);
    }

    #[test]
    fn test_interval_on_boundary() {
        // On exact boundary, should go to next interval
        assert_eq!(compute_next_interval_execution_ms(0, 1000, 3000), 4000);
    }

    #[test]
    fn test_interval_start_in_future() {
        // Start is in the future
        assert_eq!(compute_next_interval_execution_ms(5000, 1000, 2000), 5000);
    }

    #[test]
    fn test_interval_zero_interval() {
        // Zero interval should return now
        assert_eq!(compute_next_interval_execution_ms(0, 0, 5000), 5000);
    }

    #[test]
    fn test_interval_large_values() {
        // Should not overflow
        let result = compute_next_interval_execution_ms(u64::MAX - 1000, 100, u64::MAX);
        assert!(result >= u64::MAX - 1000);
    }

    #[test]
    fn test_interval_with_offset_start() {
        // Started at 500, interval 1000
        // now=1800, elapsed=1300, periods=1+1=2, next=500+2000=2500
        assert_eq!(compute_next_interval_execution_ms(500, 1000, 1800), 2500);
    }

    // ========================================================================
    // is_one_time_schedule_pending tests
    // ========================================================================

    #[test]
    fn test_one_time_future() {
        assert!(is_one_time_schedule_pending(5000, 3000));
    }

    #[test]
    fn test_one_time_past() {
        assert!(!is_one_time_schedule_pending(3000, 5000));
    }

    #[test]
    fn test_one_time_exact() {
        assert!(!is_one_time_schedule_pending(3000, 3000));
    }

    // ========================================================================
    // should_reset_rate_limit tests
    // ========================================================================

    #[test]
    fn test_rate_limit_same_hour() {
        assert!(!should_reset_rate_limit(HOUR_MS, HOUR_MS));
    }

    #[test]
    fn test_rate_limit_new_hour() {
        assert!(should_reset_rate_limit(HOUR_MS, 2 * HOUR_MS));
    }

    #[test]
    fn test_rate_limit_first_check() {
        assert!(should_reset_rate_limit(0, HOUR_MS));
    }

    // ========================================================================
    // compute_next_hour_start_ms tests
    // ========================================================================

    #[test]
    fn test_next_hour() {
        assert_eq!(compute_next_hour_start_ms(0), HOUR_MS);
        assert_eq!(compute_next_hour_start_ms(HOUR_MS), 2 * HOUR_MS);
    }

    #[test]
    fn test_next_hour_saturation() {
        // Should saturate at u64::MAX
        assert_eq!(compute_next_hour_start_ms(u64::MAX), u64::MAX);
    }

    // ========================================================================
    // rate_limit_has_capacity tests
    // ========================================================================

    #[test]
    fn test_capacity_available() {
        assert!(rate_limit_has_capacity(5, 10));
        assert!(rate_limit_has_capacity(0, 10));
    }

    #[test]
    fn test_capacity_exhausted() {
        assert!(!rate_limit_has_capacity(10, 10));
        assert!(!rate_limit_has_capacity(11, 10));
    }

    // ========================================================================
    // truncate_to_hour_ms tests
    // ========================================================================

    #[test]
    fn test_truncate_middle() {
        assert_eq!(truncate_to_hour_ms(HOUR_MS / 2), 0);
        assert_eq!(truncate_to_hour_ms(HOUR_MS + HOUR_MS / 2), HOUR_MS);
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate_to_hour_ms(HOUR_MS), HOUR_MS);
    }

    #[test]
    fn test_truncate_zero() {
        assert_eq!(truncate_to_hour_ms(0), 0);
    }
}
