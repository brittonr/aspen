//! Pure functions for three-way merge classification.
//!
//! Formally verified — see `verus/merge_spec.rs` for proofs.
//!
//! These functions classify what action to take for a single entry
//! during a three-way merge (base, ours, theirs). They are deterministic,
//! have no I/O, and use only the entry hashes to decide.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs
//! - No panics or unwraps

/// Classification of a three-way merge for a single entry.
///
/// Given base, ours, and theirs hashes (any of which may be absent),
/// this tells the merge engine what to do with the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreeWayClass {
    /// Entry is unchanged in all three — keep as-is.
    Unchanged,
    /// Entry changed only in ours — take ours.
    TakeOurs,
    /// Entry changed only in theirs — take theirs.
    TakeTheirs,
    /// Both sides made the same change — take either (convergent).
    Convergent,
    /// Both sides made different changes — conflict.
    Conflict,
}

/// Classify a three-way merge for a single entry.
///
/// Each parameter is `Option<[u8; 32]>`:
/// - `Some(hash)` means the entry exists with that content hash
/// - `None` means the entry does not exist in that tree
///
/// # Examples
///
/// ```
/// use aspen_forge::verified::merge::{classify_three_way, ThreeWayClass};
///
/// let h1 = [1u8; 32];
/// let h2 = [2u8; 32];
///
/// // Unchanged: all same
/// assert_eq!(classify_three_way(Some(h1), Some(h1), Some(h1)), ThreeWayClass::Unchanged);
///
/// // Ours changed, theirs didn't
/// assert_eq!(classify_three_way(Some(h1), Some(h2), Some(h1)), ThreeWayClass::TakeOurs);
///
/// // Both changed to different things
/// assert_eq!(classify_three_way(Some(h1), Some(h2), Some([3u8; 32])), ThreeWayClass::Conflict);
/// ```
#[inline]
pub fn classify_three_way(base: Option<[u8; 32]>, ours: Option<[u8; 32]>, theirs: Option<[u8; 32]>) -> ThreeWayClass {
    match (base, ours, theirs) {
        // All three absent — nothing to do
        (None, None, None) => ThreeWayClass::Unchanged,

        // All three present
        (Some(b), Some(o), Some(t)) => {
            if o == b && t == b {
                // Nobody changed it
                ThreeWayClass::Unchanged
            } else if o == b {
                // Only theirs changed
                ThreeWayClass::TakeTheirs
            } else if t == b {
                // Only ours changed
                ThreeWayClass::TakeOurs
            } else if o == t {
                // Both changed to the same thing
                ThreeWayClass::Convergent
            } else {
                // Both changed to different things
                ThreeWayClass::Conflict
            }
        }

        // Base exists, deleted in one or both
        (Some(b), Some(o), None) => {
            if o == b {
                // Theirs deleted, ours unchanged — take theirs (delete)
                ThreeWayClass::TakeTheirs
            } else {
                // Theirs deleted, ours modified — conflict (modify/delete)
                ThreeWayClass::Conflict
            }
        }
        (Some(b), None, Some(t)) => {
            if t == b {
                // Ours deleted, theirs unchanged — take ours (delete)
                ThreeWayClass::TakeOurs
            } else {
                // Ours deleted, theirs modified — conflict (modify/delete)
                ThreeWayClass::Conflict
            }
        }
        (Some(_), None, None) => {
            // Both deleted — convergent deletion
            ThreeWayClass::Convergent
        }

        // Base absent (new file)
        (None, Some(o), Some(t)) => {
            if o == t {
                // Both added the same content
                ThreeWayClass::Convergent
            } else {
                // Both added different content
                ThreeWayClass::Conflict
            }
        }
        (None, Some(_), None) => {
            // Only ours added it
            ThreeWayClass::TakeOurs
        }
        (None, None, Some(_)) => {
            // Only theirs added it
            ThreeWayClass::TakeTheirs
        }
    }
}

/// Check if two changes are convergent (both arrived at the same result).
///
/// Two changes are convergent when both `ours` and `theirs` are equal.
/// Convergent changes never produce conflicts.
#[inline]
pub fn is_convergent_change(ours: Option<[u8; 32]>, theirs: Option<[u8; 32]>) -> bool {
    ours == theirs
}

