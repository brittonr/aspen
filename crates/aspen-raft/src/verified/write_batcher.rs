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

/// Batch state needed by the write batcher pure helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchState {
    /// Number of pending operations in the batch.
    pub pending_count: u32,
    /// Current total bytes in the batch.
    pub current_bytes: u64,
    /// Monotonic start time for the current batch, or 0 when not set.
    pub batch_start_ms: u64,
    /// Current monotonic time for timeout-related helpers, or 0 when unused.
    pub current_time_ms: u64,
}

/// Write batcher limits used by the pure helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchLimits {
    /// Maximum operations allowed per batch.
    pub max_entries: u32,
    /// Maximum bytes allowed per batch.
    pub max_bytes: u64,
    /// Maximum time to wait before flushing.
    pub max_wait_ms: u64,
}

/// Sequence-range state used by contiguity checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchSequenceState {
    /// Minimum sequence in the batch.
    pub min_sequence: u64,
    /// Maximum sequence in the batch.
    pub max_sequence: u64,
    /// Number of operations in the batch.
    pub batch_len: u64,
}

/// Result of checking whether a batch operation would exceed limits.
///
/// Used to determine if the current batch should be flushed before
/// adding a new operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchLimitCheck {
    /// Whether adding would exceed entry count limit.
    pub would_reject_for_entries: bool,
    /// Whether adding would exceed byte size limit.
    pub would_reject_for_bytes: bool,
}

impl BatchLimitCheck {
    /// Returns true if either limit would be exceeded.
    #[inline]
    pub fn would_exceed(&self) -> bool {
        self.would_reject_for_entries || self.would_reject_for_bytes
    }
}

/// Size inputs for Set operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetOperationSizeInput {
    /// Key length in bytes.
    pub key_len_bytes: u64,
    /// Value length in bytes.
    pub value_len_bytes: u64,
}

/// Operation-kind flags for batchability checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchableOperationFlags {
    /// Whether the operation is a Set.
    pub is_set: bool,
    /// Whether the operation is a Delete.
    pub is_delete: bool,
}

