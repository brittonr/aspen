//! Pure token bucket rate limiter calculations.
//!
//! These functions compute token bucket state updates without accessing
//! system time. The shell layer provides elapsed time and manages
//! the actual state.
//!
//! # Tiger Style
//!
//! - All time values in seconds (f64 for fractional precision)
//! - Saturating arithmetic to prevent overflow
//! - No I/O or system calls

/// Compute the number of tokens after replenishment.
///
/// Calculates how many tokens should be in the bucket after a given
/// time has elapsed, capping at the bucket capacity.
///
/// # Arguments
///
/// * `current_tokens` - Current number of tokens in the bucket
/// * `elapsed_secs` - Time elapsed since last update (in seconds)
/// * `rate_per_sec` - Token replenishment rate (tokens per second)
/// * `capacity` - Maximum bucket capacity (burst size)
///
/// # Returns
///
/// The updated number of tokens (capped at capacity).
///
/// # Example
///
/// ```
/// use aspen_forge::verified::compute_replenished_tokens;
///
/// // 5 tokens, 2 seconds elapsed, 1 token/sec rate, capacity 10
/// assert_eq!(compute_replenished_tokens(5.0, 2.0, 1.0, 10.0), 7.0);
///
/// // Capped at capacity
/// assert_eq!(compute_replenished_tokens(5.0, 10.0, 1.0, 10.0), 10.0);
///
/// // No time elapsed
/// assert_eq!(compute_replenished_tokens(5.0, 0.0, 1.0, 10.0), 5.0);
/// ```
#[inline]
pub fn compute_replenished_tokens(current_tokens: f64, elapsed_secs: f64, rate_per_sec: f64, capacity: f64) -> f64 {
    let replenished = elapsed_secs * rate_per_sec;
    (current_tokens + replenished).min(capacity)
}

/// Check if a token can be consumed from the bucket.
///
/// # Arguments
///
/// * `tokens` - Current number of tokens
/// * `required` - Number of tokens required (typically 1.0)
///
/// # Returns
///
/// `true` if there are enough tokens to consume.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::can_consume_token;
///
/// assert!(can_consume_token(1.0, 1.0));
/// assert!(can_consume_token(5.5, 1.0));
/// assert!(!can_consume_token(0.5, 1.0));
/// assert!(!can_consume_token(0.0, 1.0));
/// ```
#[inline]
pub fn can_consume_token(tokens: f64, required: f64) -> bool {
    tokens >= required
}

/// Compute the remaining tokens after consumption.
///
/// # Arguments
///
/// * `tokens` - Current number of tokens
/// * `consumed` - Number of tokens to consume
///
/// # Returns
///
/// The remaining tokens (never negative).
///
/// # Example
///
/// ```
/// use aspen_forge::verified::compute_remaining_tokens;
///
/// assert_eq!(compute_remaining_tokens(5.0, 1.0), 4.0);
/// assert_eq!(compute_remaining_tokens(1.0, 1.0), 0.0);
/// // Never goes negative
/// assert_eq!(compute_remaining_tokens(0.5, 1.0), 0.0);
/// ```
#[inline]
pub fn compute_remaining_tokens(tokens: f64, consumed: f64) -> f64 {
    (tokens - consumed).max(0.0)
}

/// Convert rate from per-minute to per-second.
///
/// # Arguments
///
/// * `rate_per_minute` - Rate in tokens per minute
///
/// # Returns
///
/// Rate in tokens per second.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::rate_per_minute_to_per_sec;
///
/// assert_eq!(rate_per_minute_to_per_sec(60), 1.0);
/// assert_eq!(rate_per_minute_to_per_sec(120), 2.0);
/// assert_eq!(rate_per_minute_to_per_sec(30), 0.5);
/// ```
#[inline]
pub fn rate_per_minute_to_per_sec(rate_per_minute: u32) -> f64 {
    rate_per_minute as f64 / 60.0
}

/// Determine which rate limit was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request allowed.
    Allowed,
    /// Per-peer rate limit exceeded.
    PerPeerExceeded,
    /// Global rate limit exceeded.
    GlobalExceeded,
}

/// Check both per-peer and global rate limits.
///
/// # Arguments
///
/// * `per_peer_tokens` - Current tokens in per-peer bucket
/// * `global_tokens` - Current tokens in global bucket
///
/// # Returns
///
/// Result indicating if allowed or which limit was exceeded.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{RateLimitResult, check_rate_limits};
///
/// // Both have tokens
/// assert_eq!(check_rate_limits(5.0, 10.0), RateLimitResult::Allowed);
///
/// // Per-peer exhausted (checked first)
/// assert_eq!(check_rate_limits(0.5, 10.0), RateLimitResult::PerPeerExceeded);
///
/// // Global exhausted
/// assert_eq!(check_rate_limits(5.0, 0.5), RateLimitResult::GlobalExceeded);
///
/// // Both exhausted (per-peer checked first)
/// assert_eq!(check_rate_limits(0.5, 0.5), RateLimitResult::PerPeerExceeded);
/// ```
#[inline]
pub fn check_rate_limits(per_peer_tokens: f64, global_tokens: f64) -> RateLimitResult {
    if per_peer_tokens < 1.0 {
        RateLimitResult::PerPeerExceeded
    } else if global_tokens < 1.0 {
        RateLimitResult::GlobalExceeded
    } else {
        RateLimitResult::Allowed
    }
}

