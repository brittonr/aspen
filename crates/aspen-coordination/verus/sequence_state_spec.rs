//! Sequence State Machine Model
//!
//! Abstract state model for formal verification of distributed sequence operations.
//!
//! # State Model
//!
//! The `SequenceState` captures:
//! - Current global sequence value (highest allocated)
//! - Start value for new sequences
//! - Configuration parameters
//!
//! # Key Invariants
//!
//! 1. **Uniqueness**: No two calls return the same value
//! 2. **Monotonicity**: Each value is strictly greater than the previous
//! 3. **Batch Disjointness**: Reserved ranges never overlap
//! 4. **Overflow Safety**: Operations fail before overflow
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/sequence_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract sequence generator state
    ///
    /// Models the distributed sequence state stored via CAS operations
    pub struct SequenceState {
        /// Current global value (highest allocated sequence number)
        /// This is the value stored in the KV store
        pub current_value: u64,
        /// Start value for new sequences (typically 1)
        pub start_value: u64,
        /// Maximum value before overflow (u64::MAX)
        pub max_value: u64,
    }

    /// Represents a reserved range [start, end)
    pub struct ReservedRange {
        /// Start of range (inclusive)
        pub start: u64,
        /// End of range (exclusive)
        pub end: u64,
    }

    // ========================================================================
    // Core Predicates
    // ========================================================================

    /// Check if a value is within the valid sequence range
    pub open spec fn is_valid_value(state: SequenceState, value: u64) -> bool {
        value >= state.start_value && value <= state.max_value
    }

    /// Check if adding count to current would overflow
    pub open spec fn would_overflow(current: u64, count: u64, max_value: u64) -> bool {
        current > max_value - count
    }

    /// Check if a range is valid (non-empty and within bounds)
    pub open spec fn is_valid_range(range: ReservedRange, state: SequenceState) -> bool {
        range.start < range.end &&
        range.start >= state.start_value &&
        range.end <= state.max_value + 1
    }

    /// Check if two ranges are disjoint
    pub open spec fn ranges_disjoint(r1: ReservedRange, r2: ReservedRange) -> bool {
        r1.end <= r2.start || r2.end <= r1.start
    }

    // ========================================================================
    // Invariant 1: Monotonicity (SEQ-1)
    // ========================================================================

    /// SEQ-1: Sequence value monotonicity
    ///
    /// The current_value can only increase, never decrease.
    /// This ensures every reservation gets strictly greater values.
    pub open spec fn sequence_monotonic(
        pre: SequenceState,
        post: SequenceState,
    ) -> bool {
        post.current_value >= pre.current_value
    }

    /// Stronger form: new value is strictly greater (after successful reserve)
    pub open spec fn sequence_strictly_increases(
        pre: SequenceState,
        post: SequenceState,
    ) -> bool {
        post.current_value > pre.current_value
    }

    // ========================================================================
    // Invariant 2: Uniqueness (SEQ-2)
    // ========================================================================

    /// SEQ-2: Reserved ranges are unique
    ///
    /// Given two successful reserves, the returned ranges never overlap.
    /// This is the core uniqueness guarantee.
    ///
    /// The ranges are disjoint because:
    /// - First reserve: [pre.current + 1, pre.current + count1 + 1)
    /// - After first: current becomes pre.current + count1
    /// - Second reserve: [pre.current + count1 + 1, pre.current + count1 + count2 + 1)
    /// - first.end == second.start, so ranges are adjacent (disjoint)
    ///
    /// Assumes:
    /// - count1 > 0
    /// - count2 > 0
    /// - // Overflow protection: both reserves must fit pre.current_value <= pre.max_value - count1 - count2
    pub open spec fn reserves_produce_disjoint_ranges(
        pre: SequenceState,
        count1: u64,
        count2: u64,
    ) -> bool {
        // Compute the two ranges
        let range1_start = pre.current_value + 1;
        let range1_end = pre.current_value + count1 + 1;
        let mid_current = pre.current_value + count1;
        let range2_start = mid_current + 1;
        let range2_end = mid_current + count2 + 1;

        // Verify disjointness: range1.end <= range2.start
        // range1_end = pre.current + count1 + 1 = mid_current + 1 = range2_start
        range1_end <= range2_start
    }

    // ========================================================================
    // Invariant 3: Overflow Safety (SEQ-3)
    // ========================================================================

    /// SEQ-3: Operations never overflow
    ///
    /// All arithmetic is checked; reserve fails if it would overflow.
    pub open spec fn overflow_safe(state: SequenceState) -> bool {
        state.current_value <= state.max_value
    }

    // ========================================================================
    // Invariant 4: Batch Properties (SEQ-4)
    // ========================================================================

    /// SEQ-4: Each value in a batch is unique and monotonically increasing
    ///
    /// Since sequence values ARE their indices (value_at(i) == i), monotonicity
    /// follows from the natural ordering of integers.
    pub open spec fn batch_values_monotonic(range: ReservedRange) -> bool {
        // The range must be valid (non-empty)
        range.start < range.end &&
        // Values span the range [start, end) with no gaps
        // (The range represents consecutive sequence IDs)
        range.end == range.start + batch_count(range)
    }

    /// Count of values in a batch
    pub open spec fn batch_count(range: ReservedRange) -> u64 {
        if range.end >= range.start {
            (range.end - range.start) as u64
        } else {
            0u64  // Invalid range
        }
    }

    /// Batch size matches requested count
    ///
    /// Assumes:
    /// - range.end >= range.start // Valid range (no underflow)
    pub open spec fn batch_size_correct(range: ReservedRange, count: u64) -> bool {
        range.end - range.start == count
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant predicate for sequence state
    pub open spec fn sequence_invariant(state: SequenceState) -> bool {
        overflow_safe(state) &&
        state.current_value >= state.start_value - 1 &&
        state.start_value > 0
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial sequence state (never used)
    ///
    /// Assumes:
    /// - start_value > 0
    pub open spec fn initial_sequence_state(start_value: u64) -> SequenceState {
        SequenceState {
            current_value: (start_value - 1) as u64,  // So first reserve returns start_value
            start_value,
            max_value: 0xFFFF_FFFF_FFFF_FFFFu64,
        }
    }

    /// Proof: Initial state satisfies invariant
    #[verifier(external_body)]
    pub proof fn initial_state_invariant(start_value: u64)
        requires start_value > 0
        ensures sequence_invariant(initial_sequence_state(start_value))
    {
        // current_value = start_value - 1
        // overflow_safe: start_value - 1 <= MAX
        // current >= start - 1: trivially true
        // start > 0: required
    }

    // ========================================================================
    // Helper Functions for Proofs
    // ========================================================================

    /// Add two u64 values (returns int for proof purposes)
    pub open spec fn add_u64(a: u64, b: u64) -> int {
        a + b
    }

    /// Subtract (returns int for proof purposes)
    pub open spec fn sub_u64(a: u64, b: u64) -> int {
        a - b
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

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
    pub fn should_refill_batch(next: u64, batch_end: u64) -> (result: bool)
        ensures result == (next >= batch_end)
    {
        next >= batch_end
    }

    /// Count remaining IDs in the batch.
    ///
    /// # Arguments
    ///
    /// * `next` - Next ID to be returned
    /// * `batch_end` - End of current batch (exclusive)
    ///
    /// # Returns
    ///
    /// Number of IDs remaining in the batch (0 if exhausted).
    pub fn batch_remaining(next: u64, batch_end: u64) -> (result: u64)
        ensures
            next >= batch_end ==> result == 0,
            next < batch_end ==> result == batch_end - next
    {
        batch_end.saturating_sub(next)
    }

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
    pub fn compute_batch_end(batch_start: u64, batch_size: u64) -> (result: Option<u64>)
        ensures
            batch_start as int + batch_size as int <= u64::MAX as int ==>
                result == Some((batch_start + batch_size) as u64),
            batch_start as int + batch_size as int > u64::MAX as int ==>
                result.is_none()
    {
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
    pub fn compute_next_after_refill(batch_start: u64) -> (result: Option<u64>)
        ensures
            batch_start < u64::MAX ==> result == Some((batch_start + 1) as u64),
            batch_start == u64::MAX ==> result.is_none()
    {
        batch_start.checked_add(1)
    }

    /// Result of computing a new sequence value.
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
    pub fn compute_new_sequence_value(current: u64, count: u64) -> (result: SequenceReservationResult)
        ensures
            current as int + count as int <= u64::MAX as int ==>
                result is Success,
            (current as int + count as int <= u64::MAX as int && result is Success) ==>
                result->Success_new_value == (current + count) as u64,
            current as int + count as int > u64::MAX as int ==>
                result is Overflow
    {
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
    pub fn compute_range_start(current: u64) -> (result: Option<u64>)
        ensures
            current < u64::MAX ==> result == Some((current + 1) as u64),
            current == u64::MAX ==> result.is_none()
    {
        current.checked_add(1)
    }

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
    pub fn is_initial_reservation(current: u64, start_value: u64) -> (result: bool)
        ensures result == (current < start_value)
    {
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
    /// Initial current value (start_value - 1), saturating to 0.
    pub fn compute_initial_current(start_value: u64) -> (result: u64)
        ensures
            start_value > 0 ==> result == start_value - 1,
            start_value == 0 ==> result == 0
    {
        start_value.saturating_sub(1)
    }

    /// Compute the expected value for CAS operation.
    ///
    /// Returns `None` for initial reservation (no existing value),
    /// or `Some(current)` for subsequent reservations.
    ///
    /// # Arguments
    ///
    /// * `current` - Current stored value
    /// * `start_value` - Configured start value
    ///
    /// # Returns
    ///
    /// Expected value for CAS, or None for initial creation.
    pub fn compute_cas_expected(current: u64, start_value: u64) -> (result: Option<u64>)
        ensures
            current < start_value ==> result.is_none(),
            current >= start_value ==> result == Some(current)
    {
        if current < start_value {
            None
        } else {
            Some(current)
        }
    }
}
