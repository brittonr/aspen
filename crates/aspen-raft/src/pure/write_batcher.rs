//! Pure functions for write batcher decision logic.
//!
//! These functions encapsulate the core batch scheduling logic without
//! any async/I/O dependencies, enabling:
//!
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Deterministic behavior verification
//!
//! # Tiger Style
//!
//! - All calculations use explicit types (usize for counts/sizes)
//! - No side effects or I/O
//! - Deterministic pure functions

/// Result of checking whether a batch operation would exceed limits.
///
/// Used to determine if the current batch should be flushed before
/// adding a new operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchLimitCheck {
    /// Whether adding would exceed entry count limit.
    pub exceeds_entries: bool,
    /// Whether adding would exceed byte size limit.
    pub exceeds_bytes: bool,
}

impl BatchLimitCheck {
    /// Returns true if either limit would be exceeded.
    #[inline]
    pub fn would_exceed(&self) -> bool {
        self.exceeds_entries || self.exceeds_bytes
    }
}

/// Check if adding an operation would exceed batch limits.
///
/// This is the core decision function for batch flushing. It determines
/// whether the current batch should be flushed before adding a new
/// operation based on entry count and byte size limits.
///
/// # Arguments
///
/// * `current_entries` - Number of operations currently in the batch
/// * `current_bytes` - Current total size of the batch in bytes
/// * `op_bytes` - Size of the new operation to add in bytes
/// * `max_entries` - Maximum allowed operations per batch
/// * `max_bytes` - Maximum allowed bytes per batch
///
/// # Returns
///
/// `BatchLimitCheck` indicating which limits (if any) would be exceeded.
///
/// # Tiger Style
///
/// - Uses saturating arithmetic to prevent overflow
/// - Explicit limit enforcement for bounded operation
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::check_batch_limits;
///
/// let check = check_batch_limits(
///     99,        // current_entries
///     500_000,   // current_bytes
///     1000,      // op_bytes
///     100,       // max_entries
///     1_000_000, // max_bytes
/// );
///
/// assert!(!check.exceeds_entries); // 99 < 100
/// assert!(!check.exceeds_bytes);   // 501_000 < 1_000_000
/// assert!(!check.would_exceed());
/// ```
#[inline]
pub fn check_batch_limits(
    current_entries: usize,
    current_bytes: usize,
    op_bytes: usize,
    max_entries: usize,
    max_bytes: usize,
) -> BatchLimitCheck {
    // Only check limits if batch is non-empty
    // (we always allow adding to an empty batch)
    let has_pending = current_entries > 0;

    let exceeds_entries = has_pending && current_entries >= max_entries;
    let exceeds_bytes = has_pending && current_bytes.saturating_add(op_bytes) > max_bytes;

    BatchLimitCheck {
        exceeds_entries,
        exceeds_bytes,
    }
}

/// Decision for what flush action to take.
///
/// This enum represents the possible actions after adding an operation
/// to a batch, mirroring the `FlushAction` enum in write_batcher.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushDecision {
    /// Flush the batch immediately (limits reached or batching disabled).
    Immediate,
    /// Schedule a delayed flush (start the timeout timer).
    Delayed,
    /// No action needed (flush already scheduled or batch empty).
    None,
}

/// Determine what flush action to take after adding to a batch.
///
/// This pure function encapsulates the flush scheduling decision logic,
/// determining whether to flush immediately, schedule a delayed flush,
/// or take no action.
///
/// # Arguments
///
/// * `pending_count` - Number of operations in the batch after adding
/// * `current_bytes` - Total bytes in the batch after adding
/// * `max_entries` - Maximum allowed operations per batch
/// * `max_bytes` - Maximum allowed bytes per batch
/// * `max_wait_is_zero` - Whether batching is disabled (max_wait = 0)
/// * `flush_already_scheduled` - Whether a delayed flush is already pending
///
/// # Returns
///
/// `FlushDecision` indicating what action to take.
///
/// # Tiger Style
///
/// - Pure function with no side effects
/// - Explicit decision states (no implicit behavior)
/// - Bounded by configuration limits
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::{determine_flush_action, FlushDecision};
///
/// // Batch is full, flush immediately
/// let decision = determine_flush_action(100, 500_000, 100, 1_000_000, false, false);
/// assert_eq!(decision, FlushDecision::Immediate);
///
/// // Batch has items, not full, no flush scheduled yet
/// let decision = determine_flush_action(10, 1000, 100, 1_000_000, false, false);
/// assert_eq!(decision, FlushDecision::Delayed);
///
/// // Batching disabled, always flush immediately
/// let decision = determine_flush_action(1, 100, 100, 1_000_000, true, false);
/// assert_eq!(decision, FlushDecision::Immediate);
/// ```
#[inline]
pub fn determine_flush_action(
    pending_count: usize,
    current_bytes: usize,
    max_entries: usize,
    max_bytes: usize,
    max_wait_is_zero: bool,
    flush_already_scheduled: bool,
) -> FlushDecision {
    // Immediate flush if batch is full (entries)
    if pending_count >= max_entries {
        return FlushDecision::Immediate;
    }

    // Immediate flush if batch is full (bytes)
    if current_bytes >= max_bytes {
        return FlushDecision::Immediate;
    }

    // No pending items means no flush needed
    if pending_count == 0 {
        return FlushDecision::None;
    }

    // Batching disabled (max_wait = 0) means immediate flush
    if max_wait_is_zero {
        return FlushDecision::Immediate;
    }

    // Schedule delayed flush if not already scheduled
    if !flush_already_scheduled {
        return FlushDecision::Delayed;
    }

    FlushDecision::None
}