/// Compute whether a peer should be evicted from the tracking map.
///
/// Uses LRU eviction: the peer with the oldest last_access time
/// should be evicted when the map is at capacity.
///
/// # Arguments
///
/// * `peer_last_access_ms` - Last access time of the peer being considered
/// * `oldest_access_ms` - Oldest access time among all tracked peers
/// * `at_capacity` - Whether the tracking map is at capacity
///
/// # Returns
///
/// `true` if this peer should be evicted.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::should_evict_peer;
///
/// // At capacity and this is the oldest peer
/// assert!(should_evict_peer(1000, 1000, true));
///
/// // At capacity but not the oldest
/// assert!(!should_evict_peer(2000, 1000, true));
///
/// // Not at capacity, don't evict
/// assert!(!should_evict_peer(1000, 1000, false));
/// ```
#[inline]
pub const fn should_evict_peer(peer_last_access_ms: u64, oldest_access_ms: u64, at_capacity: bool) -> bool {
    at_capacity && peer_last_access_ms <= oldest_access_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // compute_replenished_tokens tests
    // ========================================================================

    #[test]
    fn test_replenish_basic() {
        assert!((compute_replenished_tokens(5.0, 2.0, 1.0, 10.0) - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_capped() {
        assert!((compute_replenished_tokens(5.0, 10.0, 1.0, 10.0) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_no_time() {
        assert!((compute_replenished_tokens(5.0, 0.0, 1.0, 10.0) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_empty_bucket() {
        assert!((compute_replenished_tokens(0.0, 5.0, 2.0, 10.0) - 10.0).abs() < 0.001);
    }

    // ========================================================================
    // can_consume_token tests
    // ========================================================================

    #[test]
    fn test_can_consume_exact() {
        assert!(can_consume_token(1.0, 1.0));
    }

    #[test]
    fn test_can_consume_plenty() {
        assert!(can_consume_token(5.5, 1.0));
    }

    #[test]
    fn test_cannot_consume_insufficient() {
        assert!(!can_consume_token(0.5, 1.0));
    }

    #[test]
    fn test_cannot_consume_empty() {
        assert!(!can_consume_token(0.0, 1.0));
    }

    // ========================================================================
    // compute_remaining_tokens tests
    // ========================================================================

    #[test]
    fn test_remaining_normal() {
        assert!((compute_remaining_tokens(5.0, 1.0) - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_remaining_exact() {
        assert!((compute_remaining_tokens(1.0, 1.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_remaining_never_negative() {
        assert!((compute_remaining_tokens(0.5, 1.0) - 0.0).abs() < 0.001);
    }

    // ========================================================================
    // rate_per_minute_to_per_sec tests
    // ========================================================================

    #[test]
    fn test_rate_conversion() {
        assert!((rate_per_minute_to_per_sec(60) - 1.0).abs() < 0.001);
        assert!((rate_per_minute_to_per_sec(120) - 2.0).abs() < 0.001);
        assert!((rate_per_minute_to_per_sec(30) - 0.5).abs() < 0.001);
    }

    // ========================================================================
    // check_rate_limits tests
    // ========================================================================

    #[test]
    fn test_rate_limits_allowed() {
        assert_eq!(check_rate_limits(5.0, 10.0), RateLimitResult::Allowed);
    }

    #[test]
    fn test_rate_limits_per_peer_exceeded() {
        assert_eq!(check_rate_limits(0.5, 10.0), RateLimitResult::PerPeerExceeded);
    }

    #[test]
    fn test_rate_limits_global_exceeded() {
        assert_eq!(check_rate_limits(5.0, 0.5), RateLimitResult::GlobalExceeded);
    }

    #[test]
    fn test_rate_limits_both_exceeded() {
        // Per-peer is checked first
        assert_eq!(check_rate_limits(0.5, 0.5), RateLimitResult::PerPeerExceeded);
    }

    // ========================================================================
    // should_evict_peer tests
    // ========================================================================

    #[test]
    fn test_evict_at_capacity_oldest() {
        assert!(should_evict_peer(1000, 1000, true));
    }

    #[test]
    fn test_no_evict_not_oldest() {
        assert!(!should_evict_peer(2000, 1000, true));
    }

    #[test]
    fn test_no_evict_not_at_capacity() {
        assert!(!should_evict_peer(1000, 1000, false));
    }
}
