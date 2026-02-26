//! Pure rate limiter computation functions.
//!
//! This module contains pure functions for token bucket rate limiting.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

// ============================================================================
// Token Replenishment
// ============================================================================

/// Calculate available tokens after replenishment.
///
/// Implements the token bucket algorithm where tokens are added
/// at a constant rate up to a maximum capacity.
///
/// # Arguments
///
/// * `current_tokens` - Current token count (can be fractional)
/// * `elapsed_ms` - Milliseconds elapsed since last update
/// * `rate_per_sec` - Tokens added per second
/// * `capacity` - Maximum token capacity
///
/// # Returns
///
/// Available tokens after replenishment (capped at capacity).
///
/// # Example
///
/// ```ignore
/// // 1 second elapsed, 10 tokens/sec refill
/// let available = calculate_replenished_tokens(0.0, 1000, 10.0, 100);
/// assert!((available - 10.0).abs() < 0.001);
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic concepts (min/max capping)
/// - Result is bounded by capacity
/// - Time passed explicitly as elapsed_ms
#[inline]
pub fn calculate_replenished_tokens(current_tokens: f64, elapsed_ms: u64, rate_per_sec: f64, capacity: u64) -> f64 {
    let elapsed_secs = elapsed_ms as f64 / 1000.0;
    let replenished = elapsed_secs * rate_per_sec;
    (current_tokens + replenished).min(capacity as f64)
}

// ============================================================================
// Token Consumption
// ============================================================================

/// Result of attempting to consume tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenConsumptionResult {
    /// Token was consumed successfully.
    /// Contains remaining tokens after consumption.
    Consumed { remaining: f64 },
    /// Not enough tokens available.
    /// Contains current available tokens.
    Denied { available: f64 },
}

impl TokenConsumptionResult {
    /// Check if the token was consumed.
    #[inline]
    pub fn is_consumed(&self) -> bool {
        matches!(self, TokenConsumptionResult::Consumed { .. })
    }
}

/// Check if a token can be consumed from the bucket.
///
/// # Arguments
///
/// * `available_tokens` - Currently available tokens
///
/// # Returns
///
/// `TokenConsumptionResult::Consumed` with remaining tokens if allowed,
/// `TokenConsumptionResult::Denied` with available tokens if denied.
///
/// # Tiger Style
///
/// - Pure function with no side effects
/// - Explicit result type for clarity
#[inline]
pub fn can_consume_token(available_tokens: f64) -> TokenConsumptionResult {
    if available_tokens >= 1.0 {
        TokenConsumptionResult::Consumed {
            remaining: available_tokens - 1.0,
        }
    } else {
        TokenConsumptionResult::Denied {
            available: available_tokens,
        }
    }
}

// ============================================================================
// LRU Eviction
// ============================================================================