/// Calculate the size of a Set operation in bytes.
///
/// # Arguments
///
/// * `key_len` - Length of the key in bytes
/// * `value_len` - Length of the value in bytes
///
/// # Returns
///
/// Total size in bytes (key_len + value_len).
#[inline]
pub const fn calculate_set_op_size(key_len: usize, value_len: usize) -> usize {
    key_len + value_len
}

/// Calculate the size of a Delete operation in bytes.
///
/// # Arguments
///
/// * `key_len` - Length of the key in bytes
///
/// # Returns
///
/// Size in bytes (just key_len, no value).
#[inline]
pub const fn calculate_delete_op_size(key_len: usize) -> usize {
    key_len
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // check_batch_limits tests
    // ========================================================================

    #[test]
    fn test_empty_batch_allows_any_operation() {
        // Empty batch should never report exceeded limits
        let check = check_batch_limits(0, 0, 1_000_000, 1, 1);
        assert!(!check.exceeds_entries);
        assert!(!check.exceeds_bytes);
        assert!(!check.would_exceed());
    }

    #[test]
    fn test_entry_limit_exceeded() {
        let check = check_batch_limits(100, 0, 0, 100, usize::MAX);
        assert!(check.exceeds_entries);
        assert!(!check.exceeds_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_entry_limit_not_exceeded() {
        let check = check_batch_limits(99, 0, 0, 100, usize::MAX);
        assert!(!check.exceeds_entries);
        assert!(!check.would_exceed());
    }

    #[test]
    fn test_byte_limit_exceeded() {
        let check = check_batch_limits(1, 900_000, 200_000, 100, 1_000_000);
        assert!(!check.exceeds_entries);
        assert!(check.exceeds_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_byte_limit_not_exceeded() {
        let check = check_batch_limits(1, 500_000, 499_999, 100, 1_000_000);
        assert!(!check.exceeds_bytes);
        assert!(!check.would_exceed());
    }

    #[test]
    fn test_byte_limit_exact_boundary() {
        // Exactly at the limit is OK (uses > not >=)
        let check = check_batch_limits(1, 500_000, 500_000, 100, 1_000_000);
        assert!(!check.exceeds_bytes);
    }

    #[test]
    fn test_byte_limit_one_over() {
        let check = check_batch_limits(1, 500_000, 500_001, 100, 1_000_000);
        assert!(check.exceeds_bytes);
    }

    #[test]
    fn test_both_limits_exceeded() {
        let check = check_batch_limits(100, 1_000_000, 1, 100, 1_000_000);
        assert!(check.exceeds_entries);
        assert!(check.exceeds_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_saturating_arithmetic_prevents_overflow() {
        // Should not overflow when adding huge values
        let check = check_batch_limits(1, usize::MAX - 1, usize::MAX, 100, usize::MAX);
        // This should saturate to usize::MAX, not wrap around
        assert!(!check.exceeds_bytes); // usize::MAX <= usize::MAX
    }

    // ========================================================================
    // determine_flush_action tests
    // ========================================================================

    #[test]
    fn test_flush_when_entries_full() {
        let decision = determine_flush_action(100, 0, 100, 1_000_000, false, false);
        assert_eq!(decision, FlushDecision::Immediate);
    }

    #[test]
    fn test_flush_when_bytes_full() {
        let decision = determine_flush_action(1, 1_000_000, 100, 1_000_000, false, false);
        assert_eq!(decision, FlushDecision::Immediate);
    }

    #[test]
    fn test_no_flush_when_empty() {
        let decision = determine_flush_action(0, 0, 100, 1_000_000, false, false);
        assert_eq!(decision, FlushDecision::None);
    }

    #[test]
    fn test_immediate_flush_when_batching_disabled() {
        let decision = determine_flush_action(1, 100, 100, 1_000_000, true, false);
        assert_eq!(decision, FlushDecision::Immediate);
    }

    #[test]
    fn test_delayed_flush_when_not_scheduled() {
        let decision = determine_flush_action(1, 100, 100, 1_000_000, false, false);
        assert_eq!(decision, FlushDecision::Delayed);
    }

    #[test]
    fn test_no_action_when_flush_already_scheduled() {
        let decision = determine_flush_action(1, 100, 100, 1_000_000, false, true);
        assert_eq!(decision, FlushDecision::None);
    }

    #[test]
    fn test_immediate_overrides_scheduled() {
        // Even if flush is scheduled, immediate takes precedence when full
        let decision = determine_flush_action(100, 0, 100, 1_000_000, false, true);
        assert_eq!(decision, FlushDecision::Immediate);
    }

    // ========================================================================
    // Size calculation tests
    // ========================================================================

    #[test]
    fn test_set_op_size() {
        assert_eq!(calculate_set_op_size(8, 10), 18);
        assert_eq!(calculate_set_op_size(0, 0), 0);
        assert_eq!(calculate_set_op_size(1000, 1_000_000), 1_001_000);
    }

    #[test]
    fn test_delete_op_size() {
        assert_eq!(calculate_delete_op_size(8), 8);
        assert_eq!(calculate_delete_op_size(0), 0);
    }
}
