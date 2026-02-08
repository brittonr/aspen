//! Pure counter computation functions.
//!
//! This module contains pure functions for atomic counter operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Deterministic behavior (no I/O, no system calls)
//! - Explicit types (u64, i64)
//! - No panics - all functions are total

// ============================================================================
// Unsigned Counter Operations
// ============================================================================

/// Result of applying an operation to a counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterOpResult {
    /// The new value after the operation
    pub new_value: u64,
    /// Whether saturation occurred (hit 0 or MAX)
    pub saturated: bool,
}

/// Apply an increment operation with saturation.
///
/// # Arguments
///
/// * `current` - Current counter value
/// * `amount` - Amount to add
///
/// # Returns
///
/// Result with new value and saturation flag.
///
/// # Example
///
/// ```ignore
/// let result = apply_increment(100, 50);
/// assert_eq!(result.new_value, 150);
/// assert!(!result.saturated);
///
/// let overflow = apply_increment(u64::MAX - 10, 20);
/// assert_eq!(overflow.new_value, u64::MAX);
/// assert!(overflow.saturated);
/// ```
#[inline]
pub fn apply_increment(current: u64, amount: u64) -> CounterOpResult {
    let new_value = current.saturating_add(amount);
    let saturated = new_value != current.wrapping_add(amount);
    CounterOpResult { new_value, saturated }
}

/// Apply a decrement operation with saturation at zero.
///
/// # Arguments
///
/// * `current` - Current counter value
/// * `amount` - Amount to subtract
///
/// # Returns
///
/// Result with new value and saturation flag.
///
/// # Example
///
/// ```ignore
/// let result = apply_decrement(100, 30);
/// assert_eq!(result.new_value, 70);
/// assert!(!result.saturated);
///
/// let underflow = apply_decrement(10, 20);
/// assert_eq!(underflow.new_value, 0);
/// assert!(underflow.saturated);
/// ```
#[inline]
pub fn apply_decrement(current: u64, amount: u64) -> CounterOpResult {
    let new_value = current.saturating_sub(amount);
    let saturated = new_value != current.wrapping_sub(amount);
    CounterOpResult { new_value, saturated }
}

// ============================================================================
// Signed Counter Operations
// ============================================================================

/// Result of applying an operation to a signed counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedCounterOpResult {
    /// The new value after the operation
    pub new_value: i64,
    /// Whether saturation occurred (hit MIN or MAX)
    pub saturated: bool,
}

/// Apply a signed addition with saturation.
///
/// # Arguments
///
/// * `current` - Current counter value
/// * `amount` - Amount to add (can be negative)
///
/// # Returns
///
/// Result with new value and saturation flag.
#[inline]
pub fn apply_signed_add(current: i64, amount: i64) -> SignedCounterOpResult {
    let new_value = current.saturating_add(amount);
    let saturated = new_value != current.wrapping_add(amount);
    SignedCounterOpResult { new_value, saturated }
}

/// Apply a signed subtraction with saturation.
///
/// # Arguments
///
/// * `current` - Current counter value
/// * `amount` - Amount to subtract
///
/// # Returns
///
/// Result with new value and saturation flag.
#[inline]
pub fn apply_signed_sub(current: i64, amount: i64) -> SignedCounterOpResult {
    let new_value = current.saturating_sub(amount);
    let saturated = new_value != current.wrapping_sub(amount);
    SignedCounterOpResult { new_value, saturated }
}

// ============================================================================
// CAS Expected Value
// ============================================================================

/// Compute the expected value for CAS operations on unsigned counter.
///
/// Returns `None` for zero (no existing value), `Some(current)` otherwise.
///
/// # Arguments
///
/// * `current` - Current counter value
///
/// # Returns
///
/// Expected value for CAS operation.
#[inline]
pub fn compute_unsigned_cas_expected(current: u64) -> Option<u64> {
    if current == 0 {
        None
    } else {
        Some(current)
    }
}

/// Compute the expected value for CAS operations on signed counter.
///
/// Returns `None` for zero (no existing value), `Some(current)` otherwise.
///
/// # Arguments
///
/// * `current` - Current counter value
///
/// # Returns
///
/// Expected value for CAS operation.
#[inline]
pub fn compute_signed_cas_expected(current: i64) -> Option<i64> {
    if current == 0 {
        None
    } else {
        Some(current)
    }
}

// ============================================================================
// Value Parsing
// ============================================================================

/// Result of parsing an unsigned counter value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseUnsignedResult {
    /// Successfully parsed value
    Value(u64),
    /// String was empty (treat as 0)
    Empty,
    /// String was not a valid u64
    Invalid,
}

