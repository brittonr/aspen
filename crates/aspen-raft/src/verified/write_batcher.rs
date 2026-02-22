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
//! - All calculations use explicit types (u32 for counts, u64 for byte sizes)
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
/// use aspen_raft::verified::check_batch_limits;
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
    current_entries: u32,
    current_bytes: u64,
    op_bytes: u64,
    max_entries: u32,
    max_bytes: u64,
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
/// use aspen_raft::verified::{determine_flush_action, FlushDecision};
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
    pending_count: u32,
    current_bytes: u64,
    max_entries: u32,
    max_bytes: u64,
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
pub const fn calculate_set_op_size(key_len: u64, value_len: u64) -> u64 {
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
pub const fn calculate_delete_op_size(key_len: u64) -> u64 {
    key_len
}

// ============================================================================
// Batch Add Operations
// ============================================================================

/// Check if key is valid for add operation.
///
/// # Arguments
///
/// * `key_len` - Length of the key
///
/// # Returns
///
/// `true` if key is non-empty.
#[inline]
#[allow(dead_code)]
pub const fn is_key_valid(key_len: u64) -> bool {
    key_len > 0
}

/// Compute operation size in bytes.
///
/// # Arguments
///
/// * `key_len` - Length of the key
/// * `value_len` - Length of the value
///
/// # Returns
///
/// Total operation size (saturating at u64::MAX).
#[inline]
#[allow(dead_code)]
pub const fn compute_op_size(key_len: u64, value_len: u64) -> u64 {
    key_len.saturating_add(value_len)
}

/// Check if operation fits within max bytes limit.
///
/// # Arguments
///
/// * `op_bytes` - Size of the operation
/// * `max_bytes` - Maximum batch bytes
///
/// # Returns
///
/// `true` if operation fits.
#[inline]
#[allow(dead_code)]
pub const fn does_op_fit(op_bytes: u64, max_bytes: u64) -> bool {
    op_bytes <= max_bytes
}

/// Check if sequence can be incremented.
///
/// # Arguments
///
/// * `next_sequence` - Current next sequence
///
/// # Returns
///
/// `true` if sequence can be incremented.
#[inline]
#[allow(dead_code)]
pub const fn can_increment_sequence(next_sequence: u64) -> bool {
    next_sequence < u64::MAX
}

/// Compute next sequence number.
///
/// # Arguments
///
/// * `current_sequence` - Current sequence
///
/// # Returns
///
/// Next sequence (saturating at u64::MAX).
#[inline]
#[allow(dead_code)]
pub const fn compute_next_sequence(current_sequence: u64) -> u64 {
    current_sequence.saturating_add(1)
}

/// Check if add would trigger flush.
///
/// # Arguments
///
/// * `pending_len` - Current pending count
/// * `max_entries` - Maximum entries
/// * `current_bytes` - Current batch bytes
/// * `max_bytes` - Maximum batch bytes
/// * `op_bytes` - Size of operation to add
///
/// # Returns
///
/// `true` if add would trigger flush.
///
/// # Safety
///
/// Requires `current_bytes <= max_bytes` (from bytes_bounded invariant).
#[inline]
#[allow(dead_code)]
pub fn would_add_trigger_flush(
    pending_len: u32,
    max_entries: u32,
    current_bytes: u64,
    max_bytes: u64,
    op_bytes: u64,
) -> bool {
    pending_len >= max_entries || op_bytes > max_bytes - current_bytes
}

/// Compute new batch start time.
///
/// First write sets batch start to current time,
/// subsequent writes preserve existing start.
///
/// # Arguments
///
/// * `pending_len` - Current pending count
/// * `current_batch_start` - Current batch start time
/// * `current_time_ms` - Current time
///
/// # Returns
///
/// New batch start time.
#[inline]
#[allow(dead_code)]
pub const fn compute_batch_start(pending_len: u32, current_batch_start: u64, current_time_ms: u64) -> u64 {
    if pending_len == 0 {
        current_time_ms
    } else {
        current_batch_start
    }
}

/// Compute new current bytes after add.
///
/// # Arguments
///
/// * `current_bytes` - Current batch bytes
/// * `op_bytes` - Size of operation to add
///
/// # Returns
///
/// New current bytes (saturating at u64::MAX).
#[inline]
#[allow(dead_code)]
pub const fn compute_bytes_after_add(current_bytes: u64, op_bytes: u64) -> u64 {
    current_bytes.saturating_add(op_bytes)
}

/// Check if operation is batchable.
///
/// # Arguments
///
/// * `is_set` - Whether this is a Set operation
/// * `is_delete` - Whether this is a Delete operation
///
/// # Returns
///
/// `true` if operation is batchable.
#[inline]
#[allow(dead_code)]
pub const fn is_batchable_operation(is_set: bool, is_delete: bool) -> bool {
    is_set || is_delete
}

// ============================================================================
// Batch Flush Operations
// ============================================================================

/// Check if flush is needed (has pending writes).
///
/// # Arguments
///
/// * `pending_len` - Number of pending writes
///
/// # Returns
///
/// `true` if there are pending writes to flush.
#[inline]
#[allow(dead_code)]
pub const fn should_flush(pending_len: u32) -> bool {
    pending_len > 0
}

/// Check if batch is at entries limit.
///
/// # Arguments
///
/// * `pending_len` - Current pending count
/// * `max_entries` - Maximum entries
///
/// # Returns
///
/// `true` if at or above entries limit.
#[inline]
#[allow(dead_code)]
pub const fn is_entries_full(pending_len: u32, max_entries: u32) -> bool {
    pending_len >= max_entries
}

/// Check if batch is at bytes limit.
///
/// # Arguments
///
/// * `current_bytes` - Current batch bytes
/// * `max_bytes` - Maximum bytes
///
/// # Returns
///
/// `true` if at or above bytes limit.
#[inline]
#[allow(dead_code)]
pub const fn is_bytes_full(current_bytes: u64, max_bytes: u64) -> bool {
    current_bytes >= max_bytes
}

/// Check if immediate flush mode is enabled.
///
/// # Arguments
///
/// * `max_wait_ms` - Maximum wait time (0 = immediate)
///
/// # Returns
///
/// `true` if immediate flush mode.
#[inline]
#[allow(dead_code)]
pub const fn is_immediate_mode(max_wait_ms: u64) -> bool {
    max_wait_ms == 0
}

/// Check if timeout has elapsed.
///
/// # Arguments
///
/// * `batch_start_ms` - When batch started
/// * `current_time_ms` - Current time
/// * `max_wait_ms` - Maximum wait time
///
/// # Returns
///
/// `true` if timeout has elapsed.
#[inline]
#[allow(dead_code)]
pub fn is_timeout_elapsed(batch_start_ms: u64, current_time_ms: u64, max_wait_ms: u64) -> bool {
    current_time_ms.saturating_sub(batch_start_ms) >= max_wait_ms
}

/// Flush reason enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FlushReason {
    EntriesFull,
    BytesFull,
    Timeout,
    Immediate,
}

