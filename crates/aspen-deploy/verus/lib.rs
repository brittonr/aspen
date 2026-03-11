//! Verus Formal Specifications for Aspen Deploy
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of quorum safety computations used in rolling deployments.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus deploy    # Verify deploy specs
//! nix run .#verify-verus           # Verify all specs
//! ```
//!
//! # Module Overview
//!
//! ## Quorum Safety
//! - `quorum_spec`: Quorum size, upgrade safety, and concurrent upgrade bounds
//!
//! # Invariants Verified
//!
//! ## Quorum Safety for Rolling Deploys
//!
//! 1. **QUORUM-1: Majority Guarantee**: `quorum_size(n) > n / 2` for all n >= 1. Ensures that only
//!    one partition of the cluster can form a quorum.
//!
//! 2. **QUORUM-2: Quorum Bounded**: `quorum_size(n) <= n` for all n >= 1. Quorum never exceeds the
//!    total voter count.
//!
//! 3. **QUORUM-3: Upgrade Safety**: `can_upgrade_node` returns true only when removing one more
//!    voter still leaves a quorum. Prevents losing majority during rolling upgrades.
//!
//! 4. **QUORUM-4: Max Concurrent Bound**: `max_concurrent_upgrades(n)` leaves at least
//!    `quorum_size(n)` voters operational for odd clusters >= 3.
//!
//! 5. **QUORUM-5: Single Node Minimum**: `max_concurrent_upgrades(n) >= 1` for all n, ensuring
//!    progress is always possible.
//!
//! # Trusted Axioms
//!
//! The specification assumes:
//!
//! ## Voter Count Stability
//! The total voter count does not change during the quorum safety check.
//! This is enforced by the deployment coordinator holding a consistent view
//! of membership throughout a single upgrade step.
//!
//! ## Healthy Voter Accuracy
//! The `healthy_voters` count accurately reflects the number of voters that
//! are currently participating in Raft consensus. This is provided by the
//! Raft membership and health check mechanisms.
//!
//! ## Type Bounds
//! All integer types are bounded by their Rust definitions:
//! - u32: [0, 2^32 - 1]

use vstd::prelude::*;

verus! {
    // Re-export quorum specifications
    pub use quorum_spec::quorum_size_spec;
    pub use quorum_spec::quorum_is_majority;
    pub use quorum_spec::quorum_is_bounded;
    pub use quorum_spec::can_upgrade_spec;
    pub use quorum_spec::max_concurrent_spec;

    // Verified exec functions
    pub use quorum_spec::quorum_size;
    pub use quorum_spec::can_upgrade_node;
    pub use quorum_spec::max_concurrent_upgrades;
}

mod overflow_constants_spec;
mod quorum_spec;
