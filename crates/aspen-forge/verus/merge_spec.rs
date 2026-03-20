//! Verus specifications for three-way merge classification.
//!
//! Proves correctness properties of the merge predicates in
//! `src/verified/merge.rs`:
//!
//! - MERGE-1: Conflict detection symmetry
//! - MERGE-2: Convergent changes never conflict
//! - MERGE-3: Unchanged entries never conflict
//! - MERGE-4: Classification exhaustiveness

use vstd::prelude::*;

verus! {

// ========================================================================
// Types
// ========================================================================

/// Hash value (simplified to u64 for verification).
pub type Hash = u64;

/// Classification of a three-way merge for a single entry.
#[derive(PartialEq, Eq)]
pub enum ThreeWayClass {
    Unchanged,
    TakeOurs,
    TakeTheirs,
    Convergent,
    Conflict,
}

// ========================================================================
// Spec Functions (mathematical definitions)
// ========================================================================

/// Spec-level three-way classification.
pub open spec fn classify_spec(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
) -> ThreeWayClass {
    match (base, ours, theirs) {
        (None, None, None) => ThreeWayClass::Unchanged,
        (Some(b), Some(o), Some(t)) => {
            if o == b && t == b {
                ThreeWayClass::Unchanged
            } else if o == b {
                ThreeWayClass::TakeTheirs
            } else if t == b {
                ThreeWayClass::TakeOurs
            } else if o == t {
                ThreeWayClass::Convergent
            } else {
                ThreeWayClass::Conflict
            }
        }
        (Some(b), Some(o), None) => {
            if o == b {
                ThreeWayClass::TakeTheirs
            } else {
                ThreeWayClass::Conflict
            }
        }
        (Some(b), None, Some(t)) => {
            if t == b {
                ThreeWayClass::TakeOurs
            } else {
                ThreeWayClass::Conflict
            }
        }
        (Some(_), None, None) => ThreeWayClass::Convergent,
        (None, Some(o), Some(t)) => {
            if o == t {
                ThreeWayClass::Convergent
            } else {
                ThreeWayClass::Conflict
            }
        }
        (None, Some(_), None) => ThreeWayClass::TakeOurs,
        (None, None, Some(_)) => ThreeWayClass::TakeTheirs,
    }
}

/// Spec-level conflict check.
pub open spec fn is_conflict_spec(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
) -> bool {
    classify_spec(base, ours, theirs) == ThreeWayClass::Conflict
}

/// Spec-level convergence check.
pub open spec fn is_convergent_spec(
    ours: Option<Hash>,
    theirs: Option<Hash>,
) -> bool {
    ours == theirs
}

// ========================================================================
// Exec Functions (verified implementations)
// ========================================================================

/// Classify a three-way merge for a single entry.
pub fn classify_three_way(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
) -> (result: ThreeWayClass)
    ensures result == classify_spec(base, ours, theirs)
{
    match (base, ours, theirs) {
        (None, None, None) => ThreeWayClass::Unchanged,
        (Some(b), Some(o), Some(t)) => {
            if o == b && t == b {
                ThreeWayClass::Unchanged
            } else if o == b {
                ThreeWayClass::TakeTheirs
            } else if t == b {
                ThreeWayClass::TakeOurs
            } else if o == t {
                ThreeWayClass::Convergent
            } else {
                ThreeWayClass::Conflict
            }
        }
        (Some(b), Some(o), None) => {
            if o == b {
                ThreeWayClass::TakeTheirs
            } else {
                ThreeWayClass::Conflict
            }
        }
        (Some(b), None, Some(t)) => {
            if t == b {
                ThreeWayClass::TakeOurs
            } else {
                ThreeWayClass::Conflict
            }
        }
        (Some(_), None, None) => ThreeWayClass::Convergent,
        (None, Some(o), Some(t)) => {
            if o == t {
                ThreeWayClass::Convergent
            } else {
                ThreeWayClass::Conflict
            }
        }
        (None, Some(_), None) => ThreeWayClass::TakeOurs,
        (None, None, Some(_)) => ThreeWayClass::TakeTheirs,
    }
}

/// Check if a three-way comparison is a conflict.
pub fn is_conflict(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
) -> (result: bool)
    ensures result == is_conflict_spec(base, ours, theirs)
{
    let class = classify_three_way(base, ours, theirs);
    class == ThreeWayClass::Conflict
}

/// Check if two changes are convergent.
pub fn is_convergent_change(
    ours: Option<Hash>,
    theirs: Option<Hash>,
) -> (result: bool)
    ensures result == is_convergent_spec(ours, theirs)
{
    ours == theirs
}

// ========================================================================
// Proofs
// ========================================================================

/// MERGE-1: Conflict detection is symmetric.
///
/// is_conflict(b, o, t) == is_conflict(b, t, o) for all inputs.
pub proof fn conflict_symmetry(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
)
    ensures is_conflict_spec(base, ours, theirs)
         == is_conflict_spec(base, theirs, ours)
{
    // SMT solver proves by case analysis on (base, ours, theirs)
}

/// MERGE-2: Convergent changes never produce conflicts.
///
/// When ours == theirs, the result is never Conflict.
pub proof fn convergent_never_conflicts(
    base: Option<Hash>,
    val: Option<Hash>,
)
    ensures !is_conflict_spec(base, val, val)
{
    // SMT solver proves by case analysis on (base, val)
}

/// MERGE-3: Unchanged entries never produce conflicts.
///
/// When base == ours == theirs, the result is Unchanged.
pub proof fn unchanged_never_conflicts(
    val: Option<Hash>,
)
    ensures classify_spec(val, val, val) == ThreeWayClass::Unchanged
{
    // SMT solver proves directly
}

/// MERGE-4: Classification is exhaustive — every input maps to
/// exactly one of the five variants.
///
/// We prove this by showing that classify_spec always returns
/// a valid ThreeWayClass (which is guaranteed by the match arms
/// covering all cases). The interesting property is that no input
/// combination is unclassified.
pub proof fn classification_total(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
)
    ensures {
        let c = classify_spec(base, ours, theirs);
        c == ThreeWayClass::Unchanged
        || c == ThreeWayClass::TakeOurs
        || c == ThreeWayClass::TakeTheirs
        || c == ThreeWayClass::Convergent
        || c == ThreeWayClass::Conflict
    }
{
    // Trivially true by match exhaustiveness
}

/// Classification symmetry: TakeOurs and TakeTheirs swap.
///
/// If classify(b, o, t) == TakeOurs, then classify(b, t, o) == TakeTheirs.
pub proof fn classify_swap_symmetry(
    base: Option<Hash>,
    ours: Option<Hash>,
    theirs: Option<Hash>,
)
    ensures {
        let fwd = classify_spec(base, ours, theirs);
        let rev = classify_spec(base, theirs, ours);
        (fwd == ThreeWayClass::TakeOurs ==> rev == ThreeWayClass::TakeTheirs)
        && (fwd == ThreeWayClass::TakeTheirs ==> rev == ThreeWayClass::TakeOurs)
        && (fwd == ThreeWayClass::Unchanged ==> rev == ThreeWayClass::Unchanged)
        && (fwd == ThreeWayClass::Convergent ==> rev == ThreeWayClass::Convergent)
        && (fwd == ThreeWayClass::Conflict ==> rev == ThreeWayClass::Conflict)
    }
{
    // SMT solver proves by case analysis
}

} // verus!
