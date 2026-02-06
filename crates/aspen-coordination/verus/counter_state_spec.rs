//! Counter State Machine Model
//!
//! Abstract state model for formal verification of atomic counter operations.
//!
//! # State Model
//!
//! The `CounterState` captures:
//! - Current counter value
//! - Whether this is a signed or unsigned counter
//!
//! # Key Invariants
//!
//! 1. **CAS Atomicity**: Each modify is atomic (via CAS semantics)
//! 2. **Saturating Arithmetic**: Operations saturate at bounds, never overflow/underflow
//! 3. **Monotonicity**: For strictly positive additions, value increases (or saturates)
//! 4. **Linearizability**: All operations are linearizable (via Raft)
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/counter_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract unsigned counter state
    pub struct CounterState {
        /// Current counter value
        pub value: u64,
    }

    /// Abstract signed counter state
    pub struct SignedCounterState {
        /// Current counter value
        pub value: i64,
    }

    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum u64 value
    pub open spec fn u64_max() -> u64 {
        0xFFFF_FFFF_FFFF_FFFFu64
    }

    /// Maximum i64 value
    pub open spec fn i64_max() -> i64 {
        0x7FFF_FFFF_FFFF_FFFFi64
    }

    /// Minimum i64 value
    pub open spec fn i64_min() -> i64 {
        -0x8000_0000_0000_0000i64
    }

    // ========================================================================
    // Saturating Arithmetic Specs
    // ========================================================================

    /// Saturating add for u64
    pub open spec fn saturating_add_u64(a: u64, b: u64) -> u64 {
        if a > u64_max() - b {
            u64_max()
        } else {
            (a + b) as u64
        }
    }

    /// Saturating subtract for u64
    pub open spec fn saturating_sub_u64(a: u64, b: u64) -> u64 {
        if b > a {
            0
        } else {
            (a - b) as u64
        }
    }

    /// Saturating add for i64
    pub open spec fn saturating_add_i64(a: i64, b: i64) -> i64 {
        if b > 0 && a > i64_max() - b {
            i64_max()
        } else if b < 0 && a < i64_min() - b {
            i64_min()
        } else {
            (a + b) as i64
        }
    }

    /// Saturating subtract for i64
    pub open spec fn saturating_sub_i64(a: i64, b: i64) -> i64 {
        if b < 0 && a > i64_max() + b {
            i64_max()
        } else if b > 0 && a < i64_min() + b {
            i64_min()
        } else {
            (a - b) as i64
        }
    }

    // ========================================================================
    // Core Predicates
    // ========================================================================

    /// Counter value is within valid range (always true for u64)
    pub open spec fn counter_valid(state: CounterState) -> bool {
        true  // u64 is always valid
    }

    /// Signed counter value is within valid range
    pub open spec fn signed_counter_valid(state: SignedCounterState) -> bool {
        state.value >= i64_min() && state.value <= i64_max()
    }

    // ========================================================================
    // Invariant 1: Saturation Bounds
    // ========================================================================

    /// INVARIANT 1a: Unsigned counter never overflows (saturates at MAX)
    pub open spec fn add_saturates_at_max(pre: CounterState, amount: u64) -> bool {
        let post_value = saturating_add_u64(pre.value, amount);
        post_value <= u64_max()
    }

    /// INVARIANT 1b: Unsigned counter never underflows (saturates at 0)
    pub open spec fn sub_saturates_at_zero(pre: CounterState, amount: u64) -> bool {
        let post_value = saturating_sub_u64(pre.value, amount);
        post_value >= 0
    }

    // ========================================================================
    // Invariant 2: Monotonicity for Positive Operations
    // ========================================================================

    /// INVARIANT 2: Adding a positive amount increases (or saturates)
    pub open spec fn add_increases_or_saturates(pre: CounterState, amount: u64) -> bool {
        let post_value = saturating_add_u64(pre.value, amount);
        post_value >= pre.value
    }

    /// Subtracting decreases or stays at zero
    pub open spec fn sub_decreases_or_zero(pre: CounterState, amount: u64) -> bool {
        let post_value = saturating_sub_u64(pre.value, amount);
        post_value <= pre.value
    }

    // ========================================================================
    // Invariant 3: CAS Semantics
    // ========================================================================

    /// CAS precondition: expected value matches current
    pub open spec fn cas_pre(state: CounterState, expected: u64) -> bool {
        state.value == expected
    }

    /// CAS postcondition: value is updated to new_value
    pub open spec fn cas_post(pre: CounterState, expected: u64, new_value: u64) -> CounterState
        recommends cas_pre(pre, expected)
    {
        CounterState { value: new_value }
    }

    /// CAS atomicity: if pre matches expected, post has new value
    pub open spec fn cas_atomic(pre: CounterState, expected: u64, new_value: u64) -> bool {
        cas_pre(pre, expected) ==> cas_post(pre, expected, new_value).value == new_value
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant for counter state
    pub open spec fn counter_invariant(state: CounterState) -> bool {
        counter_valid(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial counter state (zero)
    pub open spec fn initial_counter_state() -> CounterState {
        CounterState { value: 0 }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant()
        ensures counter_invariant(initial_counter_state())
    {
        // Counter with value 0 is trivially valid
    }
}