/// Parse an unsigned counter value from storage.
///
/// # Arguments
///
/// * `value_str` - String value from storage
///
/// # Returns
///
/// Parsed result.
#[inline]
pub fn parse_unsigned_counter(value_str: &str) -> ParseUnsignedResult {
    if value_str.is_empty() {
        return ParseUnsignedResult::Empty;
    }
    match value_str.parse::<u64>() {
        Ok(v) => ParseUnsignedResult::Value(v),
        Err(_) => ParseUnsignedResult::Invalid,
    }
}

/// Result of parsing a signed counter value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSignedResult {
    /// Successfully parsed value
    Value(i64),
    /// String was empty (treat as 0)
    Empty,
    /// String was not a valid i64
    Invalid,
}

/// Parse a signed counter value from storage.
///
/// # Arguments
///
/// * `value_str` - String value from storage
///
/// # Returns
///
/// Parsed result.
#[inline]
pub fn parse_signed_counter(value_str: &str) -> ParseSignedResult {
    if value_str.is_empty() {
        return ParseSignedResult::Empty;
    }
    match value_str.parse::<i64>() {
        Ok(v) => ParseSignedResult::Value(v),
        Err(_) => ParseSignedResult::Invalid,
    }
}

// ============================================================================
// Buffered Counter Logic
// ============================================================================

/// Check if a buffered counter should flush based on threshold.
///
/// # Arguments
///
/// * `local_value` - Current local accumulated value
/// * `flush_threshold` - Threshold that triggers flush
///
/// # Returns
///
/// `true` if the counter should be flushed.
#[inline]
pub fn should_flush_buffer(local_value: u64, flush_threshold: u64) -> bool {
    local_value >= flush_threshold
}

/// Compute approximate total for a buffered counter.
///
/// # Arguments
///
/// * `stored_value` - Value stored in distributed storage
/// * `local_value` - Unflushed local accumulator
///
/// # Returns
///
/// Approximate total (may be slightly stale).
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_approximate_total(stored_value: u64, local_value: u64) -> u64 {
    stored_value.saturating_add(local_value)
}

// ============================================================================
// Retry Backoff
// ============================================================================

