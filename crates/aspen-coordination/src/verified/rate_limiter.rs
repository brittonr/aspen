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
// Token Calculation
// ============================================================================

/// Calculate available tokens after replenishment.
///
/// Implements the token bucket algorithm where tokens are added
/// at a constant rate up to a maximum capacity.
///
/// # Arguments
///
/// * `current_tokens` - Current token count (can be fractional)
/// * `last_update_ms` - Time of last update in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
/// * `refill_rate` - Tokens added per second
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
/// let available = calculate_replenished_tokens(0.0, 1000, 2000, 10.0, 100);
/// assert!((available - 10.0).abs() < 0.001);
/// ```
///
/// # Tiger Style
///
/// - Uses saturating_sub for elapsed time calculation
/// - Result is bounded by capacity
#[inline]
pub fn calculate_replenished_tokens(
    current_tokens: f64,
    last_update_ms: u64,
    now_ms: u64,
    refill_rate: f64,
    capacity: u64,
) -> f64 {
    debug_assert!(refill_rate >= 0.0, "refill_rate must be non-negative");
    let elapsed_ms = now_ms.saturating_sub(last_update_ms);
    let elapsed_secs = elapsed_ms as f64 / 1000.0;
    let replenished = elapsed_secs * refill_rate;
    let result = (current_tokens + replenished).min(capacity as f64);
    debug_assert!(result <= capacity as f64, "result must not exceed capacity");
    result
}

// ============================================================================
// Availability Check
// ============================================================================

/// Result of token availability check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenAvailability {
    /// Tokens are available. Contains the remaining tokens after consumption.
    Available {
        /// Remaining tokens after consuming the requested amount.
        remaining: f64,
    },
    /// Not enough tokens. Contains information for retry.
    Exhausted {
        /// Number of tokens requested.
        requested: u64,
        /// Number of tokens currently available.
        available: u64,
        /// Suggested wait time in milliseconds before retrying.
        retry_after_ms: u64,
    },
}

impl TokenAvailability {
    /// Check if tokens are available.
    #[inline]
    pub fn is_available(&self) -> bool {
        matches!(self, TokenAvailability::Available { .. })
    }
}

/// Check if requested tokens are available.
///
/// # Arguments
///
/// * `available` - Currently available tokens
/// * `requested` - Number of tokens requested
/// * `refill_rate` - Tokens added per second (for retry calculation)
///
/// # Returns
///
/// `TokenAvailability::Available` with remaining tokens if allowed,
/// `TokenAvailability::Exhausted` with retry info if rate limited.
///
/// # Example
///
/// ```ignore
/// // Sufficient tokens
/// let result = check_token_availability(10.0, 5, 10.0);
/// assert!(matches!(result, TokenAvailability::Available { remaining: 5.0 }));
///
/// // Insufficient tokens
/// let result = check_token_availability(3.0, 5, 10.0);
/// assert!(matches!(result, TokenAvailability::Exhausted { .. }));
/// ```
///
/// # Tiger Style
///
/// - Handles edge cases (refill_rate = 0)
/// - Uses ceiling for retry time (conservative)
#[inline]
pub fn check_token_availability(available: f64, requested: u64, refill_rate: f64) -> TokenAvailability {
    let requested_f = requested as f64;

    if requested_f <= available {
        TokenAvailability::Available {
            remaining: available - requested_f,
        }
    } else {
        let deficit = requested_f - available;
        let wait_secs = if refill_rate > 0.0 {
            deficit / refill_rate
        } else {
            // If refill_rate is 0, we'll never get more tokens
            f64::MAX
        };
        let retry_after_ms = (wait_secs * 1000.0).ceil() as u64;

        TokenAvailability::Exhausted {
            requested,
            available: available as u64,
            retry_after_ms,
        }
    }
}

// ============================================================================
// Token Operations
// ============================================================================

