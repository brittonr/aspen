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

    /// Counter state well-formedness (COUNTER-1)
    ///
    /// Verifies that a counter state is well-formed. Currently type-enforced
    /// (all u64 values are valid), but kept as a predicate for:
    ///
    /// 1. **Proof structure consistency**: Enables the standard pattern of
    ///    `pre: invariant -> operation -> post: invariant`
    /// 2. **Future extensibility**: If additional constraints are needed
    ///    (e.g., named counters with ID validation), they can be added here
    /// 3. **Documentation**: Makes explicit that "valid counter" is a concept
    ///
    /// # Type-Enforced Properties
    ///
    /// The following are enforced by the Rust type system:
    /// - `value` is in range [0, u64::MAX]
    /// - No null/undefined values
    ///
    /// # Meaningful Invariants
    ///
    /// For verification of actual counter behavior, see:
    /// - `counter_saturation_invariant`: Verifies saturating arithmetic
    /// - `add_saturates_at_max`: Add never wraps (COUNTER-2)
    /// - `sub_saturates_at_zero`: Subtract never underflows (COUNTER-3)
    pub open spec fn counter_wellformed(state: CounterState) -> bool {
        true  // Type-enforced; see counter_saturation_invariant for real properties
    }

    /// Deprecated: Use `counter_wellformed` instead
    ///
    /// Alias kept for backwards compatibility during migration.
    #[verifier::inline]
    pub open spec fn counter_valid(state: CounterState) -> bool {
        counter_wellformed(state)
    }

    /// Signed counter value validity
    ///
    /// Same as counter_valid - the meaningful property is in the saturation
    /// proofs. This is kept for proof structure consistency.
    pub open spec fn signed_counter_valid(state: SignedCounterState) -> bool {
        true  // Type-enforced; see counter_saturation_invariant for real properties
    }

    /// Meaningful counter invariant: saturation semantics are preserved
    ///
    /// This is the real invariant we care about: the counter behaves correctly
    /// under saturating arithmetic operations.
    pub open spec fn counter_saturation_invariant(
        pre: CounterState,
        post: CounterState,
        op: CounterOp,
    ) -> bool {
        match op {
            CounterOp::Add(amount) => {
                // Add saturates at MAX
                if pre.value > u64_max() - amount {
                    post.value == u64_max()
                } else {
                    post.value == pre.value + amount
                }
            }
            CounterOp::Sub(amount) => {
                // Sub saturates at 0
                if amount > pre.value {
                    post.value == 0
                } else {
                    post.value == pre.value - amount
                }
            }
            CounterOp::Cas(expected, new_val) => {
                // CAS: either matches and updates, or state unchanged
                if pre.value == expected {
                    post.value == new_val
                } else {
                    post.value == pre.value  // State unchanged on CAS failure
                }
            }
        }
    }

    /// Counter operation type for invariant checking
    pub enum CounterOp {
        Add(u64),
        Sub(u64),
        Cas(u64, u64),  // (expected, new_value)
    }

    // ========================================================================
    // Invariant: Saturation Bounds (COUNTER-2, COUNTER-3)
    // ========================================================================

    /// COUNTER-2: Unsigned counter never overflows (saturates at MAX)
    pub open spec fn add_saturates_at_max(pre: CounterState, amount: u64) -> bool {
        let post_value = saturating_add_u64(pre.value, amount);
        post_value <= u64_max()
    }

    /// COUNTER-3: Unsigned counter never underflows (saturates at 0)
    ///
    /// This verifies that subtraction saturates correctly:
    /// - If amount > pre.value, result is exactly 0
    /// - Otherwise, result is pre.value - amount
    pub open spec fn sub_saturates_at_zero(pre: CounterState, amount: u64) -> bool {
        let post_value = saturating_sub_u64(pre.value, amount);
        // Verify saturation behavior, not just non-negativity (which is trivial for u64)
        if amount > pre.value {
            post_value == 0  // Must saturate to exactly 0
        } else {
            post_value == pre.value - amount  // Must be exact subtraction
        }
    }

    // ========================================================================
    // Invariant: Monotonicity for Positive Operations (COUNTER-4)
    // ========================================================================

    /// COUNTER-4: Adding a positive amount increases (or saturates)
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
    // Invariant: CAS Semantics (COUNTER-5)
    // ========================================================================

    /// CAS precondition: expected value matches current
    pub open spec fn cas_pre(state: CounterState, expected: u64) -> bool {
        state.value == expected
    }

    /// CAS postcondition: value is updated to new_value
    pub open spec fn cas_post(pre: CounterState, expected: u64, new_value: u64) -> CounterState
        requires cas_pre(pre, expected)
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
        counter_wellformed(state)
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
