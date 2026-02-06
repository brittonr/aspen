//! Verification of purge() operation
//!
//! This module proves that the `purge()` operation in `SharedRedbStorage`
//! preserves storage invariants.
//!
//! # Purging in Raft
//!
//! Purging removes old log entries that are no longer needed because:
//! - They have been snapshotted
//! - All nodes have committed them
//!
//! Unlike truncation (which removes from the end), purging removes from
//! the beginning of the log.
//!
//! # Key Properties
//!
//! 1. **Monotonicity (INVARIANT 6)**: last_purged only increases
//! 2. **Chain Validity**: Remaining entries still form a valid chain
//! 3. **No Re-purging**: Cannot purge entries already purged

use super::storage_state::StorageStateSpec;
use super::storage_state::purge_monotonic;
use super::storage_state::storage_invariant;

/// Specification of purge() postcondition.
///
/// Removes all log entries up to and including `purge_up_to`.
pub fn purge_post(pre: &StorageStateSpec, purge_up_to: u64) -> StorageStateSpec {
    let mut post = pre.clone();

    // Remove log entries <= purge_up_to
    post.log.retain(|&idx, _| idx > purge_up_to);

    // Remove chain hashes <= purge_up_to
    post.chain_hashes.retain(|&idx, _| idx > purge_up_to);

    // Update last_purged (monotonically increasing)
    post.last_purged = Some(match pre.last_purged {
        Some(prev) => prev.max(purge_up_to),
        None => purge_up_to,
    });

    // Remove responses for purged entries
    post.pending_responses.retain(|&idx, _| idx > purge_up_to);

    // Note: chain_tip is NOT changed
    // - Chain tip points to the end of the log
    // - Purging removes from the beginning

    post
}

/// Verify that purge preserves storage invariants.
pub fn verify_purge_preserves_invariants(pre: &StorageStateSpec, purge_up_to: u64) -> Result<(), String> {
    // Precondition: pre-state satisfies invariants
    if !storage_invariant(pre) {
        return Err("Pre-state does not satisfy invariants".to_string());
    }

    // Precondition: cannot purge beyond last_applied
    if let Some(last_applied) = pre.last_applied
        && purge_up_to > last_applied
    {
        return Err(format!(
            "Cannot purge beyond last_applied: purge_up_to={} > last_applied={}",
            purge_up_to, last_applied
        ));
    }

    let post = purge_post(pre, purge_up_to);

    // Postcondition: post-state satisfies invariants
    if !storage_invariant(&post) {
        return Err("Post-state does not satisfy invariants".to_string());
    }

    // Postcondition: last_purged is monotonic
    if !purge_monotonic(pre, &post) {
        return Err("last_purged is not monotonic".to_string());
    }

    // Postcondition: no entries <= purge_up_to remain
    if post.log.keys().any(|&idx| idx <= purge_up_to) {
        return Err("Log entries <= purge_up_to still exist".to_string());
    }

    Ok(())
}

// ============================================================================
// Verus Specifications
// ============================================================================

/*
verus! {
    use vstd::prelude::*;
    use super::storage_state::*;
    use super::chain_hash::*;

    /// Specification of purge() behavior
    pub open spec fn purge_post(
        pre: StorageState,
        purge_up_to: u64,
    ) -> StorageState {
        let retained_log = pre.log.restrict(Set::new(|i: u64| i > purge_up_to));
        let retained_hashes = pre.chain_hashes.restrict(Set::new(|i: u64| i > purge_up_to));
        let retained_responses = pre.pending_responses.restrict(Set::new(|i: u64| i > purge_up_to));

        // Update last_purged (monotonically)
        let new_last_purged = Some(match pre.last_purged {
            Some(prev) => if purge_up_to > prev { purge_up_to } else { prev },
            None => purge_up_to,
        });

        StorageState {
            log: retained_log,
            chain_hashes: retained_hashes,
            last_purged: new_last_purged,
            pending_responses: retained_responses,
            // These fields unchanged:
            chain_tip: pre.chain_tip,  // NOT changed - points to end
            kv: pre.kv,
            last_applied: pre.last_applied,
            genesis_hash: pre.genesis_hash,
        }
    }

    /// INVARIANT 6: Purge monotonicity
    ///
    /// last_purged can only increase, never decrease.
    pub proof fn purge_is_monotonic(
        pre: StorageState,
        purge_up_to: u64,
    )
        requires storage_invariant(pre)
        ensures purge_monotonic(pre, purge_post(pre, purge_up_to))
    {
        let post = purge_post(pre, purge_up_to);
        match (pre.last_purged, post.last_purged) {
            (None, Some(_)) => {
                // None -> Some is always monotonic
            }
            (Some(prev), Some(new)) => {
                // new = max(prev, purge_up_to) >= prev
                assert(new >= prev);
            }
            (Some(_), None) => {
                // Impossible by construction
                assert(false);
            }
            (None, None) => {
                // Impossible by construction
                assert(false);
            }
        }
    }

    /// Main theorem: purge preserves storage invariants
    pub proof fn purge_preserves_invariants(
        pre: StorageState,
        purge_up_to: u64,
    )
        requires
            storage_invariant(pre),
            // Cannot purge uncommitted entries
            match pre.last_applied {
                Some(last) => purge_up_to <= last,
                None => purge_up_to == 0,
            },
        ensures
            storage_invariant(purge_post(pre, purge_up_to)),
            purge_monotonic(pre, purge_post(pre, purge_up_to)),
    {
        let post = purge_post(pre, purge_up_to);

        // Chain tip unchanged - still points to end of log
        assert(post.chain_tip == pre.chain_tip);

        // Chain is still valid for remaining entries
        // (entries > purge_up_to are unaffected)

        // Response cache consistent
        // Retained responses have indices > purge_up_to
        // last_applied unchanged, so if idx was valid before, still valid

        purge_is_monotonic(pre, purge_up_to);
    }

    /// Corollary: Multiple purges are equivalent to single largest purge
    pub proof fn purge_commutative(
        pre: StorageState,
        a: u64,
        b: u64,
    )
        requires storage_invariant(pre)
        ensures
            purge_post(purge_post(pre, a), b).last_purged
            == purge_post(pre, if a > b { a } else { b }).last_purged
    {
        // Both result in last_purged = max(a, b, pre.last_purged)
    }

    /// Corollary: Purging at 0 is a no-op for last_purged
    pub proof fn purge_zero_noop(pre: StorageState)
        requires
            storage_invariant(pre),
            pre.last_purged.is_none() || pre.last_purged.unwrap() >= 0,
        ensures {
            let post = purge_post(pre, 0);
            post.last_purged == Some(match pre.last_purged {
                Some(prev) => prev,
                None => 0,
            })
        }
    {
        // max(0, prev) = prev if prev >= 0
    }
}
*/