/// Determine if the oldest entry should be evicted from the map.
///
/// Used for LRU eviction in bounded rate limiter maps.
///
/// # Arguments
///
/// * `current_len` - Current number of entries in the map
/// * `max_capacity` - Maximum allowed entries
///
/// # Returns
///
/// `true` if an entry should be evicted to make room, `false` otherwise.
///
/// # Tiger Style
///
/// - Pure function for eviction decision
/// - Explicit u32 types for bounds
#[inline]
pub fn should_evict_oldest(current_len: u32, max_capacity: u32) -> bool {
    current_len >= max_capacity
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Replenishment Tests
    // ========================================================================

    #[test]
    fn test_replenish_one_second() {
        let available = calculate_replenished_tokens(0.0, 1000, 10.0, 100);
        assert!((available - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_partial_second() {
        let available = calculate_replenished_tokens(0.0, 500, 10.0, 100);
        assert!((available - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_caps_at_capacity() {
        let available = calculate_replenished_tokens(90.0, 1000, 100.0, 100);
        assert!((available - 100.0).abs() < 0.001); // Capped at 100
    }

    #[test]
    fn test_replenish_no_time_elapsed() {
        let available = calculate_replenished_tokens(50.0, 0, 10.0, 100);
        assert!((available - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_zero_rate() {
        let available = calculate_replenished_tokens(50.0, 1000, 0.0, 100);
        assert!((available - 50.0).abs() < 0.001); // No replenishment
    }

    #[test]
    fn test_replenish_large_elapsed_time() {
        // Even with huge elapsed time, should cap at capacity
        let available = calculate_replenished_tokens(0.0, u64::MAX / 2, 1000.0, 100);
        assert!((available - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_fractional_tokens() {
        let available = calculate_replenished_tokens(5.5, 100, 10.0, 100);
        // 5.5 + (0.1 * 10) = 6.5
        assert!((available - 6.5).abs() < 0.001);
    }

    // ========================================================================
    // Token Consumption Tests
    // ========================================================================

    #[test]
    fn test_consume_sufficient() {
        let result = can_consume_token(5.0);
        match result {
            TokenConsumptionResult::Consumed { remaining } => {
                assert!((remaining - 4.0).abs() < 0.001);
            }
            _ => panic!("Expected Consumed"),
        }
    }

    #[test]
    fn test_consume_exact() {
        let result = can_consume_token(1.0);
        match result {
            TokenConsumptionResult::Consumed { remaining } => {
                assert!((remaining - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Consumed"),
        }
    }

    #[test]
    fn test_consume_insufficient() {
        let result = can_consume_token(0.5);
        match result {
            TokenConsumptionResult::Denied { available } => {
                assert!((available - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_consume_zero() {
        let result = can_consume_token(0.0);
        assert!(!result.is_consumed());
    }

    #[test]
    fn test_consume_negative() {
        // Edge case: negative tokens (shouldn't happen, but handle gracefully)
        let result = can_consume_token(-1.0);
        assert!(!result.is_consumed());
    }

    #[test]
    fn test_is_consumed_helper() {
        assert!(can_consume_token(5.0).is_consumed());
        assert!(!can_consume_token(0.5).is_consumed());
    }

    // ========================================================================
    // LRU Eviction Tests
    // ========================================================================

    #[test]
    fn test_evict_at_capacity() {
        assert!(should_evict_oldest(100, 100));
    }

    #[test]
    fn test_evict_over_capacity() {
        assert!(should_evict_oldest(101, 100));
    }

    #[test]
    fn test_no_evict_under_capacity() {
        assert!(!should_evict_oldest(99, 100));
    }

    #[test]
    fn test_no_evict_empty() {
        assert!(!should_evict_oldest(0, 100));
    }

    #[test]
    fn test_evict_zero_capacity() {
        // Edge case: zero capacity means always evict
        assert!(should_evict_oldest(0, 0));
        assert!(should_evict_oldest(1, 0));
    }

    #[test]
    fn test_evict_max_values() {
        assert!(should_evict_oldest(u32::MAX, u32::MAX));
        assert!(!should_evict_oldest(u32::MAX - 1, u32::MAX));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_replenish_never_exceeds_capacity() {
        check!().with_type::<(u64, u64)>().for_each(|(elapsed_ms, capacity)| {
            if *capacity > 0 {
                let available = calculate_replenished_tokens(0.0, *elapsed_ms, 1000.0, *capacity);
                assert!(available <= *capacity as f64, "Replenished tokens must not exceed capacity");
            }
        });
    }

    #[test]
    fn prop_replenish_monotonic_with_time() {
        check!().with_type::<(u64, u64)>().for_each(|(elapsed1, delta)| {
            let elapsed2 = elapsed1.saturating_add(*delta);
            let available1 = calculate_replenished_tokens(0.0, *elapsed1, 10.0, 100);
            let available2 = calculate_replenished_tokens(0.0, elapsed2, 10.0, 100);
            assert!(available2 >= available1, "More time should mean more or equal tokens");
        });
    }

    #[test]
    fn prop_consume_consistency() {
        check!().with_type::<u32>().for_each(|available_int| {
            let available = *available_int as f64 / 100.0; // Convert to fractional
            let result = can_consume_token(available);
            if available >= 1.0 {
                assert!(result.is_consumed());
            } else {
                assert!(!result.is_consumed());
            }
        });
    }

    #[test]
    fn prop_evict_decision_consistent() {
        check!().with_type::<(u32, u32)>().for_each(|(current, max)| {
            let should = should_evict_oldest(*current, *max);
            if *current >= *max {
                assert!(should, "Should evict when at or over capacity");
            } else {
                assert!(!should, "Should not evict when under capacity");
            }
        });
    }
}
