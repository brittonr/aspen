//! Pure heuristic functions for storage operations.
//!
//! This module contains pure functions for TTL calculation.
//!
//! All functions are deterministic and side-effect free, making them ideal for:
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Fuzzing for edge case discovery
//!
//! # Tiger Style
//!
//! - All calculations bounded by explicit limits
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes

// ============================================================================
// TTL Calculation Pure Functions
// ============================================================================

/// Calculate absolute expiration timestamp from relative TTL.
///
/// Converts a TTL in seconds to an absolute expiration time in milliseconds
/// since Unix epoch. Uses saturating arithmetic to prevent overflow.
///
/// # Arguments
///
/// * `now_ms` - Current timestamp in milliseconds since Unix epoch
/// * `ttl_seconds` - Time-to-live in seconds
///
/// # Returns
///
/// Absolute expiration timestamp in milliseconds. On overflow, returns `u64::MAX`.
///
/// # Example
///
/// ```
/// use aspen_redb_storage::verified::calculate_expires_at_ms;
///
/// let now_ms = 1704067200000; // Jan 1, 2024 00:00:00 UTC
/// let ttl_seconds = 3600;     // 1 hour
/// let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
/// assert_eq!(expires_at, now_ms + 3600 * 1000);
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic to prevent overflow
/// - Returns deterministic result for any valid input
/// - No panics for any input combination
#[inline]
pub fn calculate_expires_at_ms(now_ms: u64, ttl_seconds: u32) -> u64 {
    let ttl_ms = (ttl_seconds as u64).saturating_mul(1000);
    now_ms.saturating_add(ttl_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_expires_at_ms_basic() {
        let now_ms = 1704067200000; // Jan 1, 2024 00:00:00 UTC
        let ttl_seconds = 3600; // 1 hour
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, now_ms + 3600 * 1000);
    }

    #[test]
    fn test_calculate_expires_at_ms_zero_ttl() {
        let now_ms = 1000;
        let expires_at = calculate_expires_at_ms(now_ms, 0);
        assert_eq!(expires_at, 1000);
    }

    #[test]
    fn test_calculate_expires_at_ms_overflow_saturates() {
        let now_ms = u64::MAX - 1000;
        let ttl_seconds = u32::MAX;
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, u64::MAX);
    }

    #[test]
    fn test_calculate_expires_at_ms_one_second() {
        let now_ms = 1000;
        let expires_at = calculate_expires_at_ms(now_ms, 1);
        assert_eq!(expires_at, 2000);
    }

    #[test]
    fn test_calculate_expires_at_ms_max_ttl_no_overflow() {
        let now_ms = 0;
        let expires_at = calculate_expires_at_ms(now_ms, u32::MAX);
        assert_eq!(expires_at, 4294967295000);
    }
}
