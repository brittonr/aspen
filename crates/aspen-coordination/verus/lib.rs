//! Verus Formal Specifications for Aspen Coordination - Distributed Lock
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of distributed lock operations in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-coordination          # Verify all specs
//! nix run .#verify-verus-coordination -- quick # Syntax check only
//! nix run .#verify-verus                       # Verify all (Raft + Coordination)
//! ```
//!
//! # Module Overview
//!
//! - `lock_state_spec`: Lock state model and invariants
//! - `acquire_spec`: Lock acquisition operation correctness
//! - `release_spec`: Lock release operation correctness
//! - `renew_spec`: Lock renewal operation correctness
//!
//! # Invariants Verified
//!
//! 1. **Fencing Token Monotonicity**: Tokens strictly increase on each acquisition
//!    - Every successful acquire returns a token greater than any previously issued
//!    - This prevents split-brain scenarios where old holders think they still own the lock
//!
//! 2. **Mutual Exclusion**: At most one holder at any time (via CAS semantics)
//!    - Lock acquisition uses compare-and-swap to ensure atomicity
//!    - Only one client can successfully acquire a non-expired lock
//!
//! 3. **TTL Expiration Safety**: Expired locks become reacquirable
//!    - A lock is considered expired when current_time > deadline_ms or deadline_ms == 0
//!    - Expired locks can be taken by new holders with incremented tokens
//!
//! # State Model
//!
//! The lock state is modeled as:
//! ```ignore
//! LockState {
//!     entry: Option<LockEntrySpec>,     // Current lock holder (if any)
//!     current_time_ms: u64,             // System time for expiration checks
//!     max_fencing_token_issued: u64,    // Highest token ever issued (monotonic)
//! }
//! ```
//!
//! # Operation Specifications
//!
//! ## Acquire
//! - Pre: Lock must be available (None or expired)
//! - Post: New holder with token = max_fencing_token_issued + 1
//! - Preserves: Fencing token monotonicity
//!
//! ## Release
//! - Pre: Caller must be current holder with matching token
//! - Post: Lock marked as released (deadline_ms = 0), token preserved
//! - Preserves: Max token tracking
//!
//! ## Renew
//! - Pre: Caller must hold non-expired lock with matching token
//! - Post: Deadline extended, same fencing token
//! - Preserves: Fencing token unchanged
//!
//! # Trusted Axioms
//!
//! The specification assumes:
//! - CAS operations are linearizable (provided by Raft consensus)
//! - System clock advances monotonically (standard OS assumption)

use vstd::prelude::*;

verus! {
    // Re-export core specifications
    pub use lock_state_spec::LockEntrySpec;
    pub use lock_state_spec::LockState;
    pub use lock_state_spec::is_expired;
    pub use lock_state_spec::is_lock_available;
    pub use lock_state_spec::is_held_by;
    pub use lock_state_spec::fencing_token_monotonic;
    pub use lock_state_spec::entry_token_bounded;
    pub use lock_state_spec::lock_invariant;

    pub use acquire_spec::acquire_pre;
    pub use acquire_spec::acquire_post;

    pub use release_spec::release_pre;
    pub use release_spec::release_post;

    pub use renew_spec::renew_pre;
    pub use renew_spec::renew_post;
}

mod acquire_spec;
mod lock_state_spec;
mod release_spec;
mod renew_spec;