/// Check if tokens are available.
#[inline]
pub fn has_tokens_available(current_tokens: u64, requested: u64) -> bool {
    current_tokens >= requested
}

/// Consume tokens (subtract amount from current).
#[inline]
pub fn consume_tokens(current_tokens: u64, consumed: u64) -> u64 {
    let remaining = current_tokens.saturating_sub(consumed);
    debug_assert!(remaining <= current_tokens, "remaining must not exceed current_tokens");
    remaining
}

/// Add tokens during refill (capped at capacity).
#[inline]
pub fn refill_tokens(current_tokens: u64, refill_amount: u64, capacity: u64) -> u64 {
    let sum = current_tokens.saturating_add(refill_amount);
    let result = if sum > capacity { capacity } else { sum };
    debug_assert!(result <= capacity, "result must not exceed capacity");
    debug_assert!(result >= current_tokens || result == capacity, "result must be >= current or at capacity");
    result
}

// ============================================================================
// Interval Calculations
// ============================================================================

/// Calculate number of refill intervals elapsed.
///
/// # Arguments
///
/// * `last_refill_ms` - Last refill timestamp (Unix ms)
/// * `now_ms` - Current time (Unix ms)
/// * `interval_ms` - Refill interval in milliseconds
///
/// # Returns
///
/// Number of complete intervals elapsed (0 if interval_ms is 0).
#[inline]
pub fn calculate_intervals_elapsed(last_refill_ms: u64, now_ms: u64, interval_ms: u64) -> u64 {
    let elapsed = now_ms.saturating_sub(last_refill_ms);
    elapsed.checked_div(interval_ms).unwrap_or(0)
}

/// Compute tokens to add during refill.
///
/// # Arguments
///
/// * `intervals` - Number of refill intervals
/// * `refill_amount` - Amount per interval
/// * `capacity` - Maximum capacity
///
/// # Returns
///
/// Tokens to add (capped at capacity to prevent overflow).
#[inline]
pub fn compute_tokens_to_add(intervals: u64, refill_amount: u64, capacity: u64) -> u64 {
    let raw = intervals.saturating_mul(refill_amount);
    let result = if raw > capacity { capacity } else { raw };
    debug_assert!(result <= capacity, "result must not exceed capacity");
    result
}

/// Compute new last_refill_ms after refill.
#[inline]
pub fn calculate_new_last_refill(last_refill_ms: u64, intervals: u64, refill_interval_ms: u64) -> u64 {
    let increment = intervals.saturating_mul(refill_interval_ms);
    let new_last = last_refill_ms.saturating_add(increment);
    debug_assert!(new_last >= last_refill_ms, "new_last must be >= last_refill_ms");
    new_last
}

// ============================================================================
// Precondition Checks
// ============================================================================

/// Check if acquire is valid.
///
/// Returns true if:
/// - Amount is positive (cannot acquire zero tokens)
/// - Amount does not exceed capacity (request is reasonable)
/// - Sufficient tokens are available
#[inline]
pub fn is_acquire_valid(amount: u64, capacity: u64, tokens: u64) -> bool {
    let is_positive_amount = amount > 0;
    let is_within_capacity = amount <= capacity;
    let has_sufficient_tokens = tokens >= amount;

    is_positive_amount && is_within_capacity && has_sufficient_tokens
}

/// Check if refill is valid (time precondition).
#[inline]
pub fn is_refill_time_valid(current_time_ms: u64, last_refill_ms: u64) -> bool {
    current_time_ms >= last_refill_ms
}

// ============================================================================
// Burst and Rate Calculations
// ============================================================================

/// Check if burst can be handled.
#[inline]
pub fn can_handle_burst_exec(tokens: u64, burst_size: u64) -> bool {
    tokens >= burst_size
}

