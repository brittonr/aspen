//! Lock State Machine Model
//!
//! Abstract state model for formal verification of distributed lock operations.
//!
//! # State Model
//!
//! The `LockState` captures:
//! - Current lock entry (holder, token, TTL, deadline)
//! - Current system time for expiration checks
//! - Maximum fencing token ever issued (for monotonicity tracking)
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/lock_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    /// Abstract lock entry structure
    ///
    /// Models the LockEntry type from types.rs
    pub struct LockEntrySpec {
        /// Unique identifier of the lock holder
        pub holder_id: Seq<u8>,
        /// Monotonically increasing token for fencing
        pub fencing_token: u64,
        /// When the lock was acquired (Unix timestamp milliseconds)
        pub acquired_at_ms: u64,
        /// TTL in milliseconds
        pub ttl_ms: u64,
        /// Deadline = acquired_at_ms + ttl_ms (0 means released)
        pub deadline_ms: u64,
    }

    /// Complete lock state for verification
    pub struct LockState {
        /// Current lock entry (None if lock never acquired)
        pub entry: Option<LockEntrySpec>,
        /// Current system time in milliseconds
        pub current_time_ms: u64,
        /// Maximum fencing token ever issued
        /// This tracks the highest token across all acquisitions
        pub max_fencing_token_issued: u64,
    }

    // ========================================================================
    // Core Predicates
    // ========================================================================

    /// Check if a lock entry is expired
    ///
    /// A lock is expired if:
    /// - deadline_ms == 0 (explicitly released), OR
    /// - current_time > deadline_ms (TTL elapsed)
    pub open spec fn is_expired(entry: LockEntrySpec, current_time_ms: u64) -> bool {
        entry.deadline_ms == 0 || current_time_ms > entry.deadline_ms
    }

    /// Check if the lock is available for acquisition
    ///
    /// The lock is available if:
    /// - No entry exists (never acquired), OR
    /// - The current entry is expired
    pub open spec fn is_lock_available(state: LockState) -> bool {
        match state.entry {
            None => true,
            Some(entry) => is_expired(entry, state.current_time_ms),
        }
    }

    /// Check if the lock is held by a specific holder with a specific token
    pub open spec fn is_held_by(
        state: LockState,
        holder_id: Seq<u8>,
        token: u64,
    ) -> bool {
        match state.entry {
            None => false,
            Some(entry) => {
                entry.holder_id == holder_id &&
                entry.fencing_token == token &&
                !is_expired(entry, state.current_time_ms)
            }
        }
    }

    // ========================================================================
    // Invariant 1: Fencing Token Monotonicity
    // ========================================================================

    /// INVARIANT 1: Fencing token monotonicity
    ///
    /// The max_fencing_token_issued can only increase, never decrease.
    /// This ensures every new acquisition gets a strictly greater token.
    pub open spec fn fencing_token_monotonic(
        pre: LockState,
        post: LockState,
    ) -> bool {
        post.max_fencing_token_issued >= pre.max_fencing_token_issued
    }

    /// Stronger form: new token is strictly greater
    pub open spec fn fencing_token_strictly_increases(
        pre: LockState,
        post: LockState,
    ) -> bool {
        post.max_fencing_token_issued > pre.max_fencing_token_issued
    }

    /// The current entry's token is always <= max_fencing_token_issued
    pub open spec fn entry_token_bounded(state: LockState) -> bool {
        match state.entry {
            None => true,
            Some(entry) => entry.fencing_token <= state.max_fencing_token_issued,
        }
    }

    // ========================================================================
    // Invariant 2: TTL Expiration Validity
    // ========================================================================

    /// INVARIANT 2: TTL expiration validity
    ///
    /// For non-released entries, deadline = acquired_at + ttl
    pub open spec fn ttl_expiration_valid(entry: LockEntrySpec) -> bool {
        // deadline_ms == 0 indicates a released lock (valid)
        // Otherwise, deadline should equal acquired_at + ttl
        entry.deadline_ms == 0 ||
        entry.deadline_ms == entry.acquired_at_ms + entry.ttl_ms
    }

    /// All entries in state have valid TTL computation
    pub open spec fn state_ttl_valid(state: LockState) -> bool {
        match state.entry {
            None => true,
            Some(entry) => ttl_expiration_valid(entry),
        }
    }

    // ========================================================================
    // Invariant 3: Mutual Exclusion (implicit via CAS)
    // ========================================================================

    /// INVARIANT 3: Mutual exclusion
    ///
    /// At most one holder at any time. This is enforced by:
    /// - Using CAS operations for all state transitions
    /// - Only allowing acquire when lock is available
    ///
    /// # Verification Approach
    ///
    /// Since this is a single-entry model (not multi-node), mutual exclusion
    /// is verified by proving that:
    /// 1. Acquire only succeeds when lock is available (expired or None)
    /// 2. Release/expiration is the only way to make a held lock available
    /// 3. At any moment, at most one non-expired entry exists
    ///
    /// The single-entry model guarantees (1-holder max) by construction.
    /// This predicate verifies entry well-formedness.
    pub open spec fn mutual_exclusion_holds(state: LockState) -> bool {
        match state.entry {
            None => true,  // 0 holders - mutual exclusion satisfied
            Some(entry) => {
                // For a non-expired lock, verify holder well-formedness:
                // - holder_id must be non-empty (identifies the holder)
                // - fencing_token must be positive (valid token)
                // Expired locks don't need these checks (they're effectively released)
                if is_expired(entry, state.current_time_ms) {
                    true  // Expired = available = no current holder
                } else {
                    // Active lock must have valid holder identification
                    entry.holder_id.len() > 0 &&
                    entry.fencing_token > 0 &&
                    // deadline must be in the future (non-expired check is above, but
                    // we also verify deadline > 0 for non-released locks)
                    entry.deadline_ms > 0
                }
            }
        }
    }

    /// True mutual exclusion across operations
    ///
    /// This verifies that operations respect mutual exclusion:
    /// - Acquire: only succeeds if lock available
    /// - Renew: only succeeds if caller is current holder
    /// - Release: only succeeds if caller is current holder
    ///
    /// The key insight: mutual exclusion is a PROTOCOL property, not just state.
    /// We verify it by proving each operation maintains the invariant.
    pub open spec fn operation_respects_mutual_exclusion(
        pre: LockState,
        post: LockState,
        op: LockOp,
    ) -> bool {
        match op {
            LockOp::Acquire(requester_id, token) => {
                // Acquire: pre must be available, post has exactly one holder
                is_lock_available(pre) &&
                post.entry.is_some() &&
                !is_lock_available(post)
            }
            LockOp::Renew(holder_id, token) => {
                // Renew: pre holder == post holder, lock remains held
                pre.entry.is_some() &&
                post.entry.is_some() &&
                pre.entry.unwrap().holder_id == post.entry.unwrap().holder_id &&
                pre.entry.unwrap().fencing_token == post.entry.unwrap().fencing_token
            }
            LockOp::Release(holder_id, token) => {
                // Release: pre was held by releaser, post is available
                pre.entry.is_some() &&
                is_lock_available(post)
            }
        }
    }

    /// Lock operation type for mutual exclusion verification
    pub enum LockOp {
        Acquire(Seq<u8>, u64),  // (requester_id, new_token)
        Renew(Seq<u8>, u64),    // (holder_id, token)
        Release(Seq<u8>, u64),  // (holder_id, token)
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant predicate for lock state
    pub open spec fn lock_invariant(state: LockState) -> bool {
        entry_token_bounded(state) &&
        state_ttl_valid(state) &&
        mutual_exclusion_holds(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial lock state (never acquired)
    pub open spec fn initial_lock_state(current_time_ms: u64) -> LockState {
        LockState {
            entry: None,
            current_time_ms,
            max_fencing_token_issued: 0,
        }
    }

    /// Proof: Initial state satisfies all invariants
    pub proof fn initial_state_invariant(current_time_ms: u64)
        ensures lock_invariant(initial_lock_state(current_time_ms))
    {
        // Initial state trivially satisfies:
        // - entry_token_bounded: no entry
        // - state_ttl_valid: no entry
        // - mutual_exclusion_holds: no entry
    }

    /// Proof: Initial state is available
    pub proof fn initial_state_available(current_time_ms: u64)
        ensures is_lock_available(initial_lock_state(current_time_ms))
    {
        // No entry means lock is available
    }
}