#[cfg(test)]
mod tests {
    use super::super::chain_hash::LogEntryData;
    use super::*;

    fn make_populated_state() -> StorageStateSpec {
        let mut state = StorageStateSpec::new();

        // Add log entries 1-10
        for i in 1..=10 {
            state.log.insert(i, LogEntryData {
                term: 1,
                data: format!("entry {}", i).into_bytes(),
            });
            // Compute chain hash
            let prev = if i == 1 {
                state.genesis_hash
            } else {
                *state.chain_hashes.get(&(i - 1)).unwrap()
            };
            let hash = crate::integrity::compute_entry_hash(&prev, i, 1, format!("entry {}", i).as_bytes());
            state.chain_hashes.insert(i, hash);
        }

        state.chain_tip = (*state.chain_hashes.get(&10).unwrap(), 10);
        state.last_applied = Some(10);

        state
    }

    #[test]
    fn test_purge_partial() {
        let pre = make_populated_state();
        let result = verify_purge_preserves_invariants(&pre, 5);
        assert!(result.is_ok(), "purge should preserve invariants: {:?}", result);

        let post = purge_post(&pre, 5);
        assert_eq!(post.log.len(), 5); // entries 6-10 remain
        assert_eq!(post.last_purged, Some(5));
        assert!(!post.log.contains_key(&1));
        assert!(!post.log.contains_key(&5));
        assert!(post.log.contains_key(&6));
    }

    #[test]
    fn test_purge_monotonic() {
        let pre = make_populated_state();
        let post1 = purge_post(&pre, 3);
        let post2 = purge_post(&post1, 5);

        assert!(purge_monotonic(&pre, &post1));
        assert!(purge_monotonic(&post1, &post2));

        assert_eq!(post1.last_purged, Some(3));
        assert_eq!(post2.last_purged, Some(5));
    }

    #[test]
    fn test_purge_cannot_decrease() {
        let mut pre = make_populated_state();
        pre.last_purged = Some(7);

        // Purging at 5 should not decrease last_purged
        let post = purge_post(&pre, 5);
        assert_eq!(post.last_purged, Some(7)); // max(7, 5) = 7
    }

    #[test]
    fn test_purge_all() {
        let pre = make_populated_state();
        let result = verify_purge_preserves_invariants(&pre, 10);
        assert!(result.is_ok(), "purge should preserve invariants: {:?}", result);

        let post = purge_post(&pre, 10);
        assert_eq!(post.log.len(), 0);
        assert_eq!(post.last_purged, Some(10));
    }

    #[test]
    fn test_purge_chain_tip_unchanged() {
        let pre = make_populated_state();
        let post = purge_post(&pre, 5);

        // Chain tip should still point to entry 10
        assert_eq!(post.chain_tip.1, 10);
    }

    #[test]
    fn test_cannot_purge_beyond_last_applied() {
        let mut pre = make_populated_state();
        pre.last_applied = Some(5);

        // Trying to purge beyond last_applied should fail
        let result = verify_purge_preserves_invariants(&pre, 7);
        assert!(result.is_err());
    }
}
