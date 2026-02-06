//! Purge Operation Specifications
//!
//! Proves that the purge() operation preserves storage invariants.
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
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/purge_spec.rs
//! ```

use vstd::prelude::*;

use super::chain_hash_spec::*;
use super::storage_state_spec::*;

verus! {
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
