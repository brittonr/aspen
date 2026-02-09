//! Acquire Operation Specifications
//!
//! Proves that the acquire() operation preserves all lock invariants.
//!
//! # Key Properties
//!
//! 1. **Fencing Token Monotonicity**: New token > max_fencing_token_issued
//! 2. **TTL Validity**: New deadline = acquired_at + ttl
//! 3. **Mutual Exclusion**: Only succeeds if lock is available
//! 4. **Atomicity**: Via CAS semantics (assumed from storage layer)
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/acquire_spec.rs
//! ```

use vstd::prelude::*;

use super::lock_state_spec::*;

verus! {
    // ========================================================================
    // Helper: Safe increment that returns int for proof purposes
    // ========================================================================

    /// Increment by 1 (returns int to preserve arithmetic properties)
    pub open spec fn add1(n: u64) -> int {
        n + 1
    }

    /// Add two u64 values (returns int)
    pub open spec fn add_u64(a: u64, b: u64) -> int {
        a + b
    }

    // ========================================================================
    // Acquire Precondition
    // ========================================================================

    /// Precondition for lock acquisition
    ///
    /// The lock can be acquired if:
    /// - No entry exists (never acquired), OR
    /// - The current entry is expired (deadline passed or explicitly released)
    pub open spec fn acquire_pre(state: LockState) -> bool {
        is_lock_available(state)
    }

    // ========================================================================
    // Acquire Postcondition
    // ========================================================================

    /// Result of a successful acquire operation
    ///
    /// Note: We use int for intermediate arithmetic, then cast at the boundary.
    /// The new_token is defined as max_fencing_token_issued + 1 conceptually.
    pub open spec fn acquire_post(
        pre: LockState,
        requester_id: Seq<u8>,
        ttl_ms: u64,
        acquired_at_ms: u64,
    ) -> LockState
        requires pre.max_fencing_token_issued < 0xFFFF_FFFF_FFFF_FFFFu64
    {
        let new_token_int = add1(pre.max_fencing_token_issued);
        let deadline_int = add_u64(acquired_at_ms, ttl_ms);
        let new_entry = LockEntrySpec {
            holder_id: requester_id,
            fencing_token: new_token_int as u64,
            acquired_at_ms,
            ttl_ms,
            deadline_ms: deadline_int as u64,
        };

        LockState {
            entry: Some(new_entry),
            current_time_ms: pre.current_time_ms,
            max_fencing_token_issued: new_token_int as u64,
        }
    }

    /// Get the new fencing token that would be issued (as int for proofs)
    pub open spec fn new_token_int(pre: LockState) -> int {
        add1(pre.max_fencing_token_issued)
    }

    // ========================================================================
    // Proofs: Fencing Token Monotonicity
    // ========================================================================

    /// The new token (as int) is strictly greater than the old max
    pub proof fn acquire_new_token_greater(
        pre: LockState,
    )
        requires pre.max_fencing_token_issued < 0xFFFF_FFFF_FFFF_FFFFu64
        ensures new_token_int(pre) > pre.max_fencing_token_issued as int
    {
        // add1(x) = x + 1 > x when x < MAX
    }

    /// Acquire preserves fencing token monotonicity
    ///
    /// The post state's max token >= pre state's max token
    pub proof fn acquire_preserves_fencing_monotonicity(
        pre: LockState,
        requester_id: Seq<u8>,
        ttl_ms: u64,
        acquired_at_ms: u64,
    )
        requires
            acquire_pre(pre),
            pre.max_fencing_token_issued < 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures
            acquire_post(pre, requester_id, ttl_ms, acquired_at_ms).max_fencing_token_issued
            >= pre.max_fencing_token_issued
    {
        // post.max = (pre.max + 1) as u64 >= pre.max when pre.max < MAX
    }

    // ========================================================================
    // Proofs: TTL Validity (simplified)
    // ========================================================================

    /// The new entry has the correct deadline relationship
    pub proof fn acquire_deadline_computed_correctly(
        pre: LockState,
        requester_id: Seq<u8>,
        ttl_ms: u64,
        acquired_at_ms: u64,
    )
        requires
            acquire_pre(pre),
            pre.max_fencing_token_issued < 0xFFFF_FFFF_FFFF_FFFFu64,
            acquired_at_ms + ttl_ms <= 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures ({
            let post = acquire_post(pre, requester_id, ttl_ms, acquired_at_ms);
            let entry = post.entry.unwrap();
            entry.deadline_ms as int == entry.acquired_at_ms + entry.ttl_ms
        })
    {
        // By construction: deadline = (acquired_at + ttl) as u64
        // When acquired_at + ttl <= MAX, the cast preserves equality
    }

    // ========================================================================
    // Proofs: Entry Token Bounded
    // ========================================================================

    /// The new entry's token equals the new max_fencing_token_issued
    pub proof fn acquire_entry_token_equals_max(
        pre: LockState,
        requester_id: Seq<u8>,
        ttl_ms: u64,
        acquired_at_ms: u64,
    )
        requires
            acquire_pre(pre),
            pre.max_fencing_token_issued < 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures ({
            let post = acquire_post(pre, requester_id, ttl_ms, acquired_at_ms);
            let entry = post.entry.unwrap();
            entry.fencing_token == post.max_fencing_token_issued
        })
    {
        // Both are (pre.max + 1) as u64
    }

    // ========================================================================
    // Proofs: Combined Invariant Preservation
    // ========================================================================

    /// Acquire preserves entry_token_bounded
    pub proof fn acquire_preserves_entry_bounded(
        pre: LockState,
        requester_id: Seq<u8>,
        ttl_ms: u64,
        acquired_at_ms: u64,
    )
        requires
            acquire_pre(pre),
            pre.max_fencing_token_issued < 0xFFFF_FFFF_FFFF_FFFFu64,
        ensures
            entry_token_bounded(acquire_post(pre, requester_id, ttl_ms, acquired_at_ms))
    {
        acquire_entry_token_equals_max(pre, requester_id, ttl_ms, acquired_at_ms);
        // entry.token == post.max, so entry.token <= post.max
    }
}
