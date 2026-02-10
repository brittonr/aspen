//! Pure sequence generator computation functions.
//!
//! This module contains pure functions for distributed sequence operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating/checked arithmetic for all calculations
//! - Deterministic behavior (no I/O, no system calls)
//! - Explicit types (u64, not usize)
//! - No panics - all functions are total

// ============================================================================
// Batch State Checks
// ============================================================================

/// Check if the local batch needs to be refilled.
///
/// A batch needs refill when `next >= batch_end`, meaning all
/// pre-reserved IDs have been consumed.
///
/// # Arguments
///
/// * `next` - Next ID to be returned
/// * `batch_end` - End of current batch (exclusive)
///
/// # Returns
///
/// `true` if a new batch should be reserved from the cluster.
///
/// # Example
///
/// ```ignore
/// assert!(!should_refill_batch(5, 100));  // Still have IDs
/// assert!(should_refill_batch(100, 100)); // Exhausted
/// assert!(should_refill_batch(150, 100)); // Past end
/// ```
#[inline]
pub fn should_refill_batch(next: u64, batch_end: u64) -> bool {
    next >= batch_end
}

/// Check if a batch has any IDs remaining.
///
/// # Arguments
///
/// * `next` - Next ID to be returned
/// * `batch_end` - End of current batch (exclusive)
///
/// # Returns
///
/// Number of IDs remaining in the batch.
///
/// # Tiger Style
///
/// - Uses saturating_sub to handle underflow (returns 0)
#[inline]
pub fn batch_remaining(next: u64, batch_end: u64) -> u64 {
    batch_end.saturating_sub(next)
}

// ============================================================================
// Batch Computation
// ============================================================================

/// Compute the end of a batch given start and size.
///
/// # Arguments
///
/// * `batch_start` - Start of the batch (inclusive)
/// * `batch_size` - Number of IDs in the batch
///
/// # Returns
///
/// `Some(end)` if the batch fits, `None` if it would overflow.
///
/// # Tiger Style
///
/// - Uses checked_add to detect overflow
#[inline]
pub fn compute_batch_end(batch_start: u64, batch_size: u64) -> Option<u64> {
    batch_start.checked_add(batch_size)
}

/// Compute the next ID pointer after refilling a batch.
///
/// When a new batch is reserved starting at `batch_start`, we return
/// `batch_start` and advance next to `batch_start + 1`.
///
/// # Arguments
///
/// * `batch_start` - Start of the newly reserved batch
///
/// # Returns
///
/// `Some(next)` where next = batch_start + 1, or `None` on overflow.
///
/// # Tiger Style
///
/// - Uses checked_add to detect overflow
#[inline]
pub fn compute_next_after_refill(batch_start: u64) -> Option<u64> {
    batch_start.checked_add(1)
}

// ============================================================================
// Sequence Reservation
// ============================================================================

/// Result of computing a new sequence value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceReservationResult {
    /// Reservation succeeded, contains the new stored value
    Success { new_value: u64 },
    /// Would overflow u64
    Overflow,
}

/// Compute the new stored value after reserving a count of IDs.
///
/// The stored value tracks the highest allocated ID. Reserving `count`
/// IDs advances it by `count`.
///
/// # Arguments
///
/// * `current` - Current stored value (highest allocated so far)
/// * `count` - Number of IDs to reserve
///
/// # Returns
///
/// Result indicating success with new value, or overflow.
///
/// # Example
///
/// ```ignore
/// let result = compute_new_sequence_value(100, 50);
/// assert_eq!(result, SequenceReservationResult::Success { new_value: 150 });
///
/// let overflow = compute_new_sequence_value(u64::MAX - 10, 20);
/// assert_eq!(overflow, SequenceReservationResult::Overflow);
/// ```
///
/// # Tiger Style
///
/// - Uses checked_add to detect overflow
/// - Returns enum instead of Option for clarity
#[inline]
pub fn compute_new_sequence_value(current: u64, count: u64) -> SequenceReservationResult {
    match current.checked_add(count) {
        Some(new_value) => SequenceReservationResult::Success { new_value },
        None => SequenceReservationResult::Overflow,
    }
}

/// Compute the start of a reserved range.
///
/// When the stored value is `current`, the next range starts at `current + 1`.
///
/// # Arguments
///
/// * `current` - Current stored value
///
/// # Returns
///
/// `Some(start)` or `None` on overflow.
///
/// # Tiger Style
///
/// - Uses checked_add to detect overflow
#[inline]
pub fn compute_range_start(current: u64) -> Option<u64> {
    current.checked_add(1)
}