/// Compute effective rate per second.
///
/// # Verus Alignment Note
///
/// The Verus spec assumes `refill_interval_ms > 0` via a `requires` clause.
/// This production version includes an explicit zero-guard for safety.
#[inline]
pub fn compute_rate_per_second(refill_amount: u64, refill_interval_ms: u64) -> u64 {
    if refill_interval_ms == 0 {
        return 0;
    }
    let numerator = refill_amount.saturating_mul(1000);
    numerator / refill_interval_ms
}

/// Calculate load factor (tokens used / capacity).
#[inline]
pub fn calculate_load_factor(current_load: u32, capacity: u32) -> f64 {
    if capacity == 0 {
        return 0.0;
    }
    let factor = current_load as f64 / capacity as f64;
    debug_assert!(factor >= 0.0, "load factor must be non-negative");
    factor
}

// ============================================================================
// Missing Verus-aligned Functions
// ============================================================================

/// Check if a refill is needed based on time.
///
/// # Arguments
///
/// * `last_refill_ms` - Last refill timestamp (Unix ms)
/// * `now_ms` - Current time (Unix ms)
/// * `interval_ms` - Refill interval in milliseconds
///
/// # Returns
///
/// `true` if at least one interval has elapsed.
#[inline]
pub fn is_refill_needed(last_refill_ms: u64, now_ms: u64, interval_ms: u64) -> bool {
    if interval_ms == 0 {
        false
    } else {
        let elapsed = now_ms.saturating_sub(last_refill_ms);
        elapsed >= interval_ms
    }
}

/// Check if refill is possible (not at capacity).
#[inline]
pub fn is_refill_possible(tokens: u64, capacity: u64) -> bool {
    tokens < capacity
}

/// Compute number of refill intervals elapsed.
///
/// # Verus Alignment Note
///
/// The Verus spec assumes `refill_interval_ms > 0` via a `requires` clause.
/// This production version includes an explicit zero-guard for safety.
#[inline]
pub fn compute_refill_intervals(current_time_ms: u64, last_refill_ms: u64, refill_interval_ms: u64) -> u64 {
    if refill_interval_ms == 0 || current_time_ms < last_refill_ms {
        0
    } else {
        (current_time_ms - last_refill_ms) / refill_interval_ms
    }
}

/// Compute new last_refill_ms after refill.
#[inline]
pub fn compute_new_last_refill(last_refill_ms: u64, intervals: u64, refill_interval_ms: u64) -> u64 {
    let increment = intervals.saturating_mul(refill_interval_ms);
    let new_last = last_refill_ms.saturating_add(increment);
    debug_assert!(new_last >= last_refill_ms, "new_last must be >= last_refill_ms");
    new_last
}

/// Compute tokens after acquire.
#[inline]
pub fn compute_tokens_after_acquire(tokens: u64, amount: u64) -> u64 {
    let remaining = tokens.saturating_sub(amount);
    debug_assert!(remaining <= tokens, "remaining must not exceed tokens");
    remaining
}