/// Check if adding an operation would exceed batch limits.
///
/// This is the core decision function for batch flushing. It determines
/// whether the current batch should be flushed before adding a new
/// operation based on entry count and byte size limits.
///
/// # Arguments
///
/// * `batch` - Current batch state
/// * `op_bytes` - Size of the new operation to add in bytes
/// * `limits` - Batch entry/byte/time limits
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
/// use aspen_raft::verified::BatchLimits;
/// use aspen_raft::verified::BatchState;
///
/// let check = check_batch_limits(
///     BatchState {
///         pending_count: 99,
///         current_bytes: 500_000,
///         batch_start_ms: 0,
///         current_time_ms: 0,
///     },
///     1000,
///     BatchLimits {
///         max_entries: 100,
///         max_bytes: 1_000_000,
///         max_wait_ms: 0,
///     },
/// );
///
/// assert!(!check.would_reject_for_entries); // 99 < 100
/// assert!(!check.would_reject_for_bytes);   // 501_000 < 1_000_000
/// assert!(!check.would_exceed());
/// ```
#[inline]
pub fn check_batch_limits(batch: BatchState, op_bytes: u64, limits: BatchLimits) -> BatchLimitCheck {
    // Only check limits if batch is non-empty
    // (we always allow adding to an empty batch)
    let has_pending = batch.pending_count > 0;

    let should_reject_for_entries = has_pending && batch.pending_count >= limits.max_entries;
    let should_reject_for_bytes = has_pending && batch.current_bytes.saturating_add(op_bytes) > limits.max_bytes;

    BatchLimitCheck {
        would_reject_for_entries: should_reject_for_entries,
        would_reject_for_bytes: should_reject_for_bytes,
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
/// * `batch` - Batch state after adding the operation
/// * `limits` - Batch entry/byte/time limits
/// * `is_flush_already_scheduled` - Whether a delayed flush is already pending
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
/// use aspen_raft::verified::BatchLimits;
/// use aspen_raft::verified::BatchState;
/// use aspen_raft::verified::FlushDecision;
/// use aspen_raft::verified::determine_flush_action;
///
/// // Batch is full, flush immediately
/// let decision = determine_flush_action(
///     BatchState {
///         pending_count: 100,
///         current_bytes: 500_000,
///         batch_start_ms: 0,
///         current_time_ms: 0,
///     },
///     BatchLimits {
///         max_entries: 100,
///         max_bytes: 1_000_000,
///         max_wait_ms: 1,
///     },
///     false,
/// );
/// assert_eq!(decision, FlushDecision::Immediate);
///
/// // Batch has items, not full, no flush scheduled yet
/// let decision = determine_flush_action(
///     BatchState {
///         pending_count: 10,
///         current_bytes: 1000,
///         batch_start_ms: 0,
///         current_time_ms: 0,
///     },
///     BatchLimits {
///         max_entries: 100,
///         max_bytes: 1_000_000,
///         max_wait_ms: 1,
///     },
///     false,
/// );
/// assert_eq!(decision, FlushDecision::Delayed);
///
/// // Batching disabled, always flush immediately
/// let decision = determine_flush_action(
///     BatchState {
///         pending_count: 1,
///         current_bytes: 100,
///         batch_start_ms: 0,
///         current_time_ms: 0,
///     },
///     BatchLimits {
///         max_entries: 100,
///         max_bytes: 1_000_000,
///         max_wait_ms: 0,
///     },
///     false,
/// );
/// assert_eq!(decision, FlushDecision::Immediate);
/// ```
#[inline]
pub fn determine_flush_action(
    batch: BatchState,
    limits: BatchLimits,
    is_flush_already_scheduled: bool,
) -> FlushDecision {
    // Immediate flush if batch is full (entries)
    if batch.pending_count >= limits.max_entries {
        return FlushDecision::Immediate;
    }

    // Immediate flush if batch is full (bytes)
    if batch.current_bytes >= limits.max_bytes {
        return FlushDecision::Immediate;
    }

    // No pending items means no flush needed
    if batch.pending_count == 0 {
        return FlushDecision::None;
    }

    // Batching disabled (max_wait = 0) means immediate flush
    if limits.max_wait_ms == 0 {
        return FlushDecision::Immediate;
    }

    // Schedule delayed flush if not already scheduled
    if !is_flush_already_scheduled {
        return FlushDecision::Delayed;
    }

    FlushDecision::None
}

/// Calculate the size of a Set operation in bytes.
///
/// # Arguments
///
/// * `size` - Key/value byte lengths for the Set operation
///
/// # Returns
///
/// Total size in bytes (key_len + value_len).
#[inline]
pub const fn calculate_set_op_size(size: SetOperationSizeInput) -> u64 {
    size.key_len_bytes.saturating_add(size.value_len_bytes)
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
pub const fn compute_op_size(size: SetOperationSizeInput) -> u64 {
    size.key_len_bytes.saturating_add(size.value_len_bytes)
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
pub const fn does_op_fit(op_bytes: u64, limits: BatchLimits) -> bool {
    op_bytes <= limits.max_bytes
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
/// * `batch` - Current batch state
/// * `op_bytes` - Size of operation to add
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if add would trigger flush.
///
/// # Safety
///
/// Requires `batch.current_bytes <= limits.max_bytes` (from bytes_bounded invariant).
#[inline]
#[allow(dead_code)]
pub fn would_add_trigger_flush(batch: BatchState, op_bytes: u64, limits: BatchLimits) -> bool {
    batch.pending_count >= limits.max_entries || op_bytes > limits.max_bytes.saturating_sub(batch.current_bytes)
}

/// Compute new batch start time.
///
/// First write sets batch start to current time,
/// subsequent writes preserve existing start.
///
/// # Arguments
///
/// * `batch` - Current batch state
///
/// # Returns
///
/// New batch start time.
#[inline]
#[allow(dead_code)]
pub const fn compute_batch_start(batch: BatchState) -> u64 {
    if batch.pending_count == 0 {
        batch.current_time_ms
    } else {
        batch.batch_start_ms
    }
}

/// Compute new current bytes after add.
///
/// # Arguments
///
/// * `batch` - Current batch state
/// * `op_bytes` - Size of operation to add
///
/// # Returns
///
/// New current bytes (saturating at u64::MAX).
#[inline]
#[allow(dead_code)]
pub const fn compute_bytes_after_add(batch: BatchState, op_bytes: u64) -> u64 {
    batch.current_bytes.saturating_add(op_bytes)
}

/// Check if operation is batchable.
///
/// # Arguments
///
/// * `operation` - Operation-kind flags
///
/// # Returns
///
/// `true` if operation is batchable.
#[inline]
#[allow(dead_code)]
pub const fn is_batchable_operation(operation: BatchableOperationFlags) -> bool {
    operation.is_set || operation.is_delete
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
/// * `batch` - Current batch state
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if at or above entries limit.
#[inline]
#[allow(dead_code)]
pub const fn is_entries_full(batch: BatchState, limits: BatchLimits) -> bool {
    batch.pending_count >= limits.max_entries
}

/// Check if batch is at bytes limit.
///
/// # Arguments
///
/// * `batch` - Current batch state
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if at or above bytes limit.
#[inline]
#[allow(dead_code)]
pub const fn is_bytes_full(batch: BatchState, limits: BatchLimits) -> bool {
    batch.current_bytes >= limits.max_bytes
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
/// * `batch` - Timed batch state
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if timeout has elapsed.
#[inline]
#[allow(dead_code)]
pub fn is_timeout_elapsed(batch: BatchState, limits: BatchLimits) -> bool {
    batch.current_time_ms.saturating_sub(batch.batch_start_ms) >= limits.max_wait_ms
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
/// * `batch` - Current batch state
/// * `limits` - Batch entry/byte/time limits
/// * `_has_timeout_elapsed` - Whether timeout has elapsed
///
/// # Returns
///
/// The reason for flushing.
#[inline]
#[allow(dead_code)]
pub fn determine_flush_reason_exec(batch: BatchState, limits: BatchLimits, _has_timeout_elapsed: bool) -> FlushReason {
    if batch.pending_count >= limits.max_entries {
        FlushReason::EntriesFull
    } else if batch.current_bytes >= limits.max_bytes {
        FlushReason::BytesFull
    } else if limits.max_wait_ms == 0 {
        FlushReason::Immediate
    } else {
        FlushReason::Timeout
    }
}

/// Check if batch contiguity holds.
///
/// # Arguments
///
/// * `sequence` - Sequence range state for the batch
///
/// # Returns
///
/// `true` if sequences are contiguous.
#[inline]
#[allow(dead_code)]
pub fn is_batch_contiguous(sequence: BatchSequenceState) -> bool {
    sequence.max_sequence >= sequence.min_sequence
        && sequence.max_sequence.saturating_sub(sequence.min_sequence).saturating_add(1) == sequence.batch_len
}

/// Compute time since batch started.
///
/// # Arguments
///
/// * `batch` - Timed batch state
///
/// # Returns
///
/// Time elapsed since batch start (saturating at 0 if time went backwards).
#[inline]
#[allow(dead_code)]
pub const fn compute_batch_age(batch: BatchState) -> u64 {
    batch.current_time_ms.saturating_sub(batch.batch_start_ms)
}

/// Compute time until flush is required.
///
/// # Arguments
///
/// * `batch` - Timed batch state
/// * `max_wait_ms` - Maximum wait time
///
/// # Returns
///
/// Time until flush required (0 if already elapsed).
#[inline]
#[allow(dead_code)]
pub fn compute_time_until_flush(batch: BatchState, max_wait_ms: u64) -> u64 {
    let elapsed = batch.current_time_ms.saturating_sub(batch.batch_start_ms);
    max_wait_ms.saturating_sub(elapsed)
}

// ============================================================================
// Batch State Checks
// ============================================================================

/// Check if a batch has space for another operation.
///
/// # Arguments
///
/// * `batch` - Current batch state
/// * `op_bytes` - Size of the new operation to add
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if there is space for the operation.
#[inline]
#[allow(dead_code)]
pub fn has_space_exec(batch: BatchState, op_bytes: u64, limits: BatchLimits) -> bool {
    batch.pending_count < limits.max_entries && op_bytes <= limits.max_bytes.saturating_sub(batch.current_bytes)
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
/// * `batch` - Current batch state
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if the batch is full.
#[inline]
#[allow(dead_code)]
pub const fn is_batch_full_exec(batch: BatchState, limits: BatchLimits) -> bool {
    batch.pending_count >= limits.max_entries || batch.current_bytes >= limits.max_bytes
}

/// Check if a batch should be flushed.
///
/// # Arguments
///
/// * `batch` - Timed batch state
/// * `limits` - Batch entry/byte/time limits
///
/// # Returns
///
/// `true` if the batch should be flushed.
#[inline]
#[allow(dead_code)]
pub fn should_flush_exec(batch: BatchState, limits: BatchLimits) -> bool {
    batch.pending_count > 0
        && (is_batch_full_exec(batch, limits)
            || timeout_elapsed_exec(batch, limits.max_wait_ms)
            || limits.max_wait_ms == 0)
}

/// Check if timeout has elapsed for a batch.
///
/// # Arguments
///
/// * `batch` - Timed batch state
/// * `max_wait_ms` - Maximum time to wait before flushing
///
/// # Returns
///
/// `true` if timeout has elapsed and flush should occur.
#[inline]
#[allow(dead_code)]
pub fn timeout_elapsed_exec(batch: BatchState, max_wait_ms: u64) -> bool {
    batch.pending_count > 0
        && batch.batch_start_ms > 0
        && batch.current_time_ms.saturating_sub(batch.batch_start_ms) >= max_wait_ms
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
    check.would_reject_for_entries || check.would_reject_for_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch_state(pending_count: u32, current_bytes: u64) -> BatchState {
        BatchState {
            pending_count,
            current_bytes,
            batch_start_ms: 0,
            current_time_ms: 0,
        }
    }

    // ========================================================================
    // check_batch_limits tests
    // ========================================================================

    #[test]
    fn test_empty_batch_allows_any_operation() {
        // Empty batch should never report exceeded limits
        let check = check_batch_limits(batch_state(0, 0), 1_000_000, BatchLimits {
            max_entries: 1,
            max_bytes: 1,
            max_wait_ms: 0,
        });
        assert!(!check.would_reject_for_entries);
        assert!(!check.would_reject_for_bytes);
        assert!(!check.would_exceed());
    }

    #[test]
    fn test_entry_limit_exceeded() {
        let check = check_batch_limits(batch_state(100, 0), 0, BatchLimits {
            max_entries: 100,
            max_bytes: u64::MAX,
            max_wait_ms: 0,
        });
        assert!(check.would_reject_for_entries);
        assert!(!check.would_reject_for_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_entry_limit_not_exceeded() {
        let check = check_batch_limits(batch_state(99, 0), 0, BatchLimits {
            max_entries: 100,
            max_bytes: u64::MAX,
            max_wait_ms: 0,
        });
        assert!(!check.would_reject_for_entries);
        assert!(!check.would_exceed());
    }

    #[test]
    fn test_byte_limit_exceeded() {
        let check = check_batch_limits(batch_state(1, 900_000), 200_000, BatchLimits {
            max_entries: 100,
            max_bytes: 1_000_000,
            max_wait_ms: 0,
        });
        assert!(!check.would_reject_for_entries);
        assert!(check.would_reject_for_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_byte_limit_not_exceeded() {
        let check = check_batch_limits(batch_state(1, 500_000), 499_999, BatchLimits {
            max_entries: 100,
            max_bytes: 1_000_000,
            max_wait_ms: 0,
        });
        assert!(!check.would_reject_for_bytes);
        assert!(!check.would_exceed());
    }

    #[test]
    fn test_byte_limit_exact_boundary() {
        // Exactly at the limit is OK (uses > not >=)
        let check = check_batch_limits(batch_state(1, 500_000), 500_000, BatchLimits {
            max_entries: 100,
            max_bytes: 1_000_000,
            max_wait_ms: 0,
        });
        assert!(!check.would_reject_for_bytes);
    }

    #[test]
    fn test_byte_limit_one_over() {
        let check = check_batch_limits(batch_state(1, 500_000), 500_001, BatchLimits {
            max_entries: 100,
            max_bytes: 1_000_000,
            max_wait_ms: 0,
        });
        assert!(check.would_reject_for_bytes);
    }

    #[test]
    fn test_both_limits_exceeded() {
        let check = check_batch_limits(batch_state(100, 1_000_000), 1, BatchLimits {
            max_entries: 100,
            max_bytes: 1_000_000,
            max_wait_ms: 0,
        });
        assert!(check.would_reject_for_entries);
        assert!(check.would_reject_for_bytes);
        assert!(check.would_exceed());
    }

    #[test]
    fn test_saturating_arithmetic_prevents_overflow() {
        // Should not overflow when adding huge values
        let check = check_batch_limits(batch_state(1, u64::MAX.saturating_sub(1)), u64::MAX, BatchLimits {
            max_entries: 100,
            max_bytes: u64::MAX,
            max_wait_ms: 0,
        });
        // This should saturate to u64::MAX, not wrap around
        assert!(!check.would_reject_for_bytes); // u64::MAX <= u64::MAX
    }

    // ========================================================================
    // determine_flush_action tests
    // ========================================================================

    #[test]
    fn test_flush_when_entries_full() {
        let decision = determine_flush_action(
            batch_state(100, 0),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 1,
            },
            false,
        );
        assert_eq!(decision, FlushDecision::Immediate);
    }

    #[test]
    fn test_flush_when_bytes_full() {
        let decision = determine_flush_action(
            batch_state(1, 1_000_000),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 1,
            },
            false,
        );
        assert_eq!(decision, FlushDecision::Immediate);
    }

    #[test]
    fn test_no_flush_when_empty() {
        let decision = determine_flush_action(
            batch_state(0, 0),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 1,
            },
            false,
        );
        assert_eq!(decision, FlushDecision::None);
    }

    #[test]
    fn test_immediate_flush_when_batching_disabled() {
        let decision = determine_flush_action(
            batch_state(1, 100),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 0,
            },
            false,
        );
        assert_eq!(decision, FlushDecision::Immediate);
    }

    #[test]
    fn test_delayed_flush_when_not_scheduled() {
        let decision = determine_flush_action(
            batch_state(1, 100),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 1,
            },
            false,
        );
        assert_eq!(decision, FlushDecision::Delayed);
    }

    #[test]
    fn test_no_action_when_flush_already_scheduled() {
        let decision = determine_flush_action(
            batch_state(1, 100),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 1,
            },
            true,
        );
        assert_eq!(decision, FlushDecision::None);
    }

    #[test]
    fn test_immediate_overrides_scheduled() {
        // Even if flush is scheduled, immediate takes precedence when full
        let decision = determine_flush_action(
            batch_state(100, 0),
            BatchLimits {
                max_entries: 100,
                max_bytes: 1_000_000,
                max_wait_ms: 1,
            },
            true,
        );
        assert_eq!(decision, FlushDecision::Immediate);
    }

    // ========================================================================
    // Size calculation tests
    // ========================================================================

    #[test]
    fn test_set_op_size() {
        assert_eq!(
            calculate_set_op_size(SetOperationSizeInput {
                key_len_bytes: 8,
                value_len_bytes: 10,
            }),
            18,
        );
        assert_eq!(
            calculate_set_op_size(SetOperationSizeInput {
                key_len_bytes: 0,
                value_len_bytes: 0,
            }),
            0,
        );
        assert_eq!(
            calculate_set_op_size(SetOperationSizeInput {
                key_len_bytes: 1000,
                value_len_bytes: 1_000_000,
            }),
            1_001_000,
        );
    }

    #[test]
    fn test_delete_op_size() {
        assert_eq!(calculate_delete_op_size(8), 8);
        assert_eq!(calculate_delete_op_size(0), 0);
    }
}
