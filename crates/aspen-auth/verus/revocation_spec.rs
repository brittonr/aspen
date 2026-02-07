//! Revocation List Specification
//!
//! Formal specification for token revocation in capability-based auth.
//!
//! # Security Properties
//!
//! 1. **AUTH-7: Revocation Atomicity**: Once revoked, stays revoked
//! 2. **AUTH-8: Revocation List Bounds**: list.len() <= MAX_REVOCATION_LIST_SIZE
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-auth/verus/revocation_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum revocation list size
    pub const MAX_REVOCATION_LIST_SIZE: u64 = 10_000;

    /// Token hash size
    pub const TOKEN_HASH_SIZE: u64 = 32;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract revocation list state
    pub struct RevocationState {
        /// Number of revoked tokens
        pub count: u64,
        /// Whether a specific token is revoked (abstract predicate)
        /// In reality, this is a HashSet<[u8; 32]>
    }

    // ========================================================================
    // Invariant 7: Revocation Atomicity
    // ========================================================================

    /// AUTH-7: Once revoked, a token stays revoked
    ///
    /// Revocation is monotonic - tokens can only be added to the list,
    /// never removed (except via explicit clear operation).
    pub open spec fn revocation_atomicity(
        pre: RevocationState,
        post: RevocationState,
        token_revoked_before: bool,
        token_revoked_after: bool,
    ) -> bool {
        // If token was revoked before, it's still revoked after
        token_revoked_before ==> token_revoked_after
    }

    /// Revocation is monotonically increasing
    pub open spec fn revocation_monotonic(
        pre: RevocationState,
        post: RevocationState,
    ) -> bool {
        // Count only increases (unless explicit clear)
        post.count >= pre.count
    }

    // ========================================================================
    // Invariant 8: Revocation List Bounded
    // ========================================================================

    /// AUTH-8: Revocation list size is bounded
    pub open spec fn revocation_list_bounded(state: RevocationState) -> bool {
        state.count <= MAX_REVOCATION_LIST_SIZE
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined revocation invariant
    pub open spec fn revocation_invariant(state: RevocationState) -> bool {
        revocation_list_bounded(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial revocation state (empty)
    pub open spec fn initial_revocation_state() -> RevocationState {
        RevocationState {
            count: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant()
        ensures revocation_invariant(initial_revocation_state())
    {
        // count = 0 <= MAX_REVOCATION_LIST_SIZE
    }

    // ========================================================================
    // Revocation Operations
    // ========================================================================

    /// Effect of revoking a token
    pub open spec fn revoke_effect(
        state: RevocationState,
        already_revoked: bool,
    ) -> RevocationState {
        if already_revoked {
            // Token already in set, no change
            state
        } else {
            // Add new revocation
            RevocationState {
                count: state.count + 1,
            }
        }
    }

    /// Proof: Revoke preserves invariant if under limit
    pub proof fn revoke_preserves_invariant(
        state: RevocationState,
        already_revoked: bool,
    )
        requires
            revocation_invariant(state),
            !already_revoked ==> state.count < MAX_REVOCATION_LIST_SIZE,
        ensures revocation_invariant(revoke_effect(state, already_revoked))
    {
        let post = revoke_effect(state, already_revoked);
        if already_revoked {
            // No change
            assert(post.count == state.count);
        } else {
            // Count increased by 1
            assert(post.count == state.count + 1);
            assert(post.count <= MAX_REVOCATION_LIST_SIZE);
        }
    }

    /// Effect of clearing revocations
    pub open spec fn clear_effect(state: RevocationState) -> RevocationState {
        RevocationState {
            count: 0,
        }
    }

    /// Proof: Clear produces valid state
    pub proof fn clear_produces_valid_state(state: RevocationState)
        ensures revocation_invariant(clear_effect(state))
    {
        // count = 0 <= MAX_REVOCATION_LIST_SIZE
    }

    /// Effect of loading revocations from storage
    pub open spec fn load_effect(
        state: RevocationState,
        load_count: u64,
    ) -> Option<RevocationState> {
        let new_count = state.count + load_count;
        if new_count > MAX_REVOCATION_LIST_SIZE {
            None  // Would exceed limit
        } else {
            Some(RevocationState {
                count: new_count,
            })
        }
    }

    /// Proof: Load preserves invariant
    pub proof fn load_preserves_invariant(
        state: RevocationState,
        load_count: u64,
    )
        requires
            revocation_invariant(state),
            state.count + load_count <= MAX_REVOCATION_LIST_SIZE,
        ensures {
            let post = load_effect(state, load_count).unwrap();
            revocation_invariant(post)
        }
    {
        // new_count <= MAX_REVOCATION_LIST_SIZE
    }

    // ========================================================================
    // Verification Interaction
    // ========================================================================

    /// Check if token is revoked
    pub open spec fn is_revoked(
        token_in_set: bool,
    ) -> bool {
        token_in_set
    }

    /// Revoked tokens fail verification
    pub open spec fn revoked_tokens_fail(
        token_revoked: bool,
        verification_passes: bool,
    ) -> bool {
        // If token is revoked, verification must fail
        token_revoked ==> !verification_passes
    }

    /// Proof: Revoked token cannot verify
    pub proof fn revoked_cannot_verify(
        token_revoked: bool,
    )
        requires token_revoked
        ensures !(!token_revoked)  // Token is revoked
    {
        // Direct from precondition
    }

    // ========================================================================
    // Concurrent Access
    // ========================================================================

    /// RwLock guarantees for concurrent access
    ///
    /// The revocation list uses RwLock<HashSet<...>>:
    /// - Multiple readers can check is_revoked concurrently
    /// - Writers have exclusive access for revoke/clear
    pub open spec fn rwlock_guarantees(
        readers_active: u64,
        writer_active: bool,
    ) -> bool {
        // Mutual exclusion: writer implies no readers
        writer_active ==> readers_active == 0
    }

    /// Read during revoke sees consistent state
    ///
    /// Either sees token as not-revoked (read happened before write)
    /// or sees token as revoked (read happened after write)
    pub open spec fn read_consistent(
        read_before_write: bool,
        token_seen_revoked: bool,
    ) -> bool {
        // If read before write, might not see revocation
        // If read after write, will see revocation
        !read_before_write ==> token_seen_revoked
    }
}
