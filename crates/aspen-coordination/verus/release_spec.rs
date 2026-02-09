//! Release Operation Specifications
//!
//! Proves that the release() operation preserves all lock invariants.
//!
//! # Key Properties
//!
//! 1. **Token Preservation**: Max fencing token is preserved (not decreased)
//! 2. **Lock Cleared**: Entry's deadline_ms is set to 0 (released state)
//! 3. **Holder Validation**: Only the current holder can release
//! 4. **Atomicity**: Via CAS semantics (assumed from storage layer)
//!
//! # Release Semantics
//!
//! When a lock is released:
//! - The deadline_ms is set to 0 (marking it as explicitly released)
//! - The fencing_token is preserved (for history/debugging)
//! - The holder_id is cleared (empty string) - this is intentional because released entries are
//!   always in an expired state (deadline_ms = 0), so the holder_id has no meaning and is cleared
//!   for cleanliness
//! - The max_fencing_token_issued is unchanged
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/release_spec.rs
//! ```

use vstd::prelude::*;

use super::lock_state_spec::*;

verus! {
    // ========================================================================
    // Release Precondition
    // ========================================================================

    /// Precondition for lock release
    ///
    /// The lock can be released if:
    /// - An entry exists
    /// - The caller is the current holder (matching holder_id and token)
    /// - Note: We allow release of expired locks by the original holder
    pub open spec fn release_pre(
        state: LockState,
        holder_id: Seq<u8>,
        token: u64,
    ) -> bool {
        match state.entry {
            None => false,  // Cannot release non-existent lock
            Some(entry) => {
                entry.holder_id == holder_id &&
                entry.fencing_token == token
            }
        }
    }

    /// Stronger precondition: lock is currently held (not expired)
    pub open spec fn release_pre_held(
        state: LockState,
        holder_id: Seq<u8>,
        token: u64,
    ) -> bool {
        is_held_by(state, holder_id, token)
    }

    // ========================================================================
    // Release Postcondition
    // ========================================================================

    /// Result of a successful release operation
    ///
    /// The release creates a new entry with:
    /// - Empty holder_id
    /// - Same fencing_token (preserved for history)
    /// - Same acquired_at_ms
    /// - ttl_ms = 0
    /// - deadline_ms = 0 (marks as released)
    pub open spec fn release_post(pre: LockState) -> LockState {
        let old_entry = pre.entry.unwrap();
        let released_entry = LockEntrySpec {
            holder_id: Seq::empty(),  // Cleared
            fencing_token: old_entry.fencing_token,  // Preserved
            acquired_at_ms: old_entry.acquired_at_ms,
            ttl_ms: 0,
            deadline_ms: 0,  // Marks as released
        };

        LockState {
            entry: Some(released_entry),
            current_time_ms: pre.current_time_ms,
            max_fencing_token_issued: pre.max_fencing_token_issued,  // Unchanged
        }
    }

    // ========================================================================
    // Proofs: Max Token Preservation
    // ========================================================================

    /// Release preserves max_fencing_token_issued
    pub proof fn release_preserves_max_token(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures release_post(pre).max_fencing_token_issued == pre.max_fencing_token_issued
    {
        // By construction: max_fencing_token_issued is unchanged
    }

    /// Release maintains fencing token monotonicity
    pub proof fn release_maintains_fencing_monotonicity(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures fencing_token_monotonic(pre, release_post(pre))
    {
        release_preserves_max_token(pre, holder_id, token);
        // post.max_fencing_token_issued == pre.max_fencing_token_issued >= pre
    }

    // ========================================================================
    // Proofs: Lock Cleared
    // ========================================================================

    /// Release sets deadline_ms to 0
    pub proof fn release_clears_deadline(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures release_post(pre).entry.unwrap().deadline_ms == 0
    {
        // By construction: deadline_ms = 0
    }

    /// Release makes the lock expired
    pub proof fn release_makes_expired(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures
            is_expired(release_post(pre).entry.unwrap(), release_post(pre).current_time_ms)
    {
        let post = release_post(pre);
        let entry = post.entry.unwrap();
        // entry.deadline_ms == 0
        // is_expired checks deadline_ms == 0 || current_time > deadline_ms
        // Therefore: is_expired(entry, post.current_time_ms)
    }

    /// Release makes the lock available
    pub proof fn release_makes_available(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures is_lock_available(release_post(pre))
    {
        release_makes_expired(pre, holder_id, token);
        // is_lock_available returns true if entry is expired
    }

    // ========================================================================
    // Proofs: Token Preserved in Entry
    // ========================================================================

    /// Release preserves the fencing token in the entry
    pub proof fn release_preserves_entry_token(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures
            release_post(pre).entry.unwrap().fencing_token ==
            pre.entry.unwrap().fencing_token
    {
        // By construction: fencing_token is copied from old entry
    }

    // ========================================================================
    // Proofs: Invariant Preservation
    // ========================================================================

    /// Release preserves entry_token_bounded
    pub proof fn release_preserves_entry_bounded(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires
            release_pre(pre, holder_id, token),
            entry_token_bounded(pre),
        ensures
            entry_token_bounded(release_post(pre))
    {
        let post = release_post(pre);
        let old_entry = pre.entry.unwrap();
        let new_entry = post.entry.unwrap();
        // new_entry.fencing_token == old_entry.fencing_token
        // old_entry.fencing_token <= pre.max_fencing_token_issued (by entry_token_bounded)
        // post.max_fencing_token_issued == pre.max_fencing_token_issued
        // Therefore: new_entry.fencing_token <= post.max_fencing_token_issued
    }

    /// Release establishes TTL validity (trivially, since deadline_ms = 0)
    pub proof fn release_establishes_ttl_validity(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures state_ttl_valid(release_post(pre))
    {
        let post = release_post(pre);
        let entry = post.entry.unwrap();
        // entry.deadline_ms == 0
        // ttl_expiration_valid checks deadline_ms == 0 first
        // Therefore: ttl_expiration_valid(entry)
    }

    /// Release preserves mutual exclusion (lock is now released)
    pub proof fn release_preserves_mutual_exclusion(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures mutual_exclusion_holds(release_post(pre))
    {
        let post = release_post(pre);
        let entry = post.entry.unwrap();
        // entry.deadline_ms == 0, so is_expired(entry, ...) is true
        // mutual_exclusion_holds allows expired entries
    }

    /// Release preserves the combined lock invariant
    pub proof fn release_preserves_lock_invariant(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires
            release_pre(pre, holder_id, token),
            lock_invariant(pre),
        ensures
            lock_invariant(release_post(pre))
    {
        release_preserves_entry_bounded(pre, holder_id, token);
        release_establishes_ttl_validity(pre, holder_id, token);
        release_preserves_mutual_exclusion(pre, holder_id, token);
    }

    // ========================================================================
    // Proofs: Lock No Longer Held
    // ========================================================================

    /// After release, the lock is no longer held by anyone
    pub proof fn release_clears_holder(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
    )
        requires release_pre(pre, holder_id, token)
        ensures !is_held_by(release_post(pre), holder_id, token)
    {
        let post = release_post(pre);
        let entry = post.entry.unwrap();
        // is_expired(entry, post.current_time_ms) because deadline_ms == 0
        // is_held_by requires !is_expired, so returns false
    }
}
