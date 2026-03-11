//! Overflow Constants and Helper Predicates for Deploy Specs
//!
//! Provides reusable predicates for u32 overflow protection in quorum
//! calculations. Mirrors the pattern from aspen-coordination/verus/.

use vstd::prelude::*;

verus! {
    /// Maximum u32 value (2^32 - 1)
    pub open spec fn U32_MAX() -> u32 {
        0xFFFF_FFFFu32
    }

    /// Check if adding two u32 values would NOT overflow
    pub open spec fn can_add_u32(a: u32, b: u32) -> bool {
        a <= U32_MAX() - b
    }

    /// Check if incrementing a u32 value would NOT overflow
    pub open spec fn can_increment_u32(a: u32) -> bool {
        a < U32_MAX()
    }

    /// Check if subtracting two u32 values would NOT underflow
    pub open spec fn can_sub_u32(a: u32, b: u32) -> bool {
        a >= b
    }

    /// Proof: can_add_u32 implies no overflow
    #[verifier(external_body)]
    pub proof fn can_add_u32_sound(a: u32, b: u32)
        requires can_add_u32(a, b)
        ensures (a as int) + (b as int) <= U32_MAX() as int
    {
    }

    /// Proof: can_increment_u32 implies no overflow
    #[verifier(external_body)]
    pub proof fn can_increment_u32_sound(a: u32)
        requires can_increment_u32(a)
        ensures (a as int) + 1 <= U32_MAX() as int
    {
    }
}
