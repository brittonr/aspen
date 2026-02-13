//! Time utilities for Aspen distributed systems.
//!
//! This module provides safe, panic-free time access functions following Tiger Style
//! resource management principles.
//!
//! # Tiger Style
//!
//! - No `.expect()` or `.unwrap()` - safe fallback to 0
//! - Inline for hot path performance
//! - Pure functions with no external dependencies

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Get current Unix timestamp in milliseconds.
///
/// Returns 0 if system time is before UNIX epoch (should never happen
/// on properly configured systems, but prevents panics).
///
/// # Tiger Style
///
/// - No `.expect()` or `.unwrap()` - safe fallback to 0
/// - Inline for hot path performance
#[inline]
pub fn current_time_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Get current Unix timestamp in seconds.
///
/// Returns 0 if system time is before UNIX epoch (should never happen
/// on properly configured systems, but prevents panics).
///
/// # Tiger Style
///
/// - No `.expect()` or `.unwrap()` - safe fallback to 0
/// - Inline for hot path performance
#[inline]
pub fn current_time_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_ms_returns_nonzero() {
        let time = current_time_ms();
        assert!(time > 0, "current_time_ms should return non-zero on valid systems");
    }

    #[test]
    fn current_time_ms_is_monotonic() {
        let t1 = current_time_ms();
        let t2 = current_time_ms();
        assert!(t2 >= t1, "time should not go backwards");
    }

    #[test]
    fn current_time_ms_reasonable_range() {
        // Should be after year 2020 (1577836800000 ms) and before year 2100
        let time = current_time_ms();
        let year_2020_ms = 1_577_836_800_000u64;
        let year_2100_ms = 4_102_444_800_000u64;
        assert!(time > year_2020_ms, "current_time_ms {} should be after year 2020", time);
        assert!(time < year_2100_ms, "current_time_ms {} should be before year 2100", time);
    }

    #[test]
    fn current_time_secs_returns_nonzero() {
        let time = current_time_secs();
        assert!(time > 0, "current_time_secs should return non-zero on valid systems");
    }

    #[test]
    fn current_time_secs_is_monotonic() {
        let t1 = current_time_secs();
        let t2 = current_time_secs();
        assert!(t2 >= t1, "time should not go backwards");
    }

    #[test]
    fn current_time_secs_reasonable_range() {
        // Should be after year 2020 (1577836800) and before year 2100
        let time = current_time_secs();
        let year_2020 = 1_577_836_800u64;
        let year_2100 = 4_102_444_800u64;
        assert!(time > year_2020, "current_time_secs {} should be after year 2020", time);
        assert!(time < year_2100, "current_time_secs {} should be before year 2100", time);
    }

    #[test]
    fn current_time_ms_and_secs_consistent() {
        let ms = current_time_ms();
        let secs = current_time_secs();
        // ms / 1000 should be close to secs (within 1 second)
        let ms_as_secs = ms / 1000;
        assert!(
            ms_as_secs >= secs.saturating_sub(1) && ms_as_secs <= secs + 1,
            "ms/1000 ({}) and secs ({}) should be consistent",
            ms_as_secs,
            secs
        );
    }
}