// ============================================================================
// Initial State Detection
// ============================================================================

/// Check if this is the initial reservation (sequence not yet created).
///
/// # Arguments
///
/// * `current` - Current stored value
/// * `start_value` - Configured start value for sequences
///
/// # Returns
///
/// `true` if this is the first reservation (current < start_value).
#[inline]
pub fn is_initial_reservation(current: u64, start_value: u64) -> bool {
    current < start_value
}

/// Compute the initial current value for a new sequence.
///
/// New sequences start with current = start_value - 1, so the first
/// reserved ID is start_value.
///
/// # Arguments
///
/// * `start_value` - Configured start value for sequences
///
/// # Returns
///
/// Initial current value (start_value - 1), using saturating_sub.
///
/// # Tiger Style
///
/// - Uses saturating_sub to handle start_value = 0
#[inline]
pub fn compute_initial_current(start_value: u64) -> u64 {
    start_value.saturating_sub(1)
}

// ============================================================================
// Value Parsing
// ============================================================================

/// Result of parsing a sequence value from storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseSequenceResult {
    /// Successfully parsed value
    Value(u64),
    /// String was empty (treat as not found)
    Empty,
    /// String was not a valid u64
    Invalid,
}

/// Parse a stored sequence value.
///
/// # Arguments
///
/// * `value_str` - String value from storage
///
/// # Returns
///
/// Parsed result indicating success, empty, or invalid.
///
/// # Example
///
/// ```ignore
/// assert_eq!(parse_sequence_value("123"), ParseSequenceResult::Value(123));
/// assert_eq!(parse_sequence_value(""), ParseSequenceResult::Empty);
/// assert_eq!(parse_sequence_value("abc"), ParseSequenceResult::Invalid);
/// ```
#[inline]
pub fn parse_sequence_value(value_str: &str) -> ParseSequenceResult {
    if value_str.is_empty() {
        return ParseSequenceResult::Empty;
    }
    match value_str.parse::<u64>() {
        Ok(v) => ParseSequenceResult::Value(v),
        Err(_) => ParseSequenceResult::Invalid,
    }
}

