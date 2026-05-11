//! Renew Operation Specifications
//!
//! Proves that the renew() operation preserves all lock invariants.
//!
//! # Key Properties
//!
//! 1. **Token Unchanged**: Fencing token remains the same after renewal
//! 2. **Deadline Extended**: New deadline = new_acquired_at + ttl
//! 3. **Holder Validation**: Only the current holder can renew
//! 4. **Not Expired**: Can only renew a non-expired lock
//!
//! # Renew Semantics
//!
//! When a lock is renewed:
//! - The fencing_token is preserved (critical for fencing guarantees)
//! - The acquired_at_ms is updated to current time
//! - The deadline_ms is extended based on new TTL
//! - The holder_id remains the same
//! - The max_fencing_token_issued is unchanged
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/renew_spec.rs
//! ```

use vstd::prelude::*;

use super::acquire_spec::add_u64;
use super::lock_state_spec::*;

verus! {
    // ========================================================================
    // Renew Precondition
    // ========================================================================

    /// Precondition for lock renewal
    ///
    /// The lock can be renewed if:
    /// - An entry exists
    /// - The caller is the current holder (matching holder_id and token)
    /// - The lock has not expired
    pub open spec fn renew_pre(
        state: LockState,
        holder_id: Seq<u8>,
        token: u64,
    ) -> bool {
        is_held_by(state, holder_id, token)
    }

    // ========================================================================
    // Renew Postcondition
    // ========================================================================

    /// Result of a successful renew operation
    ///
    /// The renewal creates a new entry with:
    /// - Same holder_id
    /// - Same fencing_token (critical!)
    /// - Updated acquired_at_ms
    /// - New ttl_ms
    /// - New deadline_ms = new_acquired_at + new_ttl
    ///
    /// Assumes:
    /// - pre.entry.is_some()
    /// - // Deadline overflow protection: new_acquired_at + new_ttl must not overflow new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms
    pub open spec fn renew_post(
        pre: LockState,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    ) -> LockState {
        let old_entry = pre.entry.unwrap();
        let deadline_int = add_u64(new_acquired_at_ms, new_ttl_ms);
        let renewed_entry = LockEntrySpec {
            holder_id: old_entry.holder_id,  // Unchanged
            fencing_token: old_entry.fencing_token,  // Unchanged (critical!)
            acquired_at_ms: new_acquired_at_ms,
            ttl_ms: new_ttl_ms,
            deadline_ms: deadline_int as u64,
        };

        LockState {
            entry: Some(renewed_entry),
            current_time_ms: pre.current_time_ms,
            max_fencing_token_issued: pre.max_fencing_token_issued,  // Unchanged
        }
    }

    // ========================================================================
    // Proofs: Fencing Token Preserved
    // ========================================================================

    /// CRITICAL: Renew preserves the fencing token
    ///
    /// This is the most important property of renewal - the token must not
    /// change, as external services use it for fencing.
    pub proof fn renew_preserves_fencing_token(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    )
        requires
            renew_pre(pre, holder_id, token),
            // Overflow protection for deadline calculation
            new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms,
        ensures
            renew_post(pre, new_ttl_ms, new_acquired_at_ms).entry.unwrap().fencing_token ==
            pre.entry.unwrap().fencing_token
    {
        let post = renew_post(pre, new_ttl_ms, new_acquired_at_ms);
        assert(post.entry.unwrap().fencing_token == pre.entry.unwrap().fencing_token);
    }

    /// Renew preserves max_fencing_token_issued
    pub proof fn renew_preserves_max_token(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    )
        requires
            renew_pre(pre, holder_id, token),
            // Overflow protection for deadline calculation
            new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms,
        ensures
            renew_post(pre, new_ttl_ms, new_acquired_at_ms).max_fencing_token_issued ==
            pre.max_fencing_token_issued
    {
        let post = renew_post(pre, new_ttl_ms, new_acquired_at_ms);
        assert(post.max_fencing_token_issued == pre.max_fencing_token_issued);
    }

    /// Renew maintains fencing token monotonicity
    pub proof fn renew_maintains_fencing_monotonicity(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    )
        requires
            renew_pre(pre, holder_id, token),
            // Overflow protection for deadline calculation
            new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms,
        ensures fencing_token_monotonic(pre, renew_post(pre, new_ttl_ms, new_acquired_at_ms))
    {
        renew_preserves_max_token(pre, holder_id, token, new_ttl_ms, new_acquired_at_ms);
        assert(renew_post(pre, new_ttl_ms, new_acquired_at_ms).max_fencing_token_issued == pre.max_fencing_token_issued);
    }

    // ========================================================================
    // Proofs: Invariant Preservation
    // ========================================================================

    /// Renew preserves entry_token_bounded
    pub proof fn renew_preserves_entry_bounded(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    )
        requires
            renew_pre(pre, holder_id, token),
            entry_token_bounded(pre),
            // Overflow protection for deadline calculation
            new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms,
        ensures
            entry_token_bounded(renew_post(pre, new_ttl_ms, new_acquired_at_ms))
    {
        renew_preserves_fencing_token(pre, holder_id, token, new_ttl_ms, new_acquired_at_ms);
        renew_preserves_max_token(pre, holder_id, token, new_ttl_ms, new_acquired_at_ms);
        let post = renew_post(pre, new_ttl_ms, new_acquired_at_ms);
        assert(pre.entry.unwrap().fencing_token <= pre.max_fencing_token_issued);
        assert(post.entry.unwrap().fencing_token == pre.entry.unwrap().fencing_token);
        assert(post.max_fencing_token_issued == pre.max_fencing_token_issued);
    }

    /// Renew preserves mutual exclusion
    pub proof fn renew_preserves_mutual_exclusion(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    )
        requires
            renew_pre(pre, holder_id, token),
            mutual_exclusion_holds(pre),
            // Overflow protection for deadline calculation
            new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms,
        ensures
            mutual_exclusion_holds(renew_post(pre, new_ttl_ms, new_acquired_at_ms))
    {
        let old_entry = pre.entry.unwrap();
        let post = renew_post(pre, new_ttl_ms, new_acquired_at_ms);
        let new_entry = post.entry.unwrap();
        assert(!is_expired(old_entry, pre.current_time_ms));
        assert(old_entry.holder_id.len() > 0);
        assert(old_entry.fencing_token > 0);
        assert(new_entry.holder_id == old_entry.holder_id);
        assert(new_entry.fencing_token == old_entry.fencing_token);
        if is_expired(new_entry, post.current_time_ms) {
            assert(mutual_exclusion_holds(post));
        } else {
            assert(new_entry.holder_id.len() > 0);
            assert(new_entry.fencing_token > 0);
            assert(new_entry.deadline_ms > 0);
            assert(mutual_exclusion_holds(post));
        }
    }

    /// Renew preserves TTL validity
    ///
    /// After renewal, the new entry has correct TTL computation:
    /// deadline_ms == acquired_at_ms + ttl_ms
    pub proof fn renew_preserves_ttl_validity(
        pre: LockState,
        holder_id: Seq<u8>,
        token: u64,
        new_ttl_ms: u64,
        new_acquired_at_ms: u64,
    )
        requires
            renew_pre(pre, holder_id, token),
            state_ttl_valid(pre),
            // Overflow protection for deadline calculation
            new_acquired_at_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - new_ttl_ms,
        ensures
            state_ttl_valid(renew_post(pre, new_ttl_ms, new_acquired_at_ms))
    {
        let post = renew_post(pre, new_ttl_ms, new_acquired_at_ms);
        let new_entry = post.entry.unwrap();
        assert(add_u64(new_acquired_at_ms, new_ttl_ms) == new_acquired_at_ms as int + new_ttl_ms as int);
        assert(add_u64(new_acquired_at_ms, new_ttl_ms) <= u64::MAX as int);
        assert(new_entry.deadline_ms as int == add_u64(new_acquired_at_ms, new_ttl_ms));
        assert(new_entry.deadline_ms as int == new_entry.acquired_at_ms as int + new_entry.ttl_ms as int);
        assert(ttl_expiration_valid(new_entry));
    }
}
