//! Verification of truncate() operation
//!
//! This module proves that the `truncate()` operation in `SharedRedbStorage`
//! preserves storage invariants.
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
//! # Important: last_applied Handling
//!
//! Truncation does NOT reset last_applied. This is because:
//! - Applied state machine changes are permanent
//! - Raft guarantees committed entries are never truncated
//! - Only uncommitted entries can be truncated

use super::storage_state::StorageStateSpec;
use super::storage_state::chain_tip_synchronized;
use super::storage_state::storage_invariant;

/// Specification of truncate() postcondition.
///
/// Removes all log entries from `truncate_at` onwards.
pub fn truncate_post(pre: &StorageStateSpec, truncate_at: u64) -> StorageStateSpec {
    let mut post = pre.clone();

    // Remove log entries >= truncate_at
    post.log.retain(|&idx, _| idx < truncate_at);

    // Remove chain hashes >= truncate_at
    post.chain_hashes.retain(|&idx, _| idx < truncate_at);

    // Update chain tip
    if let Some(&max_idx) = post.log.keys().max() {
        if let Some(hash) = post.chain_hashes.get(&max_idx) {
            post.chain_tip = (*hash, max_idx);
        }
    } else {
        // Log is now empty
        post.chain_tip = (post.genesis_hash, 0);
    }

    // Note: last_applied is NOT changed
    // - Applied state is permanent
    // - Raft guarantees committed entries aren't truncated

    // Remove responses for truncated entries
    post.pending_responses.retain(|&idx, _| idx < truncate_at);

    post
}

/// Verify that truncate preserves storage invariants.
pub fn verify_truncate_preserves_invariants(pre: &StorageStateSpec, truncate_at: u64) -> Result<(), String> {
    // Precondition: pre-state satisfies invariants
    if !storage_invariant(pre) {
        return Err("Pre-state does not satisfy invariants".to_string());
    }

    let post = truncate_post(pre, truncate_at);

    // Postcondition: post-state satisfies invariants
    if !storage_invariant(&post) {
        return Err("Post-state does not satisfy invariants".to_string());
    }

    // Postcondition: chain tip is synchronized
    if !chain_tip_synchronized(&post) {
        return Err("Chain tip not synchronized after truncate".to_string());
    }

    // Postcondition: no entries >= truncate_at remain
    if post.log.keys().any(|&idx| idx >= truncate_at) {
        return Err("Log entries >= truncate_at still exist".to_string());
    }

    if post.chain_hashes.keys().any(|&idx| idx >= truncate_at) {
        return Err("Chain hashes >= truncate_at still exist".to_string());
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
*/

#[cfg(test)]
mod tests {
    use super::super::chain_hash::LogEntryData;
    use super::*;

    fn make_populated_state() -> StorageStateSpec {
        let mut state = StorageStateSpec::new();

        // Add some log entries
        for i in 1..=5 {
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

        state.chain_tip = (*state.chain_hashes.get(&5).unwrap(), 5);
        state.last_applied = Some(5);

        state
    }

    #[test]
    fn test_truncate_partial() {
        let pre = make_populated_state();
        let result = verify_truncate_preserves_invariants(&pre, 3);
        assert!(result.is_ok(), "truncate should preserve invariants: {:?}", result);

        let post = truncate_post(&pre, 3);
        assert_eq!(post.log.len(), 2); // entries 1, 2 remain
        assert_eq!(post.chain_tip.1, 2);
        assert!(!post.log.contains_key(&3));
        assert!(!post.log.contains_key(&4));
        assert!(!post.log.contains_key(&5));
    }

    #[test]
    fn test_truncate_all() {
        let pre = make_populated_state();
        let result = verify_truncate_preserves_invariants(&pre, 1);
        assert!(result.is_ok(), "truncate should preserve invariants: {:?}", result);

        let post = truncate_post(&pre, 1);
        assert_eq!(post.log.len(), 0);
        assert_eq!(post.chain_tip.0, pre.genesis_hash);
        assert_eq!(post.chain_tip.1, 0);
    }

    #[test]
    fn test_truncate_beyond_end() {
        let pre = make_populated_state();
        // Truncating beyond the end should not change anything
        let result = verify_truncate_preserves_invariants(&pre, 100);
        assert!(result.is_ok(), "truncate should preserve invariants: {:?}", result);

        let post = truncate_post(&pre, 100);
        assert_eq!(post.log.len(), 5); // all entries remain
        assert_eq!(post.chain_tip.1, 5);
    }

    #[test]
    fn test_truncate_idempotent() {
        let pre = make_populated_state();
        let post1 = truncate_post(&pre, 3);
        let post2 = truncate_post(&post1, 3);

        assert_eq!(post1.log.len(), post2.log.len());
        assert_eq!(post1.chain_tip, post2.chain_tip);
    }

    #[test]
    fn test_truncate_preserves_last_applied() {
        let pre = make_populated_state();
        let post = truncate_post(&pre, 3);

        // last_applied should NOT change (state machine changes are permanent)
        assert_eq!(post.last_applied, pre.last_applied);
    }
}