/// Check if a three-way comparison produces a conflict.
///
/// This is a convenience wrapper over `classify_three_way` that returns
/// `true` only for the `Conflict` classification.
///
/// # Symmetry
///
/// This function is symmetric: `is_conflict(b, o, t) == is_conflict(b, t, o)`.
/// This property is formally verified in `verus/merge_spec.rs`.
#[inline]
pub fn is_conflict(base: Option<[u8; 32]>, ours: Option<[u8; 32]>, theirs: Option<[u8; 32]>) -> bool {
    matches!(classify_three_way(base, ours, theirs), ThreeWayClass::Conflict)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H0: [u8; 32] = [0u8; 32];
    const H1: [u8; 32] = [1u8; 32];
    const H2: [u8; 32] = [2u8; 32];
    const H3: [u8; 32] = [3u8; 32];

    // ====================================================================
    // classify_three_way tests
    // ====================================================================

    #[test]
    fn test_all_absent() {
        assert_eq!(classify_three_way(None, None, None), ThreeWayClass::Unchanged);
    }

    #[test]
    fn test_all_same() {
        assert_eq!(classify_three_way(Some(H1), Some(H1), Some(H1)), ThreeWayClass::Unchanged);
    }

    #[test]
    fn test_only_ours_changed() {
        assert_eq!(classify_three_way(Some(H0), Some(H1), Some(H0)), ThreeWayClass::TakeOurs);
    }

    #[test]
    fn test_only_theirs_changed() {
        assert_eq!(classify_three_way(Some(H0), Some(H0), Some(H1)), ThreeWayClass::TakeTheirs);
    }

    #[test]
    fn test_both_changed_same() {
        assert_eq!(classify_three_way(Some(H0), Some(H1), Some(H1)), ThreeWayClass::Convergent);
    }

    #[test]
    fn test_both_changed_different() {
        assert_eq!(classify_three_way(Some(H0), Some(H1), Some(H2)), ThreeWayClass::Conflict);
    }

    #[test]
    fn test_theirs_deleted_ours_unchanged() {
        assert_eq!(classify_three_way(Some(H0), Some(H0), None), ThreeWayClass::TakeTheirs);
    }

    #[test]
    fn test_theirs_deleted_ours_modified() {
        assert_eq!(classify_three_way(Some(H0), Some(H1), None), ThreeWayClass::Conflict);
    }

    #[test]
    fn test_ours_deleted_theirs_unchanged() {
        assert_eq!(classify_three_way(Some(H0), None, Some(H0)), ThreeWayClass::TakeOurs);
    }

    #[test]
    fn test_ours_deleted_theirs_modified() {
        assert_eq!(classify_three_way(Some(H0), None, Some(H1)), ThreeWayClass::Conflict);
    }

    #[test]
    fn test_both_deleted() {
        assert_eq!(classify_three_way(Some(H0), None, None), ThreeWayClass::Convergent);
    }

    #[test]
    fn test_both_added_same() {
        assert_eq!(classify_three_way(None, Some(H1), Some(H1)), ThreeWayClass::Convergent);
    }

    #[test]
    fn test_both_added_different() {
        assert_eq!(classify_three_way(None, Some(H1), Some(H2)), ThreeWayClass::Conflict);
    }

    #[test]
    fn test_only_ours_added() {
        assert_eq!(classify_three_way(None, Some(H1), None), ThreeWayClass::TakeOurs);
    }

    #[test]
    fn test_only_theirs_added() {
        assert_eq!(classify_three_way(None, None, Some(H1)), ThreeWayClass::TakeTheirs);
    }

    // ====================================================================
    // is_conflict tests
    // ====================================================================

    #[test]
    fn test_is_conflict_true() {
        assert!(is_conflict(Some(H0), Some(H1), Some(H2)));
        assert!(is_conflict(Some(H0), Some(H1), None)); // modify/delete
        assert!(is_conflict(Some(H0), None, Some(H1))); // delete/modify
        assert!(is_conflict(None, Some(H1), Some(H2))); // both added different
    }

    #[test]
    fn test_is_conflict_false() {
        assert!(!is_conflict(Some(H0), Some(H0), Some(H0))); // unchanged
        assert!(!is_conflict(Some(H0), Some(H1), Some(H0))); // only ours
        assert!(!is_conflict(Some(H0), Some(H0), Some(H1))); // only theirs
        assert!(!is_conflict(Some(H0), Some(H1), Some(H1))); // convergent
        assert!(!is_conflict(Some(H0), None, None)); // both deleted
        assert!(!is_conflict(None, Some(H1), Some(H1))); // both added same
        assert!(!is_conflict(None, Some(H1), None)); // only ours added
        assert!(!is_conflict(None, None, Some(H1))); // only theirs added
        assert!(!is_conflict(None, None, None)); // all absent
    }

    // ====================================================================
    // Symmetry tests
    // ====================================================================

    #[test]
    fn test_conflict_symmetry() {
        // is_conflict(b, o, t) == is_conflict(b, t, o) for all combinations
        let hashes: Vec<Option<[u8; 32]>> = vec![None, Some(H0), Some(H1), Some(H2), Some(H3)];

        for base in &hashes {
            for ours in &hashes {
                for theirs in &hashes {
                    assert_eq!(
                        is_conflict(*base, *ours, *theirs),
                        is_conflict(*base, *theirs, *ours),
                        "symmetry violated for base={:?}, ours={:?}, theirs={:?}",
                        base.map(|h| h[0]),
                        ours.map(|h| h[0]),
                        theirs.map(|h| h[0]),
                    );
                }
            }
        }
    }

    #[test]
    fn test_classify_symmetry() {
        // If classify returns Conflict for (b, o, t), it must also return Conflict for (b, t, o)
        // If it returns TakeOurs for (b, o, t), it must return TakeTheirs for (b, t, o)
        let hashes: Vec<Option<[u8; 32]>> = vec![None, Some(H0), Some(H1), Some(H2)];

        for base in &hashes {
            for ours in &hashes {
                for theirs in &hashes {
                    let fwd = classify_three_way(*base, *ours, *theirs);
                    let rev = classify_three_way(*base, *theirs, *ours);

                    match fwd {
                        ThreeWayClass::Unchanged => assert_eq!(rev, ThreeWayClass::Unchanged),
                        ThreeWayClass::Convergent => assert_eq!(rev, ThreeWayClass::Convergent),
                        ThreeWayClass::Conflict => assert_eq!(rev, ThreeWayClass::Conflict),
                        ThreeWayClass::TakeOurs => assert_eq!(rev, ThreeWayClass::TakeTheirs),
                        ThreeWayClass::TakeTheirs => assert_eq!(rev, ThreeWayClass::TakeOurs),
                    }
                }
            }
        }
    }

    // ====================================================================
    // is_convergent_change tests
    // ====================================================================

    #[test]
    fn test_convergent_both_none() {
        assert!(is_convergent_change(None, None));
    }

    #[test]
    fn test_convergent_both_same() {
        assert!(is_convergent_change(Some(H1), Some(H1)));
    }

    #[test]
    fn test_not_convergent_different() {
        assert!(!is_convergent_change(Some(H1), Some(H2)));
    }

    #[test]
    fn test_not_convergent_one_none() {
        assert!(!is_convergent_change(Some(H1), None));
        assert!(!is_convergent_change(None, Some(H1)));
    }

    // ====================================================================
    // Convergent never conflicts
    // ====================================================================

    #[test]
    fn test_convergent_never_conflicts() {
        let hashes: Vec<Option<[u8; 32]>> = vec![None, Some(H0), Some(H1), Some(H2)];

        for base in &hashes {
            for val in &hashes {
                // When ours == theirs, there should never be a conflict
                if is_convergent_change(*val, *val) {
                    assert!(
                        !is_conflict(*base, *val, *val),
                        "convergent change produced conflict for base={:?}, val={:?}",
                        base.map(|h| h[0]),
                        val.map(|h| h[0]),
                    );
                }
            }
        }
    }

    // ====================================================================
    // Unchanged never conflicts
    // ====================================================================

    #[test]
    fn test_unchanged_never_conflicts() {
        let hashes: Vec<Option<[u8; 32]>> = vec![None, Some(H0), Some(H1)];

        for val in &hashes {
            // When base == ours == theirs, never conflict
            assert!(
                !is_conflict(*val, *val, *val),
                "unchanged entry produced conflict for val={:?}",
                val.map(|h| h[0]),
            );
        }
    }

    // ====================================================================
    // Exhaustiveness: every input produces exactly one classification
    // ====================================================================

    #[test]
    fn test_classification_exhaustive() {
        let hashes: Vec<Option<[u8; 32]>> = vec![None, Some(H0), Some(H1), Some(H2)];

        for base in &hashes {
            for ours in &hashes {
                for theirs in &hashes {
                    let class = classify_three_way(*base, *ours, *theirs);
                    // Every input must map to one of the five variants
                    assert!(
                        matches!(
                            class,
                            ThreeWayClass::Unchanged
                                | ThreeWayClass::TakeOurs
                                | ThreeWayClass::TakeTheirs
                                | ThreeWayClass::Convergent
                                | ThreeWayClass::Conflict
                        ),
                        "unexpected classification for base={:?}, ours={:?}, theirs={:?}: {:?}",
                        base.map(|h| h[0]),
                        ours.map(|h| h[0]),
                        theirs.map(|h| h[0]),
                        class,
                    );
                }
            }
        }
    }
}
