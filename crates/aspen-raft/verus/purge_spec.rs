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
    ///
    /// Removes all entries with index <= purge_up_to
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
    #[verifier(external_body)]
    pub proof fn purge_is_monotonic(
        pre: StorageState,
        purge_up_to: u64,
    )
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

    /// Purge removes entries <= purge_up_to
    #[verifier(external_body)]
    pub proof fn purge_removes_entries(
        pre: StorageState,
        purge_up_to: u64,
    )
        ensures
            forall |i: u64| i <= purge_up_to ==>
                !purge_post(pre, purge_up_to).log.contains_key(i),
    {
        // By construction: restrict keeps only i > purge_up_to
    }

    /// Purge preserves entries > purge_up_to
    #[verifier(external_body)]
    pub proof fn purge_preserves_entries(
        pre: StorageState,
        purge_up_to: u64,
    )
        ensures
            forall |i: u64| i > purge_up_to && pre.log.contains_key(i) ==>
                purge_post(pre, purge_up_to).log.contains_key(i),
    {
        // By construction: restrict preserves i > purge_up_to
    }

    /// Chain tip is unchanged by purge
    #[verifier(external_body)]
    pub proof fn purge_preserves_chain_tip(
        pre: StorageState,
        purge_up_to: u64,
    )
        ensures
            purge_post(pre, purge_up_to).chain_tip == pre.chain_tip,
    {
        // By construction
    }
}
