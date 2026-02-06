//! Formal verification specifications for distributed lock
//!
//! This module contains Verus specifications that prove correctness
//! of the distributed lock implementation.
//!
//! # Overview
//!
//! The specs verify three key invariants:
//!
//! 1. **Fencing Token Monotonicity**: Tokens strictly increase on each acquisition
//! 2. **Mutual Exclusion**: At most one holder at any time (via CAS semantics)
//! 3. **TTL Expiration Safety**: Expired locks become reacquirable
//!
//! # Module Structure
//!
//! - `lock_invariants`: Runtime invariant checks for lock state
//! - `verus_shim`: Zero-cost ghost code macros for production use
//!
//! # Usage
//!
//! These specifications are compiled when the `verus` feature is enabled:
//!
//! ```bash
//! # Verify specifications
//! nix run .#verify-verus-coordination
//!
//! # Check ghost code compiles
//! cargo check -p aspen-coordination --features verus
//! ```
//!
//! # Design Rationale
//!
//! The specifications model the lock as a state machine with:
//! - Lock entry (holder_id, fencing_token, TTL, deadline)
//! - Current time for expiration checks
//! - Max fencing token ever issued (monotonicity tracking)
//!
//! Each operation (acquire, release, renew) is specified with:
//! - Preconditions (what must hold before the operation)
//! - Postconditions (what holds after the operation)
//! - Preservation proofs (which invariants are maintained)

pub mod lock_invariants;
pub mod verus_shim;

// Re-export key types for use in lock.rs
pub use lock_invariants::GhostLockState;
pub use lock_invariants::LockStateSpec;
// Re-export verus shims for ghost code in production
pub use verus_shim::*;
