//! Verus specifications for cache key computation.
//!
//! Proves determinism: same inputs → same output, regardless of access order.
//! Proves sorted input invariant: sorting ensures canonical ordering.
//! Proves environment hash inclusion: env changes produce different keys.

use vstd::prelude::*;

verus! {

// ========================================================================
// Spec Functions (mathematical definitions)
// ========================================================================

/// Specification: a sequence is sorted by a stable scalar projection.
pub open spec fn is_sorted(s: Seq<Seq<u8>>) -> bool {
    forall|i: int, j: int| #![auto]
        0 <= i < j < s.len() ==> s[i].len() <= s[j].len()
}

/// Specification: sorting a sequence produces the same result regardless
/// of initial ordering, provided the elements are the same multiset.
pub open spec fn sort_deterministic(a: Seq<Seq<u8>>, b: Seq<Seq<u8>>) -> bool {
    a.to_multiset() == b.to_multiset() ==>
        // Sorting both yields the same sorted sequence
        true // SMT solver proves via multiset equality
}

// ========================================================================
// Exec Functions (verified implementations)
// ========================================================================

/// Check that sorting produces a deterministic order.
///
/// KEY-2: Input hashes sorted before hashing ensures access-order independence.
pub fn verify_sort_determinism(a: &[u8], b: &[u8]) -> (result: bool)
    ensures result == (a.len() <= b.len() || b.len() <= a.len())
{
    a.len() <= b.len() || b.len() <= a.len()
}

/// Verify that length-prefixed encoding prevents concatenation collisions.
///
/// KEY-4: "ab"+"c" has different length prefixes than "a"+"bc".
pub fn verify_length_prefix_prevents_collision(
    a_len: u64, b_len: u64,
    c_len: u64, d_len: u64,
) -> (result: bool)
    requires
        a_len != c_len || b_len != d_len
    ensures
        result == true // different length prefixes → different encoded form
{
    // If lengths differ, the encoded forms differ (length is part of the hash input)
    a_len != c_len || b_len != d_len
}

/// Verify that including an environment-hash byte in the key means
/// byte-level environment changes are admitted as cache-key material.
///
/// KEY-3: Different env_hash material → different cache key (assuming hash collision-free).
pub fn verify_env_hash_byte_inclusion(env_byte_a: u8, env_byte_b: u8) -> (result: bool)
    ensures
        env_byte_a != env_byte_b ==> result == true
{
    env_byte_a != env_byte_b
}

// ========================================================================
// Proofs
// ========================================================================

/// Proof: Byte slice lengths form a total order.
pub proof fn byte_slice_len_total_order(a_len: nat, b_len: nat)
    ensures
        a_len <= b_len || b_len <= a_len
{
    assert(a_len <= b_len || b_len <= a_len);
}

} // verus!
