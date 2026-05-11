//! Sequence Reserve Operation Specifications
//!
//! Proves that the reserve() operation preserves all sequence invariants.
//!
//! # Key Properties
//!
//! 1. **Range Validity**: Returned range is valid and within bounds
//! 2. **Uniqueness**: Each reserve produces a unique, non-overlapping range
//! 3. **Monotonicity**: Current value strictly increases
//! 4. **Overflow Check**: Fails if count would cause overflow
//!
//! # Reserve Semantics
//!
//! When reserve(count) succeeds:
//! - Returns range [current + 1, current + 1 + count)
//! - Updates current_value to current + count
//! - Atomic via CAS
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/sequence_reserve_spec.rs
//! ```

use vstd::prelude::*;

use super::sequence_state_spec::*;

verus! {
    // ========================================================================
    // Reserve Precondition
    // ========================================================================

    /// Precondition for sequence reserve
    ///
    /// The reserve can succeed if:
    /// - count > 0 (must reserve at least one)
    /// - current + count does not overflow
    pub open spec fn reserve_pre(state: SequenceState, count: u64) -> bool {
        count > 0 &&
        !would_overflow(state.current_value, count, state.max_value)
    }

    // ========================================================================
    // Reserve Postcondition
    // ========================================================================

    /// Result range from a successful reserve
    ///
    /// Returns the range [current + 1, current + count + 1)
    ///
    /// Assumes:
    /// - reserve_pre(pre, count)
    /// - // Overflow protection for the +1 in both start and end calculations // Since reserve_pre ensures current + count <= max_value
    /// - // we need current + 1 <= max_value (for start) and // current + count + 1 <= max_value + 1 (for end, which is exclusive) pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count
    /// - // Ensures current + count + 1 doesn't overflow
    pub open spec fn reserve_range(pre: SequenceState, count: u64) -> ReservedRange {
        ReservedRange {
            start: (pre.current_value + 1) as u64,
            end: (add_u64(pre.current_value, count) + 1) as u64,
        }
    }

    /// Result state after a successful reserve
    ///
    /// Updates current_value to current + count
    ///
    /// Assumes:
    /// - reserve_pre(pre, count)
    pub open spec fn reserve_post(pre: SequenceState, count: u64) -> SequenceState {
        SequenceState {
            current_value: (add_u64(pre.current_value, count)) as u64,
            start_value: pre.start_value,
            max_value: pre.max_value,
        }
    }

    // ========================================================================
    // Proofs: Range Validity
    // ========================================================================

    /// The reserved range is non-empty
    pub proof fn reserve_range_nonempty(
        pre: SequenceState,
        count: u64,
    )
        requires
            reserve_pre(pre, count),
            pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count,
        ensures ({
            let range = reserve_range(pre, count);
            range.start < range.end
        })
    {
        let range = reserve_range(pre, count);
        assert(count > 0);
        assert(pre.current_value + count < 0xFFFF_FFFF_FFFF_FFFFu64);
        assert(range.start == pre.current_value + 1);
        assert(range.end == pre.current_value + count + 1);
        assert(pre.current_value + 1 < pre.current_value + count + 1);
    }

    /// The reserved range has the correct size
    pub proof fn reserve_range_size_correct(
        pre: SequenceState,
        count: u64,
    )
        requires
            reserve_pre(pre, count),
            pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count,
        ensures ({
            let range = reserve_range(pre, count);
            range.end - range.start == count
        })
    {
        let range = reserve_range(pre, count);
        assert(count > 0);
        assert(pre.current_value + count < 0xFFFF_FFFF_FFFF_FFFFu64);
        assert(range.start == pre.current_value + 1);
        assert(range.end == pre.current_value + count + 1);
    }

    // ========================================================================
    // Proofs: Uniqueness via Disjoint Ranges
    // ========================================================================

    /// Two sequential reserves produce disjoint ranges
    ///
    /// This is the core uniqueness proof: if we reserve count1, then count2,
    /// the ranges [start1, end1) and [start2, end2) do not overlap.
    pub proof fn sequential_reserves_disjoint(
        initial: SequenceState,
        count1: u64,
        count2: u64,
    )
        requires
            reserve_pre(initial, count1),
            initial.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count1 - count2,
        ensures ({
            let range1 = reserve_range(initial, count1);
            let mid = reserve_post(initial, count1);
            let range2 = reserve_range(mid, count2);
            ranges_disjoint(range1, range2)
        })
    {
        let range1 = reserve_range(initial, count1);
        let mid = reserve_post(initial, count1);
        let range2 = reserve_range(mid, count2);
        assert(count1 > 0);
        assert(initial.current_value + count1 + count2 < 0xFFFF_FFFF_FFFF_FFFFu64);
        assert(mid.current_value == initial.current_value + count1);
        assert(range1.end == initial.current_value + count1 + 1);
        assert(range2.start == mid.current_value + 1);
        assert(range1.end == range2.start);
    }

    /// Values within the same range are unique
    ///
    /// Since sequence IDs are the indices themselves (value_at(i) == i),
    /// uniqueness follows from the injectivity of the identity function.
    ///
    /// This proof verifies that for any two distinct indices in the range,
    /// the values are distinct.
    pub proof fn range_values_unique(range: ReservedRange)
        requires range.start < range.end
        ensures
            // For any two distinct indices in the range, the values are distinct
            forall |i: u64, j: u64|
                range.start <= i < range.end &&
                range.start <= j < range.end &&
                i != j
                ==> value_at(range, i) != value_at(range, j)
    {
        assert forall |i: u64, j: u64|
            range.start <= i < range.end &&
            range.start <= j < range.end &&
            i != j
            implies value_at(range, i) != value_at(range, j)
        by {
            assert(value_at(range, i) == i);
            assert(value_at(range, j) == j);
        }
    }

    /// Get the value at a given index in the range
    /// (The sequence returns the index itself as the value)
    ///
    /// Assumes:
    /// - range.start <= idx < range.end
    pub open spec fn value_at(range: ReservedRange, idx: u64) -> u64 {
        idx  // Sequence IDs are their indices
    }

    // ========================================================================
    // Proofs: Monotonicity
    // ========================================================================

    /// Reserve strictly increases current_value
    pub proof fn reserve_strictly_increases(
        pre: SequenceState,
        count: u64,
    )
        requires
            reserve_pre(pre, count),
            pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count,
        ensures
            sequence_strictly_increases(pre, reserve_post(pre, count))
    {
        let post = reserve_post(pre, count);
        assert(count > 0);
        assert(post.current_value == pre.current_value + count);
    }

    /// Reserve maintains monotonicity
    pub proof fn reserve_maintains_monotonicity(
        pre: SequenceState,
        count: u64,
    )
        requires
            reserve_pre(pre, count),
            pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count,
        ensures
            sequence_monotonic(pre, reserve_post(pre, count))
    {
        reserve_strictly_increases(pre, count);
    }

    // ========================================================================
    // Proofs: Range Start Monotonicity
    // ========================================================================

    /// Each reserve returns a range starting after the previous
    pub proof fn reserve_start_increases(
        pre: SequenceState,
        count: u64,
    )
        requires
            reserve_pre(pre, count),
            pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count,
        ensures ({
            let range = reserve_range(pre, count);
            range.start > pre.current_value
        })
    {
        let range = reserve_range(pre, count);
        assert(pre.current_value < u64::MAX);
        assert(range.start == pre.current_value + 1);
    }

    // ========================================================================
    // Proofs: Invariant Preservation
    // ========================================================================

    /// Reserve preserves the sequence invariant
    pub proof fn reserve_preserves_invariant(
        pre: SequenceState,
        count: u64,
    )
        requires
            reserve_pre(pre, count),
            sequence_invariant(pre),
            pre.current_value < 0xFFFF_FFFF_FFFF_FFFFu64 - count,
        ensures
            sequence_invariant(reserve_post(pre, count))
    {
        let post = reserve_post(pre, count);
        assert(count > 0);
        assert(post.current_value == pre.current_value + count);
        assert(post.start_value == pre.start_value);
        assert(post.max_value == pre.max_value);
        assert(pre.current_value <= pre.max_value - count);
        assert(post.current_value <= post.max_value);
        assert(pre.current_value >= pre.start_value - 1);
        assert(post.current_value >= post.start_value - 1);
        assert(post.start_value > 0);
    }

    // ========================================================================
    // Proofs: First Reserve
    // ========================================================================

    /// First reserve on initial state returns start_value
    pub proof fn first_reserve_returns_start(
        start_value: u64,
        count: u64,
    )
        requires
            start_value > 0,
            count > 0,
            start_value + count <= 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures ({
            let initial = initial_sequence_state(start_value);
            let range = reserve_range(initial, count);
            range.start == start_value
        })
    {
        let initial = initial_sequence_state(start_value);
        let range = reserve_range(initial, count);
        assert(initial.current_value == start_value - 1);
        assert(range.start == initial.current_value + 1);
        assert(range.start == start_value);
    }
}