/// Determine flush reason.
///
/// # Arguments
///
/// * `pending_len` - Current pending count
/// * `max_entries` - Maximum entries
/// * `current_bytes` - Current bytes
/// * `max_bytes` - Maximum bytes
/// * `max_wait_ms` - Maximum wait (0 = immediate)
/// * `timeout_elapsed` - Whether timeout has elapsed
///
/// # Returns
///
/// The reason for flushing.
#[inline]
#[allow(dead_code)]
pub fn determine_flush_reason_exec(
    pending_len: u32,
    max_entries: u32,
    current_bytes: u64,
    max_bytes: u64,
    max_wait_ms: u64,
    _timeout_elapsed: bool,
) -> FlushReason {
    if pending_len >= max_entries {
        FlushReason::EntriesFull
    } else if current_bytes >= max_bytes {
        FlushReason::BytesFull
    } else if max_wait_ms == 0 {
        FlushReason::Immediate
    } else {
        FlushReason::Timeout
    }
}

/// Check if batch contiguity holds.
///
/// # Arguments
///
/// * `min_sequence` - Minimum sequence in batch
/// * `max_sequence` - Maximum sequence in batch
/// * `batch_len` - Number of operations
///
/// # Returns
///
/// `true` if sequences are contiguous.
#[inline]
#[allow(dead_code)]
pub fn is_batch_contiguous(min_sequence: u64, max_sequence: u64, batch_len: u64) -> bool {
    max_sequence >= min_sequence && (max_sequence - min_sequence).saturating_add(1) == batch_len
}

/// Compute time since batch started.
///
/// # Arguments
///
/// * `batch_start_ms` - When batch started
/// * `current_time_ms` - Current time
///
/// # Returns
///
/// Time elapsed since batch start (saturating at 0 if time went backwards).
#[inline]
#[allow(dead_code)]
pub const fn compute_batch_age(batch_start_ms: u64, current_time_ms: u64) -> u64 {
    current_time_ms.saturating_sub(batch_start_ms)
}

/// Compute time until flush is required.
///
/// # Arguments
///
/// * `batch_start_ms` - When batch started
/// * `current_time_ms` - Current time
/// * `max_wait_ms` - Maximum wait time
///
/// # Returns
///
/// Time until flush required (0 if already elapsed).
#[inline]
#[allow(dead_code)]
pub fn compute_time_until_flush(batch_start_ms: u64, current_time_ms: u64, max_wait_ms: u64) -> u64 {
    let elapsed = current_time_ms.saturating_sub(batch_start_ms);
    max_wait_ms.saturating_sub(elapsed)
}

