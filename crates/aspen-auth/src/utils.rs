//! Utility functions for capability-based authorization.
//!
//! Provides safe time access without panics.

use std::time::{SystemTime, UNIX_EPOCH};

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
