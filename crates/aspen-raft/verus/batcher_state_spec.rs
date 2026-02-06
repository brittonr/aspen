//! Write Batcher State Machine Model
//!
//! Abstract state model for formal verification of write batching operations.
//!
//! # State Model
//!
//! The `BatcherState` captures:
//! - Pending writes waiting to be batched
//! - Current batch size tracking
//! - Configuration limits
//!
//! # Key Invariants
//!
//! 1. **BATCH-1: Size Bound**: pending.len() <= max_entries
//! 2. **BATCH-2: Bytes Bound**: current_bytes <= max_bytes
//! 3. **BATCH-3: Bytes Consistency**: current_bytes == sum of pending sizes
//! 4. **BATCH-4: Ordering Preservation**: FIFO order maintained
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/batcher_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Batch configuration
    pub struct BatchConfigSpec {
        /// Maximum number of operations per batch
        pub max_entries: u32,
        /// Maximum total size of values in bytes per batch
        pub max_bytes: u64,
        /// Maximum time to wait before flushing in milliseconds
        pub max_wait_ms: u64,
    }

    /// A pending write operation
    pub struct PendingWriteSpec {
        /// The operation: is_set=true for Set, false for Delete
        pub is_set: bool,
        /// Key bytes
        pub key: Seq<u8>,
        /// Value bytes (empty for Delete)
        pub value: Seq<u8>,
        /// Size of this operation in bytes (key.len() + value.len())
        pub size_bytes: u64,
        /// Sequence number for ordering (FIFO preservation)
        pub sequence: u64,
    }

    /// Complete batcher state
    pub struct BatcherState {
        /// Pending operations waiting to be batched (in FIFO order)
        pub pending: Seq<PendingWriteSpec>,
        /// Current total bytes in pending batch
        pub current_bytes: u64,
        /// Next sequence number for ordering
        pub next_sequence: u64,
        /// Batch configuration
        pub config: BatchConfigSpec,
        /// When the first item was added (0 if empty)
        pub batch_start_ms: u64,
        /// Current time for timeout checks
        pub current_time_ms: u64,
    }

    // ========================================================================
    // Invariant 1: Size Bound
    // ========================================================================

    /// BATCH-1: pending.len() <= max_entries
    pub open spec fn size_bounded(state: BatcherState) -> bool {
        state.pending.len() <= state.config.max_entries as int
    }

    // ========================================================================
    // Invariant 2: Bytes Bound
    // ========================================================================

    /// BATCH-2: current_bytes <= max_bytes
    pub open spec fn bytes_bounded(state: BatcherState) -> bool {
        state.current_bytes <= state.config.max_bytes
    }

    // ========================================================================
    // Invariant 3: Bytes Consistency
    // ========================================================================

    /// Sum of all pending write sizes
    pub open spec fn sum_pending_bytes(pending: Seq<PendingWriteSpec>) -> int
        decreases pending.len()
    {
        if pending.len() == 0 {
            0
        } else {
            pending[0].size_bytes as int + sum_pending_bytes(pending.skip(1))
        }
    }

    /// BATCH-3: current_bytes == sum of pending sizes
    pub open spec fn bytes_consistent(state: BatcherState) -> bool {
        state.current_bytes as int == sum_pending_bytes(state.pending)
    }

    // ========================================================================
    // Invariant 4: Ordering Preservation
    // ========================================================================

    /// Check if sequences are strictly increasing (FIFO order)
    pub open spec fn sequences_ordered(pending: Seq<PendingWriteSpec>) -> bool {
        forall |i: int, j: int|
            0 <= i < j < pending.len() ==>
            pending[i].sequence < pending[j].sequence
    }

    /// BATCH-4: FIFO order maintained via sequence numbers
    pub open spec fn ordering_preserved(state: BatcherState) -> bool {
        sequences_ordered(state.pending)
    }

    // ========================================================================
    // Invariant 5: Sequence Monotonicity
    // ========================================================================

    /// All pending sequences are less than next_sequence
    pub open spec fn sequences_valid(state: BatcherState) -> bool {
        forall |i: int|
            0 <= i < state.pending.len() ==>
            state.pending[i].sequence < state.next_sequence
    }

    // ========================================================================
    // Invariant 6: Size Consistency
    // ========================================================================

    /// Each write's size_bytes equals key.len() + value.len()
    pub open spec fn sizes_valid(state: BatcherState) -> bool {
        forall |i: int|
            0 <= i < state.pending.len() ==>
            state.pending[i].size_bytes as int ==
                state.pending[i].key.len() + state.pending[i].value.len()
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined batcher invariant
    pub open spec fn batcher_invariant(state: BatcherState) -> bool {
        size_bounded(state) &&
        bytes_bounded(state) &&
        bytes_consistent(state) &&
        ordering_preserved(state) &&
        sequences_valid(state) &&
        sizes_valid(state) &&
        // Configuration is valid
        state.config.max_entries > 0 &&
        state.config.max_bytes > 0
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial empty batcher state
    pub open spec fn initial_batcher_state(config: BatchConfigSpec) -> BatcherState {
        BatcherState {
            pending: Seq::empty(),
            current_bytes: 0,
            next_sequence: 0,
            config: config,
            batch_start_ms: 0,
            current_time_ms: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant(config: BatchConfigSpec)
        requires
            config.max_entries > 0,
            config.max_bytes > 0,
        ensures batcher_invariant(initial_batcher_state(config))
    {
        // Empty seq trivially satisfies ordering and consistency
    }

    // ========================================================================
    // Helper Predicates
    // ========================================================================

    /// Check if batch is full (would trigger flush)
    pub open spec fn is_batch_full(state: BatcherState) -> bool {
        state.pending.len() >= state.config.max_entries as int ||
        state.current_bytes >= state.config.max_bytes
    }

    /// Check if batch has space for another operation
    pub open spec fn has_space(state: BatcherState, op_bytes: u64) -> bool {
        state.pending.len() < state.config.max_entries as int &&
        state.current_bytes + op_bytes <= state.config.max_bytes
    }

    /// Check if timeout has elapsed (should flush)
    pub open spec fn timeout_elapsed(state: BatcherState) -> bool {
        state.pending.len() > 0 &&
        state.batch_start_ms > 0 &&
        state.current_time_ms >= state.batch_start_ms + state.config.max_wait_ms
    }

    /// Check if flush should happen
    pub open spec fn should_flush(state: BatcherState) -> bool {
        state.pending.len() > 0 &&
        (is_batch_full(state) || timeout_elapsed(state) || state.config.max_wait_ms == 0)
    }

    /// Get pending count
    pub open spec fn pending_count(state: BatcherState) -> int {
        state.pending.len()
    }

    /// Check if batcher is empty
    pub open spec fn is_empty(state: BatcherState) -> bool {
        state.pending.len() == 0
    }

    // ========================================================================
    // Write Identification
    // ========================================================================

    /// Check if a write is a Set operation
    pub open spec fn is_set_operation(write: PendingWriteSpec) -> bool {
        write.is_set
    }

    /// Check if a write is a Delete operation
    pub open spec fn is_delete_operation(write: PendingWriteSpec) -> bool {
        !write.is_set
    }

    /// For delete operations, value should be empty
    pub open spec fn delete_has_empty_value(write: PendingWriteSpec) -> bool {
        !write.is_set ==> write.value.len() == 0
    }

    /// All deletes have empty values
    pub open spec fn deletes_have_empty_values(state: BatcherState) -> bool {
        forall |i: int|
            0 <= i < state.pending.len() ==>
            delete_has_empty_value(state.pending[i])
    }
}
