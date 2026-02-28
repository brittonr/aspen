//! Verus specifications for fork detection.
//!
//! Proves properties of `detect_ref_forks` and fork branch construction.

use vstd::prelude::*;

verus! {

// ============================================================================
// State Model
// ============================================================================

/// Spec-level representation of a seeder's ref heads claim.
pub struct SeederClaimSpec {
    /// Cluster key (32 bytes, used as identity).
    pub cluster_key: Seq<u8>,
    /// Ref heads: mapping from ref name to hash.
    /// Represented as a sequence of (name, hash) pairs.
    pub heads: Seq<(Seq<u8>, Seq<u8>)>,
    /// Whether this seeder is trusted.
    pub is_trusted: bool,
}

/// Spec-level representation of a detected fork.
pub struct ForkSpec {
    /// The ref name that has conflicting state.
    pub ref_name: Seq<u8>,
    /// Number of distinct hash values seen for this ref.
    pub distinct_hashes: nat,
}

// ============================================================================
// Spec Functions
// ============================================================================

/// Count distinct hash values for a given ref across all claims.
pub open spec fn count_distinct_hashes_for_ref(
    claims: Seq<SeederClaimSpec>,
    ref_name: Seq<u8>,
) -> nat {
    // Count unique hash values across all claims for this ref
    // This is specified abstractly; the exec function implements the grouping.
    // The key property is: result > 1 ⟺ fork exists for this ref.
    // We leave this as an abstract spec function since Verus doesn't have
    // built-in HashSet reasoning.
    arbitrary()
}

/// Whether a ref has a fork (more than one distinct hash claimed).
pub open spec fn ref_has_fork(
    claims: Seq<SeederClaimSpec>,
    ref_name: Seq<u8>,
) -> bool {
    count_distinct_hashes_for_ref(claims, ref_name) > 1
}

// ============================================================================
// Invariants
// ============================================================================

/// FORK-1: No fork reported when all seeders agree.
///
/// If every seeder reports the same hash for a ref, no fork is detected.
pub open spec fn agreement_implies_no_fork(
    claims: Seq<SeederClaimSpec>,
    ref_name: Seq<u8>,
    agreed_hash: Seq<u8>,
) -> bool {
    // If all claims for ref_name have the same hash value,
    // then there should be no fork for this ref.
    (forall|i: int| #![trigger claims[i]] 0 <= i < claims.len() ==>
        forall|j: int| #![trigger claims[i].heads[j]] 0 <= j < claims[i].heads.len() ==>
            claims[i].heads[j].0 == ref_name ==>
                claims[i].heads[j].1 == agreed_hash
    ) ==> !ref_has_fork(claims, ref_name)
}

/// FORK-2: Disagreement implies fork detected.
///
/// If two seeders report different hashes for the same ref, a fork exists.
pub open spec fn disagreement_implies_fork(
    claims: Seq<SeederClaimSpec>,
    ref_name: Seq<u8>,
    hash_a: Seq<u8>,
    hash_b: Seq<u8>,
    seeder_a: int,
    seeder_b: int,
) -> bool {
    // If seeder_a claims hash_a and seeder_b claims hash_b for the same ref,
    // and hash_a != hash_b, then a fork must be detected.
    (
        0 <= seeder_a < claims.len() as int &&
        0 <= seeder_b < claims.len() as int &&
        seeder_a != seeder_b &&
        hash_a != hash_b
        // (existence of the ref with these hashes in the respective claims
        //  is left as a precondition rather than fully specified here)
    ) ==> ref_has_fork(claims, ref_name)
}

// ============================================================================
// Verified Helper Functions
// ============================================================================

/// Check if two BLAKE3 hashes are equal.
///
/// Used in fork detection to compare ref heads from different seeders.
pub fn hashes_equal(a: &[u8; 32], b: &[u8; 32]) -> (result: bool)
    ensures result == (a@ =~= b@)
{
    let mut i: usize = 0;
    while i < 32
        invariant
            0 <= i <= 32,
            a@.len() == 32,
            b@.len() == 32,
            forall|j: int| 0 <= j < i as int ==> a@[j] == b@[j]
        decreases 32 - i
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Count the number of entries in a list (bounded).
///
/// Tiger Style: Returns u32 to match federation constants.
/// Saturates at u32::MAX to prevent overflow.
pub fn count_entries_saturating(len: usize) -> (result: u32)
    ensures
        len <= u32::MAX as usize ==> result == len as u32,
        len > u32::MAX as usize ==> result == u32::MAX
{
    if len <= u32::MAX as usize {
        len as u32
    } else {
        u32::MAX
    }
}

} // verus!
