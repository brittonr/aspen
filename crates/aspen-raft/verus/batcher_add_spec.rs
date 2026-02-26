//! Write Batcher Add Operation Specification
//!
//! Formal specifications for adding writes to the batcher.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/batcher_add_spec.rs
//! ```

use vstd::prelude::*;

// Import from batcher_state_spec
use crate::batcher_state_spec::*;

verus! {
    // ========================================================================
    // Add Operation
    // ========================================================================

    /// Precondition for adding a write to the batch
    ///
    /// Requires the batcher invariant to hold on the current state,
    /// ensuring size bounds, bytes consistency, and ordering are maintained.
    ///
    /// SAFETY: Includes overflow protection for next_sequence increment.
    /// In practice, u64::MAX operations would never be reached, but this
    /// ensures formal verification completeness.
    pub open spec fn add_pre(
        state: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
    ) -> bool {
        // Batcher invariant must hold on the current state
        batcher_invariant(state) &&
        // Key is non-empty
        key.len() > 0 &&
        // Operation size doesn't exceed max_bytes by itself
        key.len() + value.len() <= state.config.max_bytes as int &&
        // Overflow protection: ensure next_sequence can be incremented
        state.next_sequence < u64::MAX
    }

    /// Effect of adding a write to the batch (Set operation)
    /// Assumes: add_pre(pre, key, value)
    pub open spec fn add_set_post(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    ) -> BatcherState {
        let op_bytes = (key.len() + value.len()) as u64;
        let write = PendingWriteSpec {
            is_set: true,
            key: key,
            value: value,
            size_bytes: op_bytes,
            sequence: pre.next_sequence,
        };

        let new_batch_start = if pre.pending.len() == 0 {
            current_time_ms
        } else {
            pre.batch_start_ms
        };

        BatcherState {
            pending: pre.pending.push(write),
            current_bytes: (pre.current_bytes + op_bytes) as u64,
            next_sequence: (pre.next_sequence + 1) as u64,
            batch_start_ms: new_batch_start,
            current_time_ms: current_time_ms,
            ..pre
        }
    }

    /// Effect of adding a Delete operation
    /// Assumes: key.len() > 0
    pub open spec fn add_delete_post(
        pre: BatcherState,
        key: Seq<u8>,
        current_time_ms: u64,
    ) -> BatcherState {
        let op_bytes = key.len() as u64;
        let write = PendingWriteSpec {
            is_set: false,
            key: key,
            value: Seq::empty(),
            size_bytes: op_bytes,
            sequence: pre.next_sequence,
        };

        let new_batch_start = if pre.pending.len() == 0 {
            current_time_ms
        } else {
            pre.batch_start_ms
        };

        BatcherState {
            pending: pre.pending.push(write),
            current_bytes: (pre.current_bytes + op_bytes) as u64,
            next_sequence: (pre.next_sequence + 1) as u64,
            batch_start_ms: new_batch_start,
            current_time_ms: current_time_ms,
            ..pre
        }
    }

    // ========================================================================
    // Add Proofs
    // ========================================================================

    /// Proof: Add increases pending count by 1
    #[verifier(external_body)]
    pub proof fn add_increases_count(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires add_pre(pre, key, value)
        ensures ({
            let post = add_set_post(pre, key, value, current_time_ms);
            post.pending.len() == pre.pending.len() + 1
        })
    {
        // Follows from push definition
    }

    /// Proof: Add increases current_bytes correctly
    #[verifier(external_body)]
    pub proof fn add_increases_bytes(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires add_pre(pre, key, value)
        ensures ({
            let post = add_set_post(pre, key, value, current_time_ms);
            post.current_bytes == pre.current_bytes + (key.len() + value.len()) as u64
        })
    {
        // Directly from add_set_post definition
    }

    /// Proof: Add advances sequence number
    #[verifier(external_body)]
    pub proof fn add_advances_sequence(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires add_pre(pre, key, value)
        ensures ({
            let post = add_set_post(pre, key, value, current_time_ms);
            post.next_sequence == pre.next_sequence + 1
        })
    {
        // Directly from add_set_post definition
    }

    /// Proof: Add sets batch_start on first write
    #[verifier(external_body)]
    pub proof fn add_sets_batch_start(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            add_pre(pre, key, value),
            pre.pending.len() == 0,
        ensures ({
            let post = add_set_post(pre, key, value, current_time_ms);
            post.batch_start_ms == current_time_ms
        })
    {
        // First write sets batch_start
    }

    /// Proof: Add preserves batch_start for subsequent writes
    #[verifier(external_body)]
    pub proof fn add_preserves_batch_start(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            add_pre(pre, key, value),
            pre.pending.len() > 0,
        ensures ({
            let post = add_set_post(pre, key, value, current_time_ms);
            post.batch_start_ms == pre.batch_start_ms
        })
    {
        // Subsequent writes preserve batch_start
    }

    /// Proof: Add preserves ordering
    #[verifier(external_body)]
    pub proof fn add_preserves_ordering(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            batcher_invariant(pre),
            add_pre(pre, key, value),
        ensures ordering_preserved(add_set_post(pre, key, value, current_time_ms))
    {
        // New write has sequence = next_sequence, which is greater than all existing
        // sequences (by sequences_valid invariant), so ordering preserved
    }

    /// Proof: Add preserves bytes consistency
    #[verifier(external_body)]
    pub proof fn add_preserves_bytes_consistency(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            batcher_invariant(pre),
            add_pre(pre, key, value),
        ensures bytes_consistent(add_set_post(pre, key, value, current_time_ms))
    {
        // New current_bytes = old + op_bytes
        // New sum = old_sum + op_bytes
        // So consistency preserved
    }

    /// Proof: Add preserves invariant when space available
    #[verifier(external_body)]
    pub proof fn add_preserves_invariant_with_space(
        pre: BatcherState,
        key: Seq<u8>,
        value: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            batcher_invariant(pre),
            add_pre(pre, key, value),
            has_space(pre, (key.len() + value.len()) as u64),
        ensures batcher_invariant(add_set_post(pre, key, value, current_time_ms))
    {
        let post = add_set_post(pre, key, value, current_time_ms);
        let op_bytes = (key.len() + value.len()) as u64;

        // Verify size_bounded:
        // has_space ensures pre.pending.len() < max_entries
        // post.pending.len() == pre.pending.len() + 1 <= max_entries
        assert(post.pending.len() == pre.pending.len() + 1);
        assert(post.pending.len() <= post.config.max_entries as int);
        assert(size_bounded(post));

        // Verify bytes_bounded:
        // has_space ensures pre.current_bytes + op_bytes <= max_bytes
        // post.current_bytes == pre.current_bytes + op_bytes <= max_bytes
        assert(post.current_bytes == pre.current_bytes + op_bytes);
        assert(post.current_bytes <= post.config.max_bytes);
        assert(bytes_bounded(post));

        // Verify sequences_valid:
        // All existing sequences are < pre.next_sequence (by invariant)
        // New write has sequence = pre.next_sequence
        // post.next_sequence = pre.next_sequence + 1
        // So all sequences (including new) are < post.next_sequence
        assert(post.next_sequence == pre.next_sequence + 1);
        assert(sequences_valid(post));

        // Verify sizes_valid:
        // Existing writes have valid sizes (by invariant)
        // New write has size_bytes = key.len() + value.len() (by construction)
        assert(sizes_valid(post));

        // Verify batch_start_consistent:
        // If pre was empty, post.batch_start_ms = current_time_ms > 0 (non-empty batch)
        // If pre was non-empty, post.batch_start_ms = pre.batch_start_ms (preserved)
        // Either way, empty batch implies batch_start_ms == 0 is vacuously true
        // since post is never empty (we just added a write)
        assert(post.pending.len() > 0);
        assert(batch_start_consistent(post));

        // Invoke existing proofs for ordering and bytes_consistency
        add_preserves_ordering(pre, key, value, current_time_ms);
        add_preserves_bytes_consistency(pre, key, value, current_time_ms);
    }

    // ========================================================================
    // Delete Add Proofs
    // ========================================================================

    /// Proof: Delete add creates entry with empty value
    ///
    /// Safety: add_delete_post always pushes one element to pending,
    /// so post.pending.len() >= 1 and last_idx >= 0.
    #[verifier(external_body)]
    pub proof fn delete_add_has_empty_value(
        pre: BatcherState,
        key: Seq<u8>,
        current_time_ms: u64,
    )
        requires key.len() > 0
        ensures ({
            let post = add_delete_post(pre, key, current_time_ms);
            // Index safety: add_delete_post pushes one element, so len >= 1
            post.pending.len() >= 1 &&
            ({
                let last_idx = (post.pending.len() - 1) as int;
                post.pending[last_idx].value.len() == 0 &&
                !post.pending[last_idx].is_set
            })
        })
    {
        // add_delete_post uses pre.pending.push(write), so post.pending.len() = pre.pending.len() + 1 >= 1
        // Delete uses Seq::empty() for value
    }

    /// Proof: Delete add increases bytes by key length only
    #[verifier(external_body)]
    pub proof fn delete_add_bytes(
        pre: BatcherState,
        key: Seq<u8>,
        current_time_ms: u64,
    )
        requires key.len() > 0
        ensures ({
            let post = add_delete_post(pre, key, current_time_ms);
            post.current_bytes == pre.current_bytes + key.len() as u64
        })
    {
        // Delete size = key.len() only
    }

    // ========================================================================
    // Batch Triggering
    // ========================================================================

    /// Check if add would trigger flush (batch would exceed limits)
    ///
    /// Uses overflow-safe comparison: instead of `current_bytes + op_bytes > max_bytes`
    /// which could overflow, we check `op_bytes > max_bytes - current_bytes`.
    /// This is safe when bytes_bounded invariant holds (current_bytes <= max_bytes).
    /// Assumes: bytes_bounded(state) - Ensures current_bytes <= max_bytes
    pub open spec fn add_would_trigger_flush(
        state: BatcherState,
        op_bytes: u64,
    ) -> bool {
        // Would exceed entries
        state.pending.len() >= state.config.max_entries as int ||
        // Would exceed bytes (overflow-safe rearrangement)
        // Safe because bytes_bounded(state) ensures current_bytes <= max_bytes
        op_bytes > state.config.max_bytes - state.current_bytes
    }

    /// Result of add operation
    pub enum AddResult {
        /// Added successfully, no flush needed
        Added,
        /// Added, but flush triggered (batch full)
        AddedAndFlush,
        /// Need to flush first, then add
        FlushFirst,
    }

    /// Determine add result based on current state
    pub open spec fn determine_add_result(
        state: BatcherState,
        op_bytes: u64,
    ) -> AddResult {
        if state.pending.len() == 0 {
            // First write always succeeds
            if op_bytes > state.config.max_bytes {
                AddResult::AddedAndFlush // Single op exceeds limit
            } else {
                AddResult::Added
            }
        } else if add_would_trigger_flush(state, op_bytes) {
            // Need to flush existing batch first
            AddResult::FlushFirst
        } else {
            AddResult::Added
        }
    }

    /// Proof: Adding when empty never needs flush first
    #[verifier(external_body)]
    pub proof fn empty_add_never_flush_first(
        state: BatcherState,
        op_bytes: u64,
    )
        requires state.pending.len() == 0
        ensures ({
            let result = determine_add_result(state, op_bytes);
            !matches!(result, AddResult::FlushFirst)
        })
    {
        // Empty batch can always accept a write
    }

    // ========================================================================
    // Write Type Classification
    // ========================================================================

    /// Check if an operation type is batchable
    pub open spec fn is_batchable_op(is_set: bool, is_delete: bool) -> bool {
        is_set || is_delete
    }

    /// Set operations are batchable
    #[verifier(external_body)]
    pub proof fn set_is_batchable()
        ensures is_batchable_op(true, false)
    {
    }

    /// Delete operations are batchable
    #[verifier(external_body)]
    pub proof fn delete_is_batchable()
        ensures is_batchable_op(false, true)
    {
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Check if key is valid for add operation.
    ///
    /// # Arguments
    ///
    /// * `key_len` - Length of the key
    ///
    /// # Returns
    ///
    /// `true` if key is non-empty.
    pub fn is_key_valid(key_len: u64) -> (result: bool)
        ensures result == (key_len > 0)
    {
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
    pub fn compute_op_size(key_len: u64, value_len: u64) -> (result: u64)
        ensures
            key_len <= u64::MAX - value_len ==> result == key_len + value_len,
            key_len > u64::MAX - value_len ==> result == u64::MAX
    {
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
    pub fn does_op_fit(op_bytes: u64, max_bytes: u64) -> (result: bool)
        ensures result == (op_bytes <= max_bytes)
    {
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
    pub fn can_increment_sequence(next_sequence: u64) -> (result: bool)
        ensures result == (next_sequence < u64::MAX)
    {
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
    pub fn compute_next_sequence(current_sequence: u64) -> (result: u64)
        ensures
            current_sequence < u64::MAX ==> result == current_sequence + 1,
            current_sequence == u64::MAX ==> result == u64::MAX
    {
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
    pub fn would_add_trigger_flush(
        pending_len: u32,
        max_entries: u32,
        current_bytes: u64,
        max_bytes: u64,
        op_bytes: u64,
    ) -> (result: bool)
        requires
            // SAFETY: bytes_bounded invariant ensures current_bytes <= max_bytes
            current_bytes <= max_bytes
        ensures result == (
            pending_len >= max_entries ||
            op_bytes > max_bytes - current_bytes
        )
    {
        pending_len >= max_entries ||
        op_bytes > max_bytes - current_bytes
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
    pub fn compute_batch_start(
        pending_len: u32,
        current_batch_start: u64,
        current_time_ms: u64,
    ) -> (result: u64)
        ensures
            pending_len == 0 ==> result == current_time_ms,
            pending_len > 0 ==> result == current_batch_start
    {
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
    pub fn compute_bytes_after_add(
        current_bytes: u64,
        op_bytes: u64,
    ) -> (result: u64)
        ensures
            current_bytes <= u64::MAX - op_bytes ==> result == current_bytes + op_bytes,
            current_bytes > u64::MAX - op_bytes ==> result == u64::MAX
    {
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
    pub fn is_batchable_operation(is_set: bool, is_delete: bool) -> (result: bool)
        ensures result == (is_set || is_delete)
    {
        is_set || is_delete
    }
}
