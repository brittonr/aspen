//! Overflow Constants and Helper Predicates
//!
//! Centralized constants and helper predicates for overflow protection
//! across all Verus specifications in aspen-coordination.
//!
//! # Design Rationale
//!
//! Overflow protection is critical for formal verification. This module
//! provides:
//!
//! 1. **Type-maximum constants** - Named constants for u64, u32, i64 bounds
//! 2. **Overflow check predicates** - Reusable predicates for common patterns
//!
//! Using named constants instead of raw hex literals improves:
//! - Readability: `can_increment_u64(x)` vs `x < 0xFFFF_FFFF_FFFF_FFFFu64`
//! - Maintainability: Single source of truth for constants
//! - Verification: Easier to reason about overflow safety
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/overflow_constants_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Type Maximum Constants
    // ========================================================================

    /// Maximum u64 value (2^64 - 1)
    pub open spec fn U64_MAX() -> u64 {
        0xFFFF_FFFF_FFFF_FFFFu64
    }

    /// Maximum u32 value (2^32 - 1)
    pub open spec fn U32_MAX() -> u32 {
        0xFFFF_FFFFu32
    }

    /// Maximum i64 value (2^63 - 1)
    pub open spec fn I64_MAX() -> i64 {
        0x7FFF_FFFF_FFFF_FFFFi64
    }

    /// Minimum i64 value (-2^63)
    pub open spec fn I64_MIN() -> i64 {
        -0x8000_0000_0000_0000i64
    }

    // ========================================================================
    // u64 Overflow Check Predicates
    // ========================================================================

    /// Check if adding two u64 values would NOT overflow
    ///
    /// Returns true if `a + b <= U64_MAX()`
    pub open spec fn can_add_u64(a: u64, b: u64) -> bool {
        a <= U64_MAX() - b
    }

    /// Check if incrementing a u64 value would NOT overflow
    ///
    /// Returns true if `a + 1 <= U64_MAX()`
    pub open spec fn can_increment_u64(a: u64) -> bool {
        a < U64_MAX()
    }

    /// Check if subtracting two u64 values would NOT underflow
    ///
    /// Returns true if `a >= b`
    pub open spec fn can_sub_u64(a: u64, b: u64) -> bool {
        a >= b
    }

    /// Check if decrementing a u64 value would NOT underflow
    ///
    /// Returns true if `a > 0`
    pub open spec fn can_decrement_u64(a: u64) -> bool {
        a > 0
    }

    // ========================================================================
    // u32 Overflow Check Predicates
    // ========================================================================

    /// Check if adding two u32 values would NOT overflow
    ///
    /// Returns true if `a + b <= U32_MAX()`
    pub open spec fn can_add_u32(a: u32, b: u32) -> bool {
        a <= U32_MAX() - b
    }

    /// Check if incrementing a u32 value would NOT overflow
    ///
    /// Returns true if `a + 1 <= U32_MAX()`
    pub open spec fn can_increment_u32(a: u32) -> bool {
        a < U32_MAX()
    }

    /// Check if subtracting two u32 values would NOT underflow
    ///
    /// Returns true if `a >= b`
    pub open spec fn can_sub_u32(a: u32, b: u32) -> bool {
        a >= b
    }

    /// Check if decrementing a u32 value would NOT underflow
    ///
    /// Returns true if `a > 0`
    pub open spec fn can_decrement_u32(a: u32) -> bool {
        a > 0
    }

    // ========================================================================
    // i64 Overflow Check Predicates
    // ========================================================================

    /// Check if adding two i64 values would NOT overflow
    ///
    /// Returns true if the addition stays within [I64_MIN(), I64_MAX()]
    pub open spec fn can_add_i64(a: i64, b: i64) -> bool {
        if b > 0 {
            a <= I64_MAX() - b
        } else if b < 0 {
            a >= I64_MIN() - b
        } else {
            true  // b == 0, no change
        }
    }

    /// Check if subtracting two i64 values would NOT overflow
    ///
    /// Returns true if the subtraction stays within [I64_MIN(), I64_MAX()]
    pub open spec fn can_sub_i64(a: i64, b: i64) -> bool {
        if b < 0 {
            a <= I64_MAX() + b
        } else if b > 0 {
            a >= I64_MIN() + b
        } else {
            true  // b == 0, no change
        }
    }

    // ========================================================================
    // Proof Helpers
    // ========================================================================

    /// Proof: U64_MAX is the largest u64
    pub proof fn u64_max_is_maximum()
        ensures
            forall |x: u64| x <= U64_MAX(),
    {
        // Follows from u64 type definition
    }

    /// Proof: U32_MAX is the largest u32
    pub proof fn u32_max_is_maximum()
        ensures
            forall |x: u32| x <= U32_MAX(),
    {
        // Follows from u32 type definition
    }

    /// Proof: can_add_u64 implies no overflow
    pub proof fn can_add_u64_sound(a: u64, b: u64)
        requires can_add_u64(a, b)
        ensures (a as int) + (b as int) <= U64_MAX() as int
    {
        // a <= U64_MAX - b
        // a + b <= U64_MAX
    }

    /// Proof: can_increment_u64 implies no overflow
    pub proof fn can_increment_u64_sound(a: u64)
        requires can_increment_u64(a)
        ensures (a as int) + 1 <= U64_MAX() as int
    {
        // a < U64_MAX
        // a + 1 <= U64_MAX
    }

    /// Proof: can_add_u32 implies no overflow
    pub proof fn can_add_u32_sound(a: u32, b: u32)
        requires can_add_u32(a, b)
        ensures (a as int) + (b as int) <= U32_MAX() as int
    {
        // a <= U32_MAX - b
        // a + b <= U32_MAX
    }

    /// Proof: can_increment_u32 implies no overflow
    pub proof fn can_increment_u32_sound(a: u32)
        requires can_increment_u32(a)
        ensures (a as int) + 1 <= U32_MAX() as int
    {
        // a < U32_MAX
        // a + 1 <= U32_MAX
    }
}