// ============================================================================
// Batch State Checks
// ============================================================================

/// Check if a batch has space for another operation.
///
/// # Arguments
///
/// * `pending_count` - Number of operations in the batch
/// * `current_bytes` - Total bytes in the batch
/// * `op_bytes` - Size of the new operation to add
/// * `max_entries` - Maximum allowed operations per batch
/// * `max_bytes` - Maximum allowed bytes per batch
///
/// # Returns
///
/// `true` if there is space for the operation.
#[inline]
#[allow(dead_code)]
pub fn has_space_exec(pending_count: u32, current_bytes: u64, op_bytes: u64, max_entries: u32, max_bytes: u64) -> bool {
    pending_count < max_entries && op_bytes <= max_bytes.saturating_sub(current_bytes)
}

/// Check if a batch is empty.
///
/// # Arguments
///
/// * `pending_count` - Number of operations in the batch
///
/// # Returns
///
/// `true` if the batch is empty.
#[inline]
#[allow(dead_code)]
pub const fn is_batch_empty(pending_count: u32) -> bool {
    pending_count == 0
}

/// Check if a batch is full (entry count or byte limit reached).
///
/// # Arguments
///
/// * `pending_count` - Number of operations in the batch
/// * `current_bytes` - Total bytes in the batch
/// * `max_entries` - Maximum allowed operations per batch
/// * `max_bytes` - Maximum allowed bytes per batch
///
/// # Returns
///
/// `true` if the batch is full.
#[inline]
#[allow(dead_code)]
pub const fn is_batch_full_exec(pending_count: u32, current_bytes: u64, max_entries: u32, max_bytes: u64) -> bool {
    pending_count >= max_entries || current_bytes >= max_bytes
}

/// Check if a batch should be flushed.
///
/// # Arguments
///
/// * `pending_count` - Number of operations in the batch
/// * `current_bytes` - Total bytes in the batch
/// * `batch_start_ms` - When the first item was added
/// * `current_time_ms` - Current time
/// * `max_entries` - Maximum allowed operations per batch
/// * `max_bytes` - Maximum allowed bytes per batch
/// * `max_wait_ms` - Maximum time to wait before flushing
///
/// # Returns
///
/// `true` if the batch should be flushed.
#[inline]
#[allow(dead_code)]
pub fn should_flush_exec(
    pending_count: u32,
    current_bytes: u64,
    batch_start_ms: u64,
    current_time_ms: u64,
    max_entries: u32,
    max_bytes: u64,
    max_wait_ms: u64,
) -> bool {
    pending_count > 0
        && (is_batch_full_exec(pending_count, current_bytes, max_entries, max_bytes)
            || timeout_elapsed_exec(pending_count, batch_start_ms, current_time_ms, max_wait_ms)
            || max_wait_ms == 0)
}

/// Check if timeout has elapsed for a batch.
///
/// # Arguments
///
/// * `pending_count` - Number of operations in the batch
/// * `batch_start_ms` - When the first item was added (0 if empty)
/// * `current_time_ms` - Current time
/// * `max_wait_ms` - Maximum time to wait before flushing
///
/// # Returns
///
/// `true` if timeout has elapsed and flush should occur.
#[inline]
#[allow(dead_code)]
pub fn timeout_elapsed_exec(pending_count: u32, batch_start_ms: u64, current_time_ms: u64, max_wait_ms: u64) -> bool {
    pending_count > 0 && batch_start_ms > 0 && current_time_ms.saturating_sub(batch_start_ms) >= max_wait_ms
}

/// Check if either limit would be exceeded (convenience wrapper).
///
/// # Arguments
///
/// * `check` - The batch limit check result
///
/// # Returns
///
/// `true` if either limit would be exceeded.
#[inline]
#[allow(dead_code)]
pub const fn would_exceed(check: &BatchLimitCheck) -> bool {
    check.exceeds_entries || check.exceeds_bytes
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
        let check = check_batch_limits(100, 0, 0, 100, u64::MAX);
        assert!(check.exceeds_entries);
        assert!(!check.exceeds_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_entry_limit_not_exceeded() {
        let check = check_batch_limits(99, 0, 0, 100, u64::MAX);
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
        let check = check_batch_limits(1, u64::MAX - 1, u64::MAX, 100, u64::MAX);
        // This should saturate to u64::MAX, not wrap around
        assert!(!check.exceeds_bytes); // u64::MAX <= u64::MAX
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
