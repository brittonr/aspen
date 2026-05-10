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

/// Admission preflight for the production `check_quorum` shell.
pub open spec fn quorum_preflight_allows(
    report_count: u32,
    trusted_count: u32,
    threshold: u32,
) -> bool {
    report_count > 0u32 && trusted_count >= threshold
}

/// Trusted seeder filter: only the `Trusted` trust level contributes to quorum counts.
pub open spec fn trusted_increment(is_trusted: bool) -> u32 {
    if is_trusted { 1u32 } else { 0u32 }
}

/// Saturating tally for one trusted-report classification step.
pub open spec fn trusted_count_after_step(count: u32, is_trusted: bool) -> u32 {
    if is_trusted {
        if count == u32::MAX { u32::MAX } else { (count + 1) as u32 }
    } else {
        count
    }
}

/// Whether a ref's best hash vote count is enough to become canonical.
pub open spec fn ref_vote_reaches_quorum(best_count: u32, threshold: u32) -> bool {
    best_count >= threshold
}

/// Saturating shortfall used by diagnostics: zero means the ref reached threshold.
pub open spec fn vote_shortfall_spec(best_count: u32, threshold: u32) -> u32 {
    if best_count >= threshold { 0u32 } else { (threshold - best_count) as u32 }
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

/// Decide whether the quorum preflight can proceed before per-ref vote tallying.
pub fn quorum_preflight(report_count: u32, trusted_count: u32, threshold: u32) -> (allowed: bool)
    ensures
        allowed == quorum_preflight_allows(report_count, trusted_count, threshold),
        allowed ==> report_count > 0u32,
        allowed ==> trusted_count >= threshold,
        report_count == 0u32 ==> !allowed,
        trusted_count < threshold ==> !allowed,
{
    if report_count == 0 {
        false
    } else if trusted_count < threshold {
        false
    } else {
        true
    }
}

/// Count one report only when it is from a trusted seeder.
pub fn trusted_count_step(count: u32, is_trusted: bool) -> (next: u32)
    ensures
        next == trusted_count_after_step(count, is_trusted),
        !is_trusted ==> next == count,
        is_trusted ==> next >= count,
        next >= count,
        next <= u32::MAX,
{
    if is_trusted {
        if count == u32::MAX {
            u32::MAX
        } else {
            count + 1
        }
    } else {
        count
    }
}

/// Decide whether the best vote count for a ref reaches quorum.
pub fn ref_vote_has_quorum(best_count: u32, threshold: u32) -> (accepted: bool)
    ensures
        accepted == ref_vote_reaches_quorum(best_count, threshold),
        accepted ==> best_count >= threshold,
        !accepted ==> best_count < threshold,
{
    best_count >= threshold
}

/// Compute the diagnostic shortfall from a best vote count to the threshold.
pub fn vote_shortfall(best_count: u32, threshold: u32) -> (shortfall: u32)
    ensures
        shortfall == vote_shortfall_spec(best_count, threshold),
        shortfall == 0u32 <==> best_count >= threshold,
        shortfall > 0u32 ==> best_count < threshold,
{
    if best_count >= threshold {
        0u32
    } else {
        threshold - best_count
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

/// QUORUM-4: Untrusted seeders never increase the trusted tally.
pub proof fn untrusted_report_excluded(count: u32)
    ensures
        trusted_increment(false) == 0u32,
        trusted_count_after_step(count, false) == count,
{
}

/// A trusted report increases a non-saturated tally by exactly one.
pub proof fn trusted_report_increments(count: u32)
    requires count < u32::MAX
    ensures
        trusted_increment(true) == 1u32,
        trusted_count_after_step(count, true) == count + 1,
        trusted_count_after_step(count, true) > count,
{
}

/// Trusted tallying saturates rather than wrapping at u32::MAX.
pub proof fn trusted_count_saturates_at_max()
    ensures trusted_count_after_step(u32::MAX, true) == u32::MAX
{
}

/// Preflight admission is exactly non-empty reports plus enough trusted seeders.
pub proof fn preflight_equivalence(report_count: u32, trusted_count: u32, threshold: u32)
    ensures
        quorum_preflight_allows(report_count, trusted_count, threshold)
            == (report_count > 0u32 && trusted_count >= threshold),
        quorum_preflight_allows(0u32, trusted_count, threshold) == false,
        trusted_count < threshold ==> quorum_preflight_allows(report_count, trusted_count, threshold) == false,
{
}

/// A canonical ref vote cannot be accepted unless it reaches threshold.
pub proof fn accepted_ref_vote_reaches_threshold(best_count: u32, threshold: u32)
    ensures
        ref_vote_reaches_quorum(best_count, threshold) ==> best_count >= threshold,
        !ref_vote_reaches_quorum(best_count, threshold) ==> best_count < threshold,
{
}

/// Vote shortfall is zero exactly when the best vote reaches threshold.
pub proof fn vote_shortfall_zero_iff_quorum(best_count: u32, threshold: u32)
    ensures vote_shortfall_spec(best_count, threshold) == 0u32 <==> best_count >= threshold
{
}

} // verus!
