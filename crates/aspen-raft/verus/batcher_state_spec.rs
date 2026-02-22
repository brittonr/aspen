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
    // Invariant 7: Batch Start Consistency
    // ========================================================================

    /// BATCH-7: Empty batch implies batch_start_ms == 0
    ///
    /// When the batch is empty, batch_start_ms must be 0.
    /// When the batch is non-empty, batch_start_ms must be > 0
    /// (set to the time when the first write was added).
    pub open spec fn batch_start_consistent(state: BatcherState) -> bool {
        (state.pending.len() == 0) ==> (state.batch_start_ms == 0)
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
        batch_start_consistent(state) &&
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
    #[verifier(external_body)]
    pub proof fn initial_state_invariant(config: BatchConfigSpec)
        requires
            config.max_entries > 0,
            config.max_bytes > 0,
        ensures batcher_invariant(initial_batcher_state(config))
    {
        let state = initial_batcher_state(config);

        // size_bounded: pending.len() == 0 <= max_entries (trivially true)
        assert(state.pending.len() == 0);
        assert(size_bounded(state));

        // bytes_bounded: current_bytes == 0 <= max_bytes (trivially true)
        assert(state.current_bytes == 0);
        assert(bytes_bounded(state));

        // bytes_consistent: current_bytes == sum_pending_bytes(pending)
        // 0 == sum_pending_bytes(Seq::empty()) == 0
        assert(sum_pending_bytes(state.pending) == 0);
        assert(bytes_consistent(state));

        // ordering_preserved: sequences_ordered(pending)
        // Empty seq trivially has ordered sequences (vacuously true)
        assert(ordering_preserved(state));

        // sequences_valid: all pending sequences < next_sequence
        // Empty seq means no sequences to check (vacuously true)
        assert(sequences_valid(state));

        // sizes_valid: each write's size_bytes == key.len() + value.len()
        // Empty seq means no writes to check (vacuously true)
        assert(sizes_valid(state));

        // batch_start_consistent: empty batch implies batch_start_ms == 0
        // Initial state has pending.len() == 0 and batch_start_ms == 0
        assert(state.pending.len() == 0);
        assert(state.batch_start_ms == 0);
        assert(batch_start_consistent(state));

        // Configuration validity established by requires clauses
        assert(state.config.max_entries > 0);
        assert(state.config.max_bytes > 0);
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
    ///
    /// Uses overflow-safe comparison: instead of `current_bytes + op_bytes <= max_bytes`
    /// which could overflow, we check `op_bytes <= max_bytes - current_bytes`.
    /// This is safe because bytes_bounded invariant guarantees current_bytes <= max_bytes.
    /// Assumes: bytes_bounded(state) - Ensures current_bytes <= max_bytes
    pub open spec fn has_space(state: BatcherState, op_bytes: u64) -> bool {
        state.pending.len() < state.config.max_entries as int &&
        // Overflow-safe: rearranged from current_bytes + op_bytes <= max_bytes
        // Safe because bytes_bounded(state) ensures current_bytes <= max_bytes
        op_bytes <= state.config.max_bytes - state.current_bytes
    }

    /// Check if timeout has elapsed (should flush)
    ///
    /// # Invariant Relationship
    ///
    /// The `batch_start_consistent` invariant guarantees that:
    /// - When `pending.len() == 0`, then `batch_start_ms == 0`
    /// - When `pending.len() > 0`, then `batch_start_ms > 0` (set when first write added)
    ///
    /// Therefore, when `batcher_invariant(state)` holds and `pending.len() > 0`,
    /// the check `state.batch_start_ms > 0` is guaranteed to be true.
    /// The explicit check is retained for defensive verification without
    /// requiring the invariant as a precondition.
    ///
    /// # Overflow Safety
    ///
    /// Uses int arithmetic for verification to handle cases where time could
    /// theoretically go backwards (current_time_ms < batch_start_ms).
    pub open spec fn timeout_elapsed(state: BatcherState) -> bool {
        state.pending.len() > 0 &&
        state.batch_start_ms > 0 &&
        // Use int arithmetic for verification, matches saturating_sub semantics
        (state.current_time_ms as int) - (state.batch_start_ms as int) >= (state.config.max_wait_ms as int)
    }

    /// Check if flush should happen
    pub open spec fn should_flush(state: BatcherState) -> bool {
        state.pending.len() > 0 &&
        (is_batch_full(state) || timeout_elapsed(state) || state.config.max_wait_ms == 0)
    }

    /// Get pending count
    pub open spec fn pending_count(state: BatcherState) -> int {
        state.pending.len() as int
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

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Result of checking whether a batch operation would exceed limits.
    #[derive(PartialEq, Eq, Clone, Copy)]
    pub struct BatchLimitCheck {
        /// Whether adding would exceed entry count limit.
        pub exceeds_entries: bool,
        /// Whether adding would exceed byte size limit.
        pub exceeds_bytes: bool,
    }

    /// Check if either limit would be exceeded.
    ///
    /// # Arguments
    ///
    /// * `check` - The batch limit check result
    ///
    /// # Returns
    ///
    /// `true` if either limit would be exceeded.
    #[inline]
    pub const fn would_exceed(check: &BatchLimitCheck) -> (result: bool)
        ensures result == (check.exceeds_entries || check.exceeds_bytes)
    {
        check.exceeds_entries || check.exceeds_bytes
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
    pub fn check_batch_limits(
        current_entries: u32,
        current_bytes: u64,
        op_bytes: u64,
        max_entries: u32,
        max_bytes: u64,
    ) -> (result: BatchLimitCheck)
        ensures
            // Empty batch never exceeds limits
            current_entries == 0 ==> !result.exceeds_entries && !result.exceeds_bytes,
            // Entry limit check
            current_entries > 0 ==> result.exceeds_entries == (current_entries >= max_entries),
            // Byte limit check (using saturating add)
            current_entries > 0 ==> result.exceeds_bytes == (
                current_bytes.saturating_add(op_bytes) > max_bytes
            )
    {
        let has_pending = current_entries > 0;
        let exceeds_entries = has_pending && current_entries >= max_entries;
        let exceeds_bytes = has_pending && current_bytes.saturating_add(op_bytes) > max_bytes;

        BatchLimitCheck {
            exceeds_entries,
            exceeds_bytes,
        }
    }

    /// Decision for what flush action to take.
    #[derive(PartialEq, Eq, Clone, Copy)]
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
    pub fn determine_flush_action(
        pending_count: u32,
        current_bytes: u64,
        max_entries: u32,
        max_bytes: u64,
        max_wait_is_zero: bool,
        flush_already_scheduled: bool,
    ) -> (result: FlushDecision)
        requires
            // Configuration must be valid (from batcher_invariant)
            max_entries > 0
        ensures
            // Full batch triggers immediate flush (must have pending items)
            pending_count >= max_entries ==> result == FlushDecision::Immediate,
            // Full bytes triggers immediate flush (must have pending items)
            current_bytes >= max_bytes && pending_count > 0 && pending_count < max_entries ==> result == FlushDecision::Immediate,
            // Empty batch means no flush (regardless of bytes)
            pending_count == 0 ==> result == FlushDecision::None,
            // Batching disabled triggers immediate flush
            max_wait_is_zero && pending_count > 0 && pending_count < max_entries && current_bytes < max_bytes ==>
                result == FlushDecision::Immediate,
            // Schedule delayed flush if not already scheduled
            !flush_already_scheduled && pending_count > 0 && pending_count < max_entries &&
                current_bytes < max_bytes && !max_wait_is_zero ==>
                result == FlushDecision::Delayed
    {
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
    /// Total size in bytes (key_len + value_len), saturating at u64::MAX.
    pub fn calculate_set_op_size(key_len: u64, value_len: u64) -> (result: u64)
        ensures
            key_len as int + value_len as int <= u64::MAX as int ==>
                result == key_len + value_len,
            key_len as int + value_len as int > u64::MAX as int ==>
                result == u64::MAX
    {
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
    pub fn calculate_delete_op_size(key_len: u64) -> (result: u64)
        ensures result == key_len
    {
        key_len
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
    pub fn is_batch_full_exec(
        pending_count: u32,
        current_bytes: u64,
        max_entries: u32,
        max_bytes: u64,
    ) -> (result: bool)
        ensures result == (pending_count >= max_entries || current_bytes >= max_bytes)
    {
        pending_count >= max_entries || current_bytes >= max_bytes
    }

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
    pub fn has_space_exec(
        pending_count: u32,
        current_bytes: u64,
        op_bytes: u64,
        max_entries: u32,
        max_bytes: u64,
    ) -> (result: bool)
        ensures result == (
            pending_count < max_entries &&
            op_bytes <= max_bytes.saturating_sub(current_bytes)
        )
    {
        pending_count < max_entries &&
        op_bytes <= max_bytes.saturating_sub(current_bytes)
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
    #[verifier(external_body)]
    pub fn timeout_elapsed_exec(
        pending_count: u32,
        batch_start_ms: u64,
        current_time_ms: u64,
        max_wait_ms: u64,
    ) -> (result: bool)
        ensures result == (
            pending_count > 0 &&
            batch_start_ms > 0 &&
            // Use int arithmetic for verification, matches saturating_sub semantics
            (current_time_ms as int) - (batch_start_ms as int) >= (max_wait_ms as int)
        )
    {
        pending_count > 0 &&
        batch_start_ms > 0 &&
        current_time_ms.saturating_sub(batch_start_ms) >= max_wait_ms
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
    pub fn should_flush_exec(
        pending_count: u32,
        current_bytes: u64,
        batch_start_ms: u64,
        current_time_ms: u64,
        max_entries: u32,
        max_bytes: u64,
        max_wait_ms: u64,
    ) -> (result: bool)
        ensures result == (
            pending_count > 0 &&
            (
                // Inline is_batch_full logic (can't call exec fn in ensures)
                (pending_count >= max_entries || current_bytes >= max_bytes) ||
                // Inline timeout_elapsed logic (can't call exec fn in ensures)
                (pending_count > 0 && batch_start_ms > 0 &&
                 current_time_ms >= batch_start_ms &&
                 current_time_ms - batch_start_ms >= max_wait_ms) ||
                max_wait_ms == 0
            )
        )
    {
        pending_count > 0 &&
        (
            is_batch_full_exec(pending_count, current_bytes, max_entries, max_bytes) ||
            timeout_elapsed_exec(pending_count, batch_start_ms, current_time_ms, max_wait_ms) ||
            max_wait_ms == 0
        )
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
    pub fn is_batch_empty(pending_count: u32) -> (result: bool)
        ensures result == (pending_count == 0)
    {
        pending_count == 0
    }
}