/// Compute the expected value for CAS operation.
///
/// Returns `None` for initial reservation (no existing value),
/// or `Some(current_string)` for subsequent reservations.
///
/// # Arguments
///
/// * `current` - Current stored value
/// * `start_value` - Configured start value
///
/// # Returns
///
/// Expected value for CAS, or None for initial creation.
#[inline]
pub fn compute_cas_expected(current: u64, start_value: u64) -> Option<u64> {
    if is_initial_reservation(current, start_value) {
        None
    } else {
        Some(current)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Batch State Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_should_refill_batch() {
        assert!(!should_refill_batch(5, 100)); // Has remaining
        assert!(!should_refill_batch(99, 100)); // Has 1 remaining
        assert!(should_refill_batch(100, 100)); // Exhausted
        assert!(should_refill_batch(150, 100)); // Past end
    }

    #[test]
    fn test_should_refill_batch_zero() {
        // Empty batch
        assert!(should_refill_batch(0, 0));
        assert!(should_refill_batch(1, 0));
    }

    #[test]
    fn test_batch_remaining() {
        assert_eq!(batch_remaining(5, 100), 95);
        assert_eq!(batch_remaining(100, 100), 0);
        assert_eq!(batch_remaining(150, 100), 0); // Saturates to 0
    }

    // ------------------------------------------------------------------------
    // Batch Computation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_compute_batch_end() {
        assert_eq!(compute_batch_end(1, 100), Some(101));
        assert_eq!(compute_batch_end(0, 100), Some(100));
        assert_eq!(compute_batch_end(u64::MAX - 50, 50), Some(u64::MAX));
        assert_eq!(compute_batch_end(u64::MAX - 50, 51), None); // Overflow
        assert_eq!(compute_batch_end(u64::MAX, 1), None);
    }

    #[test]
    fn test_compute_next_after_refill() {
        assert_eq!(compute_next_after_refill(0), Some(1));
        assert_eq!(compute_next_after_refill(100), Some(101));
        assert_eq!(compute_next_after_refill(u64::MAX - 1), Some(u64::MAX));
        assert_eq!(compute_next_after_refill(u64::MAX), None);
    }

    // ------------------------------------------------------------------------
    // Sequence Reservation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_compute_new_sequence_value() {
        assert_eq!(compute_new_sequence_value(100, 50), SequenceReservationResult::Success { new_value: 150 });
        assert_eq!(compute_new_sequence_value(0, 100), SequenceReservationResult::Success { new_value: 100 });
        assert_eq!(compute_new_sequence_value(u64::MAX - 10, 20), SequenceReservationResult::Overflow);
        assert_eq!(compute_new_sequence_value(u64::MAX, 1), SequenceReservationResult::Overflow);
    }

    #[test]
    fn test_compute_range_start() {
        assert_eq!(compute_range_start(0), Some(1));
        assert_eq!(compute_range_start(100), Some(101));
        assert_eq!(compute_range_start(u64::MAX - 1), Some(u64::MAX));
        assert_eq!(compute_range_start(u64::MAX), None);
    }

    // ------------------------------------------------------------------------
    // Initial State Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_initial_reservation() {
        assert!(is_initial_reservation(0, 1));
        assert!(!is_initial_reservation(1, 1));
        assert!(!is_initial_reservation(100, 1));
        assert!(is_initial_reservation(5, 10));
        assert!(!is_initial_reservation(10, 10));
    }

    #[test]
    fn test_compute_initial_current() {
        assert_eq!(compute_initial_current(1), 0);
        assert_eq!(compute_initial_current(100), 99);
        assert_eq!(compute_initial_current(0), 0); // Saturates
    }

    // ------------------------------------------------------------------------
    // Parsing Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_sequence_value() {
        assert_eq!(parse_sequence_value("123"), ParseSequenceResult::Value(123));
        assert_eq!(parse_sequence_value("0"), ParseSequenceResult::Value(0));
        assert_eq!(parse_sequence_value(&u64::MAX.to_string()), ParseSequenceResult::Value(u64::MAX));
        assert_eq!(parse_sequence_value(""), ParseSequenceResult::Empty);
        assert_eq!(parse_sequence_value("abc"), ParseSequenceResult::Invalid);
        assert_eq!(parse_sequence_value("-1"), ParseSequenceResult::Invalid);
        assert_eq!(parse_sequence_value("1.5"), ParseSequenceResult::Invalid);
    }

    #[test]
    fn test_compute_cas_expected() {
        assert_eq!(compute_cas_expected(0, 1), None); // Initial
        assert_eq!(compute_cas_expected(1, 1), Some(1)); // Not initial
        assert_eq!(compute_cas_expected(100, 1), Some(100));
        assert_eq!(compute_cas_expected(5, 10), None); // current < start
        assert_eq!(compute_cas_expected(10, 10), Some(10));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_batch_remaining_bounded() {
        check!().with_type::<(u64, u64)>().for_each(|(next, end)| {
            let remaining = batch_remaining(*next, *end);
            assert!(remaining <= *end);
        });
    }

    #[test]
    fn prop_compute_batch_end_no_panic() {
        check!().with_type::<(u64, u64)>().for_each(|(start, size)| {
            // Should never panic
            let _ = compute_batch_end(*start, *size);
        });
    }

    #[test]
    fn prop_sequence_reservation_no_panic() {
        check!().with_type::<(u64, u64)>().for_each(|(current, count)| {
            // Should never panic
            let _ = compute_new_sequence_value(*current, *count);
        });
    }

    #[test]
    fn prop_batch_refill_consistent() {
        check!().with_type::<(u64, u64)>().for_each(|(next, end)| {
            let needs_refill = should_refill_batch(*next, *end);
            let remaining = batch_remaining(*next, *end);
            // If remaining > 0, shouldn't need refill
            if remaining > 0 {
                assert!(!needs_refill);
            }
            // If needs refill, remaining should be 0
            if needs_refill {
                assert_eq!(remaining, 0);
            }
        });
    }

    #[test]
    fn prop_initial_reservation_consistent() {
        check!().with_type::<(u64, u64)>().for_each(|(current, start)| {
            let is_initial = is_initial_reservation(*current, *start);
            let expected = compute_cas_expected(*current, *start);
            // If initial, expected should be None
            if is_initial {
                assert!(expected.is_none());
            } else {
                assert!(expected.is_some());
            }
        });
    }
}