/// Compute tokens after refill.
#[inline]
pub fn compute_tokens_after_refill(current_tokens: u64, tokens_to_add: u64, capacity: u64) -> u64 {
    let sum = current_tokens.saturating_add(tokens_to_add);
    let result = if sum > capacity { capacity } else { sum };
    debug_assert!(result <= capacity, "result must not exceed capacity");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Replenishment Tests
    // ========================================================================

    #[test]
    fn test_replenish_one_second() {
        let available = calculate_replenished_tokens(0.0, 1000, 2000, 10.0, 100);
        assert!((available - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_partial_second() {
        let available = calculate_replenished_tokens(0.0, 1000, 1500, 10.0, 100);
        assert!((available - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_caps_at_capacity() {
        let available = calculate_replenished_tokens(90.0, 1000, 2000, 100.0, 100);
        assert!((available - 100.0).abs() < 0.001); // Capped at 100
    }

    #[test]
    fn test_replenish_no_time_elapsed() {
        let available = calculate_replenished_tokens(50.0, 1000, 1000, 10.0, 100);
        assert!((available - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_time_went_backwards() {
        // now_ms < last_update_ms (clock skew or test scenario)
        let available = calculate_replenished_tokens(50.0, 2000, 1000, 10.0, 100);
        // saturating_sub gives 0 elapsed, so tokens stay at 50
        assert!((available - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_replenish_zero_refill_rate() {
        let available = calculate_replenished_tokens(50.0, 1000, 2000, 0.0, 100);
        assert!((available - 50.0).abs() < 0.001); // No replenishment
    }

    // ========================================================================
    // Availability Tests
    // ========================================================================

    #[test]
    fn test_availability_sufficient() {
        let result = check_token_availability(10.0, 5, 10.0);
        match result {
            TokenAvailability::Available { remaining } => {
                assert!((remaining - 5.0).abs() < 0.001);
            }
            _ => panic!("Expected Available"),
        }
    }

    #[test]
    fn test_availability_exact() {
        let result = check_token_availability(5.0, 5, 10.0);
        match result {
            TokenAvailability::Available { remaining } => {
                assert!((remaining - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Available"),
        }
    }

    #[test]
    fn test_availability_insufficient() {
        let result = check_token_availability(3.0, 5, 10.0);
        match result {
            TokenAvailability::Exhausted {
                requested,
                available,
                retry_after_ms,
            } => {
                assert_eq!(requested, 5);
                assert_eq!(available, 3);
                // Need 2 more tokens, at 10/sec = 0.2 sec = 200ms
                assert_eq!(retry_after_ms, 200);
            }
            _ => panic!("Expected Exhausted"),
        }
    }

    #[test]
    fn test_availability_zero_refill() {
        let result = check_token_availability(0.0, 5, 0.0);
        match result {
            TokenAvailability::Exhausted { retry_after_ms, .. } => {
                // With 0 refill rate, retry time is effectively infinite
                assert!(retry_after_ms > 1_000_000);
            }
            _ => panic!("Expected Exhausted"),
        }
    }

    #[test]
    fn test_availability_fractional_tokens() {
        // Available is 4.5, requesting 5
        let result = check_token_availability(4.5, 5, 10.0);
        match result {
            TokenAvailability::Exhausted {
                requested,
                available,
                retry_after_ms,
            } => {
                assert_eq!(requested, 5);
                assert_eq!(available, 4); // Truncated to u64
                // Need 0.5 more tokens, at 10/sec = 0.05 sec = 50ms
                assert_eq!(retry_after_ms, 50);
            }
            _ => panic!("Expected Exhausted"),
        }
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_replenish_never_exceeds_capacity() {
        check!().with_type::<(u64, u64, u64)>().for_each(|(last_update, now, capacity)| {
            if *capacity > 0 {
                let available = calculate_replenished_tokens(0.0, *last_update, *now, 1000.0, *capacity);
                assert!(available <= *capacity as f64, "Replenished tokens must not exceed capacity");
            }
        });
    }

    #[test]
    fn prop_replenish_monotonic_with_time() {
        check!().with_type::<(u64, u64, u64)>().for_each(|(base_time, delta1, delta2)| {
            let now1 = base_time.saturating_add(*delta1);
            let now2 = now1.saturating_add(*delta2);

            let available1 = calculate_replenished_tokens(0.0, *base_time, now1, 10.0, 100);
            let available2 = calculate_replenished_tokens(0.0, *base_time, now2, 10.0, 100);

            assert!(available2 >= available1, "More time should mean more or equal tokens");
        });
    }

    #[test]
    fn prop_availability_check_consistent() {
        check!().with_type::<(u32, u32)>().for_each(|(available, requested)| {
            let result = check_token_availability(*available as f64, *requested as u64, 10.0);
            if *available >= *requested {
                assert!(result.is_available());
            } else {
                assert!(!result.is_available());
            }
        });
    }
}