/// Compute jittered backoff delay for CAS retry.
///
/// # Arguments
///
/// * `base_delay_ms` - Base delay in milliseconds
/// * `jitter_value` - Random value for jitter (should be in 0..base_delay_ms+1)
///
/// # Returns
///
/// Delay to use, clamped to base_delay_ms.
///
/// # Tiger Style
///
/// - Clamps result to prevent excessive delays
#[inline]
pub fn compute_retry_delay(base_delay_ms: u64, jitter_value: u64) -> u64 {
    jitter_value.min(base_delay_ms)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Unsigned Increment Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_apply_increment_normal() {
        let result = apply_increment(100, 50);
        assert_eq!(result.new_value, 150);
        assert!(!result.saturated);
    }

    #[test]
    fn test_apply_increment_zero() {
        let result = apply_increment(0, 100);
        assert_eq!(result.new_value, 100);
        assert!(!result.saturated);
    }

    #[test]
    fn test_apply_increment_saturation() {
        let result = apply_increment(u64::MAX - 10, 20);
        assert_eq!(result.new_value, u64::MAX);
        assert!(result.saturated);
    }

    #[test]
    fn test_apply_increment_at_max() {
        let result = apply_increment(u64::MAX, 1);
        assert_eq!(result.new_value, u64::MAX);
        assert!(result.saturated);
    }

    // ------------------------------------------------------------------------
    // Unsigned Decrement Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_apply_decrement_normal() {
        let result = apply_decrement(100, 30);
        assert_eq!(result.new_value, 70);
        assert!(!result.saturated);
    }

    #[test]
    fn test_apply_decrement_to_zero() {
        let result = apply_decrement(50, 50);
        assert_eq!(result.new_value, 0);
        assert!(!result.saturated);
    }

    #[test]
    fn test_apply_decrement_saturation() {
        let result = apply_decrement(10, 20);
        assert_eq!(result.new_value, 0);
        assert!(result.saturated);
    }

    #[test]
    fn test_apply_decrement_from_zero() {
        let result = apply_decrement(0, 100);
        assert_eq!(result.new_value, 0);
        assert!(result.saturated);
    }

    // ------------------------------------------------------------------------
    // Signed Operation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_apply_signed_add_positive() {
        let result = apply_signed_add(100, 50);
        assert_eq!(result.new_value, 150);
        assert!(!result.saturated);
    }

    #[test]
    fn test_apply_signed_add_negative() {
        let result = apply_signed_add(100, -30);
        assert_eq!(result.new_value, 70);
        assert!(!result.saturated);
    }

    #[test]
    fn test_apply_signed_add_overflow() {
        let result = apply_signed_add(i64::MAX - 10, 20);
        assert_eq!(result.new_value, i64::MAX);
        assert!(result.saturated);
    }

    #[test]
    fn test_apply_signed_add_underflow() {
        let result = apply_signed_add(i64::MIN + 10, -20);
        assert_eq!(result.new_value, i64::MIN);
        assert!(result.saturated);
    }

    #[test]
    fn test_apply_signed_sub() {
        let result = apply_signed_sub(100, 30);
        assert_eq!(result.new_value, 70);
        assert!(!result.saturated);
    }

    // ------------------------------------------------------------------------
    // CAS Expected Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_compute_unsigned_cas_expected() {
        assert_eq!(compute_unsigned_cas_expected(0), None);
        assert_eq!(compute_unsigned_cas_expected(100), Some(100));
        assert_eq!(compute_unsigned_cas_expected(u64::MAX), Some(u64::MAX));
    }

    #[test]
    fn test_compute_signed_cas_expected() {
        assert_eq!(compute_signed_cas_expected(0), None);
        assert_eq!(compute_signed_cas_expected(100), Some(100));
        assert_eq!(compute_signed_cas_expected(-100), Some(-100));
    }

    // ------------------------------------------------------------------------
    // Parsing Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_unsigned_counter() {
        assert_eq!(parse_unsigned_counter("123"), ParseUnsignedResult::Value(123));
        assert_eq!(parse_unsigned_counter("0"), ParseUnsignedResult::Value(0));
        assert_eq!(parse_unsigned_counter(""), ParseUnsignedResult::Empty);
        assert_eq!(parse_unsigned_counter("abc"), ParseUnsignedResult::Invalid);
        assert_eq!(parse_unsigned_counter("-1"), ParseUnsignedResult::Invalid);
    }

    #[test]
    fn test_parse_signed_counter() {
        assert_eq!(parse_signed_counter("123"), ParseSignedResult::Value(123));
        assert_eq!(parse_signed_counter("-123"), ParseSignedResult::Value(-123));
        assert_eq!(parse_signed_counter("0"), ParseSignedResult::Value(0));
        assert_eq!(parse_signed_counter(""), ParseSignedResult::Empty);
        assert_eq!(parse_signed_counter("abc"), ParseSignedResult::Invalid);
    }

    // ------------------------------------------------------------------------
    // Buffered Counter Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_should_flush_buffer() {
        assert!(!should_flush_buffer(50, 100));
        assert!(should_flush_buffer(100, 100));
        assert!(should_flush_buffer(150, 100));
        assert!(!should_flush_buffer(0, 100));
    }

    #[test]
    fn test_compute_approximate_total() {
        assert_eq!(compute_approximate_total(100, 50), 150);
        assert_eq!(compute_approximate_total(0, 100), 100);
        assert_eq!(compute_approximate_total(u64::MAX - 10, 20), u64::MAX); // Saturates
    }

    // ------------------------------------------------------------------------
    // Retry Delay Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_compute_retry_delay() {
        assert_eq!(compute_retry_delay(10, 5), 5);
        assert_eq!(compute_retry_delay(10, 15), 10); // Clamped
        assert_eq!(compute_retry_delay(10, 10), 10);
        assert_eq!(compute_retry_delay(10, 0), 0);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_increment_monotonic() {
        check!()
            .with_type::<(u64, u64)>()
            .for_each(|(current, amount)| {
                let result = apply_increment(*current, *amount);
                assert!(result.new_value >= *current);
            });
    }

    #[test]
    fn prop_decrement_bounded() {
        check!()
            .with_type::<(u64, u64)>()
            .for_each(|(current, amount)| {
                let result = apply_decrement(*current, *amount);
                assert!(result.new_value <= *current);
            });
    }

    #[test]
    fn prop_signed_add_no_panic() {
        check!()
            .with_type::<(i64, i64)>()
            .for_each(|(current, amount)| {
                // Should never panic
                let _ = apply_signed_add(*current, *amount);
            });
    }

    #[test]
    fn prop_saturation_detected() {
        check!()
            .with_type::<(u64, u64)>()
            .for_each(|(current, amount)| {
                let result = apply_increment(*current, *amount);
                // If saturated, we should be at MAX
                if result.saturated {
                    assert_eq!(result.new_value, u64::MAX);
                }
            });
    }

    #[test]
    fn prop_approximate_total_bounded() {
        check!()
            .with_type::<(u64, u64)>()
            .for_each(|(stored, local)| {
                let total = compute_approximate_total(*stored, *local);
                assert!(total >= *stored);
                assert!(total >= *local);
            });
    }
}
