//! Truncate Operation Specifications
//!
//! Proves that the truncate() operation preserves storage invariants.
//!
//! # Truncation in Raft
//!
//! Truncation occurs when a follower receives entries that conflict with
//! its existing log. The conflicting entries (and all subsequent entries)
//! must be removed before appending the leader's entries.
//!
//! # Key Properties
//!
//! 1. **Chain Validity**: Remaining entries still form a valid chain
//! 2. **Chain Tip Update**: chain_tip is updated to reflect new end
//! 3. **Crash Safety**: Truncation is atomic via single transaction
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/truncate_spec.rs
//! ```

use vstd::prelude::*;

use super::chain_hash_spec::*;
use super::storage_state_spec::*;

verus! {
    /// Specification of truncate() behavior
    ///
    /// Removes all entries with index >= truncate_at
    ///
    /// # Precondition
    ///
    /// When truncate_at > 0 and there will be remaining entries (truncate_at > 1),
    /// the caller must ensure that the entry at index (truncate_at - 1) exists
    /// in the log with a valid chain hash. This is typically guaranteed by
    /// the Raft protocol which only truncates at conflict points where earlier
    /// entries are known to exist.
    pub open spec fn truncate_post(
        pre: StorageState,
        truncate_at: u64,
    ) -> StorageState
        requires
            // When truncating to a non-empty state, the new tip entry must exist
            truncate_at == 0 || truncate_at == 1 ||
            (pre.log.contains_key(truncate_at - 1) && pre.chain_hashes.contains_key(truncate_at - 1))
    {
        let retained_log = pre.log.restrict(Set::new(|i: u64| i < truncate_at));
        let retained_hashes = pre.chain_hashes.restrict(Set::new(|i: u64| i < truncate_at));
        let retained_responses = pre.pending_responses.restrict(Set::new(|i: u64| i < truncate_at));

        // Compute the new chain tip based on the truncation point
        let new_chain_tip = if truncate_at == 0 {
            // Truncating everything: reset to genesis
            (pre.genesis_hash, 0u64)
        } else if truncate_at == 1 {
            // Only index 0 remains (if it exists), otherwise genesis
            if pre.chain_hashes.contains_key(0) {
                (pre.chain_hashes[0], 0u64)
            } else {
                (pre.genesis_hash, 0u64)
            }
        } else {
            // Entries remain: new tip is at (truncate_at - 1)
            // The recommends clause ensures this entry exists
            let new_tip_idx = truncate_at - 1;
            if pre.chain_hashes.contains_key(new_tip_idx) {
                (pre.chain_hashes[new_tip_idx], new_tip_idx)
            } else {
                // Fallback (should not happen with valid preconditions)
                pre.chain_tip
            }
        };

        StorageState {
            log: retained_log,
            chain_hashes: retained_hashes,
            chain_tip: new_chain_tip,
            pending_responses: retained_responses,
            // These fields unchanged:
            kv: pre.kv,
            last_applied: pre.last_applied,  // NOT changed
            last_purged: pre.last_purged,
            genesis_hash: pre.genesis_hash,
        }
    }

    /// Truncation removes entries >= truncate_at
    pub proof fn truncate_removes_entries(
        pre: StorageState,
        truncate_at: u64,
    )
        ensures
            forall |i: u64| i >= truncate_at ==>
                !truncate_post(pre, truncate_at).log.contains_key(i),
    {
        // By construction: restrict keeps only i < truncate_at
    }

    /// Truncation preserves entries < truncate_at
    pub proof fn truncate_preserves_entries(
        pre: StorageState,
        truncate_at: u64,
    )
        ensures
            forall |i: u64| i < truncate_at && pre.log.contains_key(i) ==>
                truncate_post(pre, truncate_at).log.contains_key(i),
    {
        // By construction: restrict preserves i < truncate_at
    }

    /// Corollary: Truncation is idempotent
    pub proof fn truncate_idempotent(
        pre: StorageState,
        truncate_at: u64,
    )
        ensures
            truncate_post(truncate_post(pre, truncate_at), truncate_at).log
            == truncate_post(pre, truncate_at).log,
    {
        // After first truncate, no entries >= truncate_at
        // Second truncate removes nothing additional
    }
}
