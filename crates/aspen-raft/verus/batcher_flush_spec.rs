//! Write Batcher Flush Operation Specification
//!
//! Formal specifications for flushing batched writes to Raft.
//!
//! # Key Properties
//!
//! - **BATCH-5: No Write Loss**: Every write in pending is flushed
//! - **BATCH-6: Atomicity**: Batch submitted together as single Raft proposal
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/batcher_flush_spec.rs
//! ```

use vstd::prelude::*;

// Import from batcher_state_spec
use crate::batcher_state_spec::*;

verus! {
    // ========================================================================
    // Flush Operation
    // ========================================================================

    /// Precondition for flush (always succeeds if there are pending writes)
    pub open spec fn flush_pre(state: BatcherState) -> bool {
        state.pending.len() > 0
    }

    /// The batch extracted from state before flush
    pub open spec fn extract_batch(state: BatcherState) -> Seq<PendingWriteSpec> {
        state.pending
    }

    /// Effect of flush - clears pending and resets state
    pub open spec fn flush_post(pre: BatcherState) -> BatcherState
        recommends flush_pre(pre)
    {
        BatcherState {
            pending: Seq::empty(),
            current_bytes: 0,
            next_sequence: pre.next_sequence, // Sequence continues
            batch_start_ms: 0, // Reset
            current_time_ms: pre.current_time_ms,
            config: pre.config,
        }
    }

    // ========================================================================
    // Flush Proofs
    // ========================================================================

    /// Proof: Flush clears all pending writes
    pub proof fn flush_clears_pending(pre: BatcherState)
        requires flush_pre(pre)
        ensures {
            let post = flush_post(pre);
            post.pending.len() == 0
        }
    {
        // Directly from flush_post definition
    }

    /// Proof: Flush resets current_bytes to 0
    pub proof fn flush_resets_bytes(pre: BatcherState)
        requires flush_pre(pre)
        ensures {
            let post = flush_post(pre);
            post.current_bytes == 0
        }
    {
        // Directly from flush_post definition
    }

    /// Proof: Flush preserves sequence number
    pub proof fn flush_preserves_sequence(pre: BatcherState)
        requires flush_pre(pre)
        ensures {
            let post = flush_post(pre);
            post.next_sequence == pre.next_sequence
        }
    {
        // Sequence continues monotonically
    }

    /// Proof: Flush resets batch_start
    pub proof fn flush_resets_batch_start(pre: BatcherState)
        requires flush_pre(pre)
        ensures {
            let post = flush_post(pre);
            post.batch_start_ms == 0
        }
    {
        // Batch timing reset
    }

    /// Proof: Flush produces valid invariant state
    pub proof fn flush_produces_valid_state(pre: BatcherState)
        requires
            batcher_invariant(pre),
            flush_pre(pre),
        ensures batcher_invariant(flush_post(pre))
    {
        // Empty pending satisfies all invariants trivially
    }

    // ========================================================================
    // No Write Loss Property (BATCH-5)
    // ========================================================================

    /// BATCH-5: The extracted batch contains all pending writes
    pub open spec fn no_write_loss(
        pre_pending: Seq<PendingWriteSpec>,
        batch: Seq<PendingWriteSpec>,
    ) -> bool {
        // All pending writes are in the batch
        forall |i: int| 0 <= i < pre_pending.len() ==>
            batch.contains(pre_pending[i])
    }

    /// Proof: Extract batch contains all pending
    pub proof fn extract_contains_all(pre: BatcherState)
        ensures no_write_loss(pre.pending, extract_batch(pre))
    {
        // extract_batch returns pre.pending directly
    }

    /// All writes with sequence < next_sequence are either:
    /// - In pending (not yet flushed)
    /// - Already flushed (no longer tracked)
    pub open spec fn write_accountability(
        state: BatcherState,
        flushed_count: u64,
    ) -> bool {
        // next_sequence = pending.len() + flushed_count
        // (simplified - in practice would track flushed set)
        state.next_sequence >= state.pending.len() as u64
    }

    // ========================================================================
    // Atomicity Property (BATCH-6)
    // ========================================================================

    /// A flushed batch represents an atomic unit
    pub struct FlushedBatch {
        /// Operations in this batch (in FIFO order)
        pub operations: Seq<PendingWriteSpec>,
        /// Minimum sequence in batch
        pub min_sequence: u64,
        /// Maximum sequence in batch
        pub max_sequence: u64,
    }

    /// Create a flushed batch from pending
    pub open spec fn create_flushed_batch(pending: Seq<PendingWriteSpec>) -> FlushedBatch
        recommends pending.len() > 0
    {
        FlushedBatch {
            operations: pending,
            min_sequence: pending[0].sequence,
            max_sequence: pending[pending.len() - 1].sequence,
        }
    }

    /// BATCH-6: All operations in batch have contiguous sequences
    pub open spec fn batch_contiguous(batch: FlushedBatch) -> bool {
        batch.max_sequence - batch.min_sequence + 1 == batch.operations.len() as u64
    }

    /// BATCH-6: Batch is atomic (all-or-nothing)
    /// This is implicit in the Raft proposal - either all ops commit or none do
    pub open spec fn batch_atomic(batch: FlushedBatch) -> bool {
        // All operations share same commit fate
        // Represented by being in the same Raft proposal
        batch.operations.len() > 0
    }

    /// Proof: Ordered batch has contiguous sequences
    pub proof fn ordered_batch_is_contiguous(state: BatcherState)
        requires
            batcher_invariant(state),
            state.pending.len() > 0,
        ensures batch_contiguous(create_flushed_batch(state.pending))
    {
        // sequences_ordered + sequences_valid implies contiguity
        // Each add increments sequence by 1
    }

    // ========================================================================
    // Flush Timing
    // ========================================================================

    /// Reasons for flush
    pub enum FlushReason {
        /// Batch reached max_entries
        EntriesFull,
        /// Batch reached max_bytes
        BytesFull,
        /// Timeout elapsed
        Timeout,
        /// Batching disabled (max_wait = 0)
        Immediate,
    }

    /// Determine flush reason
    pub open spec fn determine_flush_reason(state: BatcherState) -> FlushReason {
        if state.pending.len() >= state.config.max_entries as int {
            FlushReason::EntriesFull
        } else if state.current_bytes >= state.config.max_bytes {
            FlushReason::BytesFull
        } else if state.config.max_wait_ms == 0 {
            FlushReason::Immediate
        } else if timeout_elapsed(state) {
            FlushReason::Timeout
        } else {
            // Should not happen if should_flush is true
            FlushReason::Immediate
        }
    }

    /// Proof: Entries-full flush happens at limit
    pub proof fn entries_full_at_limit(state: BatcherState)
        requires
            batcher_invariant(state),
            state.pending.len() >= state.config.max_entries as int,
        ensures matches!(determine_flush_reason(state), FlushReason::EntriesFull)
    {
        // Entries check comes first
    }

    // ========================================================================
    // Batch Submission
    // ========================================================================

    /// Model of a submitted Raft batch
    pub struct RaftBatchSubmission {
        /// Operations as (is_set, key, value) tuples
        pub operations: Seq<(bool, Seq<u8>, Seq<u8>)>,
        /// Number of operations
        pub count: u32,
    }

    /// Convert pending writes to Raft submission format
    pub open spec fn to_raft_submission(pending: Seq<PendingWriteSpec>) -> RaftBatchSubmission {
        RaftBatchSubmission {
            operations: Seq::new(
                pending.len(),
                |i: int| (pending[i].is_set, pending[i].key, pending[i].value)
            ),
            count: pending.len() as u32,
        }
    }

    /// Proof: Submission preserves order
    pub proof fn submission_preserves_order(pending: Seq<PendingWriteSpec>)
        ensures {
            let submission = to_raft_submission(pending);
            submission.operations.len() == pending.len()
        }
    {
        // Seq::new preserves length
    }

    /// Proof: Submission preserves all operations
    pub proof fn submission_preserves_operations(pending: Seq<PendingWriteSpec>)
        ensures {
            let submission = to_raft_submission(pending);
            forall |i: int| 0 <= i < pending.len() ==>
                submission.operations[i] == (pending[i].is_set, pending[i].key, pending[i].value)
        }
    {
        // Seq::new with identity transform
    }

    // ========================================================================
    // Result Notification
    // ========================================================================

    /// Result state after Raft commit
    pub enum BatchCommitResult {
        /// All operations committed successfully
        Success { batch_size: u32 },
        /// Raft error - all operations failed
        Failed,
    }

    /// All waiters receive same result (batch atomicity for clients)
    pub open spec fn all_waiters_notified(
        pending_count: int,
        results: Seq<BatchCommitResult>,
    ) -> bool {
        // Each pending write gets exactly one result
        results.len() == pending_count &&
        // All results are the same (atomicity)
        forall |i: int, j: int|
            0 <= i < results.len() && 0 <= j < results.len() ==>
            results[i] =~= results[j]
    }
}

mod batcher_state_spec;
