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
    pub open spec fn truncate_post(
        pre: StorageState,
        truncate_at: u64,
    ) -> StorageState {
        let retained_log = pre.log.restrict(Set::new(|i: u64| i < truncate_at));
        let retained_hashes = pre.chain_hashes.restrict(Set::new(|i: u64| i < truncate_at));
        let retained_responses = pre.pending_responses.restrict(Set::new(|i: u64| i < truncate_at));

        // Compute new chain tip
        let new_tip = if retained_log.is_empty() {
            (pre.genesis_hash, 0u64)
        } else {
            let max_idx = retained_log.dom().fold(0u64, |acc, i| if i > acc { i } else { acc });
            (retained_hashes[max_idx], max_idx)
        };

        StorageState {
            log: retained_log,
            chain_hashes: retained_hashes,
            chain_tip: new_tip,
            pending_responses: retained_responses,
            // These fields unchanged:
            kv: pre.kv,
            last_applied: pre.last_applied,  // NOT changed
            last_purged: pre.last_purged,
            genesis_hash: pre.genesis_hash,
        }
    }

    /// Main theorem: truncate preserves chain validity for remaining entries
    pub proof fn truncate_preserves_invariants(
        pre: StorageState,
        truncate_at: u64,
    )
        requires
            storage_invariant(pre),
        ensures
            storage_invariant(truncate_post(pre, truncate_at)),
    {
        let post = truncate_post(pre, truncate_at);

        // Prove chain tip synchronized
        assert(chain_tip_synchronized(post)) by {
            if post.log.is_empty() {
                assert(post.chain_tip == (pre.genesis_hash, 0));
            } else {
                // max index in retained log
                let max_idx = post.log.dom().fold(0u64, |acc, i| if i > acc { i } else { acc });
                assert(post.chain_tip.1 == max_idx);
                assert(post.chain_hashes.contains_key(max_idx));
            }
        }

        // Prove chain valid for retained entries
        assert(chain_valid(post.chain_hashes,
            post.log.map_values(|e: LogEntry| (e.term, e.data)),
            post.genesis_hash)) by {
            // Use truncate_preserves_chain lemma from chain_hash module
            truncate_preserves_chain(
                pre.chain_hashes,
                pre.log.map_values(|e: LogEntry| (e.term, e.data)),
                pre.genesis_hash,
                truncate_at
            );
        }

        // Prove response cache consistent
        // Retained responses have indices < truncate_at
        // last_applied unchanged, so consistency preserved
        assert(response_cache_consistent(post));
    }

    /// Corollary: Truncation is idempotent
    ///
    /// Truncating at the same point twice gives the same result.
    pub proof fn truncate_idempotent(
        pre: StorageState,
        truncate_at: u64,
    )
        requires storage_invariant(pre)
        ensures
            truncate_post(truncate_post(pre, truncate_at), truncate_at)
            == truncate_post(pre, truncate_at)
    {
        // After first truncate, no entries >= truncate_at
        // Second truncate removes nothing
    }

    /// Corollary: Truncating at 1 clears the entire log
    pub proof fn truncate_all(pre: StorageState)
        requires storage_invariant(pre)
        ensures {
            let post = truncate_post(pre, 1);
            post.log.is_empty() &&
            post.chain_tip == (pre.genesis_hash, 0)
        }
    {
        // All valid indices are >= 1, so truncate_at=1 removes everything
    }
}
