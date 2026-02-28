//! Verus specifications for seeder quorum verification.
//!
//! Proves properties of `calculate_seeder_quorum` and quorum checking logic.

use vstd::prelude::*;

verus! {

// ============================================================================
// Spec Functions (mathematical definitions)
// ============================================================================

/// Mathematical definition of seeder quorum: majority formula.
pub open spec fn quorum_spec(total: u32) -> u32 {
    if total == 0 {
        1u32
    } else {
        ((total / 2) + 1) as u32
    }
}

/// Whether a quorum check would succeed given vote counts.
/// For each ref, at least `threshold` seeders must agree on the same hash.
pub open spec fn quorum_met(
    max_votes_per_ref: Seq<u32>,
    threshold: u32,
) -> bool {
    forall|i: int| 0 <= i < max_votes_per_ref.len() ==>
        max_votes_per_ref[i] >= threshold
}

// ============================================================================
// Exec Functions (verified implementations)
// ============================================================================

/// Calculate quorum size for a given number of trusted seeders.
///
/// Ensures:
/// - Result is always >= 1 (QUORUM-1)
/// - Result equals mathematical majority formula (QUORUM-2)
/// - Result is at most total (never requires more than all seeders)
pub fn calculate_seeder_quorum(total_trusted_seeders: u32) -> (result: u32)
    ensures
        result >= 1u32,
        result == quorum_spec(total_trusted_seeders),
        total_trusted_seeders > 0u32 ==> result <= total_trusted_seeders,
{
    if total_trusted_seeders == 0 {
        1u32
    } else {
        // (total / 2) + 1 is always <= total for total >= 1:
        //   total=1: (0)+1 = 1 <= 1 ✓
        //   total=2: (1)+1 = 2 <= 2 ✓
        //   total=3: (1)+1 = 2 <= 3 ✓
        //   total=n: (n/2)+1 <= n for n >= 1 ✓
        (total_trusted_seeders / 2) + 1
    }
}

// ============================================================================
// Invariant Proofs
// ============================================================================

/// QUORUM-1: Quorum threshold is always at least 1.
pub proof fn quorum_always_at_least_one(total: u32)
    ensures quorum_spec(total) >= 1u32
{
    // Trivially true from the definition:
    // - total == 0 → returns 1
    // - total > 0 → (total/2) + 1 >= 0 + 1 = 1
}

/// QUORUM-3: Monotonicity — more seeders means same or higher quorum.
///
/// This ensures that adding seeders never reduces the quorum requirement.
pub proof fn quorum_monotonic(a: u32, b: u32)
    requires a <= b
    ensures quorum_spec(a) <= quorum_spec(b)
{
    // When a == 0, quorum_spec(a) == 1, and quorum_spec(b) >= 1
    // When a > 0 and b > 0:
    //   a <= b ==> a/2 <= b/2 ==> a/2 + 1 <= b/2 + 1
}

/// QUORUM-2: Quorum is strict majority for non-zero counts.
///
/// For N > 0 trusted seeders, quorum > N/2 (strict majority).
pub proof fn quorum_is_strict_majority(total: u32)
    requires total > 0u32
    ensures quorum_spec(total) as int > (total as int) / 2
{
    // (total/2) + 1 > total/2 is trivially true (adding 1)
}

/// Quorum never exceeds total seeders.
pub proof fn quorum_bounded_by_total(total: u32)
    requires total > 0u32
    ensures quorum_spec(total) <= total
{
    // For n >= 1: (n/2) + 1 <= n
    // Equivalently: 1 <= n - n/2 = ceil(n/2)
    // Which holds for all n >= 1.
}

} // verus!
