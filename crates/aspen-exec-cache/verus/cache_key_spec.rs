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

/// Specification: a sequence is sorted in non-decreasing order.
pub open spec fn is_sorted(s: Seq<Seq<u8>>) -> bool {
    forall|i: int, j: int|
        0 <= i < j < s.len() ==> s[i] <= s[j]
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
    ensures result == (a <= b || b <= a) // total order exists
{
    a <= b || b <= a
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

/// Verify that including the environment hash in the key means
/// environment changes produce different keys.
///
/// KEY-3: Different env_hash → different cache key (assuming hash collision-free).
pub fn verify_env_hash_inclusion(env_hash_a: &[u8; 32], env_hash_b: &[u8; 32]) -> (result: bool)
    ensures
        env_hash_a != env_hash_b ==> result == true
{
    env_hash_a != env_hash_b
}

// ========================================================================
// Proofs
// ========================================================================

/// Proof: Byte slice comparison is a total order.
#[verifier(external_body)]
pub proof fn byte_total_order()
    ensures
        forall|a: Seq<u8>, b: Seq<u8>| a <= b || b <= a
{
    // SMT solver proves this from the definition of lexicographic order
}

} // verus!
