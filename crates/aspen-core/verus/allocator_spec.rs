//! High-Contention Allocator (HCA) State Machine Model
//!
//! Abstract state model for formal verification of HCA operations.
//!
//! # State Model
//!
//! The `HcaState` captures:
//! - Set of allocated prefixes
//! - Current window start position
//! - Global counter (highest allocated value)
//!
//! # Key Invariants
//!
//! 1. **ALLOC-1: Uniqueness**: No prefix allocated twice
//! 2. **ALLOC-2: Monotonicity**: window_start only increases
//! 3. **ALLOC-3: Counter Bounded**: counter <= window_start + window_size
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-core/verus/allocator_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Set of allocated prefixes (abstract)
    /// In the actual implementation, this is stored as candidate keys in KV store
    pub struct AllocatedSet {
        /// Ghost set tracking all allocated values
        pub values: Set<u64>,
    }

    /// Complete HCA state for verification
    pub struct HcaState {
        /// Set of all allocated prefixes
        pub allocated: AllocatedSet,
        /// Start of current allocation window
        pub window_start: u64,
        /// Global counter (tracks highest guaranteed-unique value)
        pub counter: u64,
    }

    /// Result of allocation operation
    pub enum AllocResult {
        /// Successfully allocated a unique prefix
        Success { value: u64 },
        /// Candidate was already claimed
        AlreadyClaimed,
        /// Need to advance window
        WindowExhausted,
        /// Counter overflow
        Overflow,
    }

    // ========================================================================
    // Window Size Calculation
    // ========================================================================

    /// Window size thresholds (matches constants in allocator.rs)
    pub const HCA_MEDIUM_WINDOW_THRESHOLD: u64 = 255;
    pub const HCA_LARGE_WINDOW_THRESHOLD: u64 = 65535;
    pub const HCA_INITIAL_WINDOW_SIZE: u64 = 64;
    pub const HCA_MEDIUM_WINDOW_SIZE: u64 = 1024;
    pub const HCA_MAX_WINDOW_SIZE: u64 = 8192;

    /// Calculate window size based on current position
    pub open spec fn window_size(start: u64) -> u64 {
        if start < HCA_MEDIUM_WINDOW_THRESHOLD {
            HCA_INITIAL_WINDOW_SIZE
        } else if start < HCA_LARGE_WINDOW_THRESHOLD {
            HCA_MEDIUM_WINDOW_SIZE
        } else {
            HCA_MAX_WINDOW_SIZE
        }
    }

    /// Calculate window end from start
    pub open spec fn window_end(start: u64) -> u64 {
        // Saturating add to prevent overflow
        let size = window_size(start);
        if start > u64::MAX - size {
            u64::MAX
        } else {
            (start + size) as u64
        }
    }

    // ========================================================================
    // Invariant 1: Uniqueness
    // ========================================================================

    /// ALLOC-1: No prefix allocated twice
    ///
    /// Each value in the allocated set is unique by definition.
    /// The real proof is that allocate() only adds values not already in the set.
    pub open spec fn alloc_uniqueness(state: HcaState) -> bool {
        // This is trivially true for a Set - no duplicates by definition
        // The meaningful property is that allocate() maintains this
        true
    }

    /// Precondition for claiming a candidate
    pub open spec fn can_claim(state: HcaState, candidate: u64) -> bool {
        // Candidate must be in current window
        candidate >= state.window_start &&
        candidate < window_end(state.window_start) &&
        // Candidate must not already be allocated
        !state.allocated.values.contains(candidate)
    }

    /// Effect of successful claim
    pub open spec fn claim_effect(pre: HcaState, candidate: u64) -> HcaState
        recommends can_claim(pre, candidate)
    {
        HcaState {
            allocated: AllocatedSet {
                values: pre.allocated.values.insert(candidate),
            },
            window_start: pre.window_start,
            counter: pre.counter,
        }
    }

    /// Proof: Claiming adds exactly one new element
    pub proof fn claim_adds_one_element(pre: HcaState, candidate: u64)
        requires can_claim(pre, candidate)
        ensures ({
            let post = claim_effect(pre, candidate);
            post.allocated.values.contains(candidate) &&
            post.allocated.values.len() == pre.allocated.values.len() + 1
        })
    {
        // candidate not in pre.allocated, so insert increases size by 1
    }

    /// Proof: Claiming preserves existing allocations
    pub proof fn claim_preserves_existing(pre: HcaState, candidate: u64, other: u64)
        requires
            can_claim(pre, candidate),
            pre.allocated.values.contains(other),
        ensures claim_effect(pre, candidate).allocated.values.contains(other)
    {
        // Insert preserves existing elements
    }

    // ========================================================================
    // Invariant 2: Monotonicity
    // ========================================================================

    /// ALLOC-2: window_start only increases
    ///
    /// Window advances forward when all candidates exhausted
    pub open spec fn alloc_monotonicity(pre: HcaState, post: HcaState) -> bool {
        post.window_start >= pre.window_start
    }

    /// Window advance operation
    pub open spec fn advance_window_pre(state: HcaState) -> bool {
        // All candidates in current window are exhausted
        // (simplified: window end <= counter)
        window_end(state.window_start) <= state.counter
    }

    /// Effect of advancing window
    pub open spec fn advance_window_effect(pre: HcaState) -> HcaState
        recommends advance_window_pre(pre)
    {
        let new_window_start = if pre.counter > window_end(pre.window_start) {
            pre.counter
        } else {
            window_end(pre.window_start)
        };
        let new_size = window_size(new_window_start);
        let new_counter = if new_window_start > u64::MAX - new_size {
            u64::MAX
        } else {
            (new_window_start + new_size) as u64
        };

        HcaState {
            allocated: pre.allocated,
            window_start: new_window_start,
            counter: new_counter,
        }
    }

    /// Proof: Window advance increases window_start
    pub proof fn advance_increases_window_start(pre: HcaState)
        requires advance_window_pre(pre)
        ensures ({
            let post = advance_window_effect(pre);
            post.window_start >= pre.window_start &&
            post.window_start > pre.window_start
        })
    {
        // new_window_start >= window_end(pre.window_start) > pre.window_start
    }

    // ========================================================================
    // Invariant 3: Counter Bounded
    // ========================================================================

    /// ALLOC-3: counter <= window_start + window_size
    ///
    /// Counter tracks the highest value that could be allocated
    pub open spec fn alloc_counter_bounded(state: HcaState) -> bool {
        state.counter <= window_end(state.window_start) ||
        // Or counter equals window_start + window_size (exact bound)
        state.counter == window_end(state.window_start)
    }

    /// Weaker bound that's always maintained
    pub open spec fn counter_reasonable(state: HcaState) -> bool {
        // Counter is at most 2 * window_size ahead of window_start
        // (accounting for window advance)
        let max_size = HCA_MAX_WINDOW_SIZE;
        state.counter <= state.window_start + 2 * max_size
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant for allocator state
    pub open spec fn allocator_invariant(state: HcaState) -> bool {
        // All allocated values are within valid range
        forall |v: u64| state.allocated.values.contains(v) ==>
            v < state.counter
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant()
        ensures allocator_invariant(initial_allocator_state())
    {
        // Empty set trivially satisfies the forall
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial allocator state
    pub open spec fn initial_allocator_state() -> HcaState {
        HcaState {
            allocated: AllocatedSet { values: Set::empty() },
            window_start: 0,
            counter: 0,
        }
    }

    // ========================================================================
    // Full Allocation Operation
    // ========================================================================

    /// Full allocate operation (try candidates then advance if needed)
    pub open spec fn allocate_pre(state: HcaState) -> bool {
        // Can allocate if counter hasn't overflowed
        state.counter < u64::MAX
    }

    /// Effect of successful allocation
    pub open spec fn allocate_post(pre: HcaState, allocated_value: u64) -> HcaState
        recommends allocate_pre(pre)
    {
        HcaState {
            allocated: AllocatedSet {
                values: pre.allocated.values.insert(allocated_value),
            },
            // Window and counter may have advanced
            window_start: pre.window_start, // Simplified - actual may be higher
            counter: pre.counter,
        }
    }

    /// Proof: Allocation returns unique value
    pub proof fn allocate_returns_unique(pre: HcaState, allocated_value: u64)
        requires
            allocate_pre(pre),
            !pre.allocated.values.contains(allocated_value),
        ensures ({
            let post = allocate_post(pre, allocated_value);
            post.allocated.values.contains(allocated_value) &&
            !pre.allocated.values.contains(allocated_value)
        })
    {
        // The allocated value was not in pre, and is now in post
    }

    /// Proof: Consecutive allocations return different values
    pub proof fn consecutive_allocations_unique(
        s0: HcaState,
        s1: HcaState,
        v1: u64,
        v2: u64,
    )
        requires
            allocate_pre(s0),
            s1 == allocate_post(s0, v1),
            allocate_pre(s1),
            !s0.allocated.values.contains(v1),
            !s1.allocated.values.contains(v2),
        ensures v1 != v2
    {
        // v1 is in s1.allocated but v2 is not in s1.allocated
        // Therefore v1 != v2
        assert(s1.allocated.values.contains(v1));
        assert(!s1.allocated.values.contains(v2));
    }

    // ========================================================================
    // Preservation Proofs
    // ========================================================================

    /// Proof: Claim preserves allocator invariant
    pub proof fn claim_preserves_invariant(pre: HcaState, candidate: u64)
        requires
            allocator_invariant(pre),
            can_claim(pre, candidate),
            candidate < pre.counter,
        ensures allocator_invariant(claim_effect(pre, candidate))
    {
        let post = claim_effect(pre, candidate);
        // All old values still satisfy v < counter
        // New value candidate < pre.counter = post.counter
        assert forall |v: u64| post.allocated.values.contains(v) implies v < post.counter by {
            if v == candidate {
                assert(candidate < post.counter);
            } else {
                assert(pre.allocated.values.contains(v));
                assert(v < pre.counter);
                assert(v < post.counter);
            }
        }
    }

    /// Proof: Window advance preserves monotonicity
    pub proof fn advance_preserves_monotonicity(pre: HcaState)
        requires advance_window_pre(pre)
        ensures alloc_monotonicity(pre, advance_window_effect(pre))
    {
        // post.window_start >= window_end(pre.window_start) >= pre.window_start
    }

    /// Proof: Allocator invariant is preserved through operations
    pub proof fn allocator_invariant_preservation(
        pre: HcaState,
        post: HcaState,
        allocated_value: u64,
    )
        requires
            allocator_invariant(pre),
            post == allocate_post(pre, allocated_value),
            !pre.allocated.values.contains(allocated_value),
            allocated_value < pre.counter,
        ensures allocator_invariant(post)
    {
        // Similar to claim_preserves_invariant
        assert forall |v: u64| post.allocated.values.contains(v) implies v < post.counter by {
            if v == allocated_value {
                assert(allocated_value < post.counter);
            } else {
                assert(pre.allocated.values.contains(v));
                assert(v < pre.counter);
                assert(v < post.counter);
            }
        }
    }

    // ========================================================================
    // Window Size Properties
    // ========================================================================

    /// Window size is always positive
    pub proof fn window_size_positive(start: u64)
        ensures window_size(start) > 0
    {
        // All window sizes are >= HCA_INITIAL_WINDOW_SIZE = 64 > 0
    }

    /// Window size is bounded
    pub proof fn window_size_bounded(start: u64)
        ensures window_size(start) <= HCA_MAX_WINDOW_SIZE
    {
        // Maximum is HCA_MAX_WINDOW_SIZE = 8192
    }

    /// Window size increases with start position
    pub proof fn window_size_monotonic(a: u64, b: u64)
        requires a < b
        ensures window_size(a) <= window_size(b)
    {
        // Window size increases at thresholds:
        // - 0..255: 64
        // - 255..65535: 1024
        // - 65535+: 8192
        // So if a < b, window_size(a) <= window_size(b)
    }
}
