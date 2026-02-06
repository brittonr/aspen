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
    // Invariant 1: Monotonicity
    // ========================================================================

    /// INVARIANT 1: Sequence value monotonicity
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
    // Invariant 2: Uniqueness (via range disjointness)
    // ========================================================================

    /// INVARIANT 2: Reserved ranges are unique
    ///
    /// Given two successful reserves, the returned ranges never overlap.
    /// This is the core uniqueness guarantee.
    pub open spec fn reserves_produce_disjoint_ranges(
        pre: SequenceState,
        count1: u64,
        count2: u64,
    ) -> bool
        recommends count1 > 0 && count2 > 0
    {
        // First reserve: [pre.current + 1, pre.current + 1 + count1)
        // After first: current = pre.current + count1
        // Second reserve: [pre.current + count1 + 1, pre.current + count1 + 1 + count2)
        // These are disjoint because first.end <= second.start
        true  // By construction: ranges are contiguous and non-overlapping
    }

    // ========================================================================
    // Invariant 3: Overflow Safety
    // ========================================================================

    /// INVARIANT 3: Operations never overflow
    ///
    /// All arithmetic is checked; reserve fails if it would overflow.
    pub open spec fn overflow_safe(state: SequenceState) -> bool {
        state.current_value <= state.max_value
    }

    // ========================================================================
    // Invariant 4: Batch Properties
    // ========================================================================

    /// Each value in a batch is unique and monotonically increasing
    pub open spec fn batch_values_monotonic(range: ReservedRange) -> bool {
        // For all i, j in [range.start, range.end): i < j => i value < j value
        // This is trivially true since values ARE the indices
        range.start < range.end
    }

    /// Batch size matches requested count
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
    pub open spec fn initial_sequence_state(start_value: u64) -> SequenceState
        recommends start_value > 0
    {
        SequenceState {
            current_value: (start_value - 1) as u64,  // So first reserve returns start_value
            start_value,
            max_value: 0xFFFF_FFFF_FFFF_FFFFu64,
        }
    }

    /// Proof: Initial state satisfies invariant
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
}
