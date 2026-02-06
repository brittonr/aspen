//! Verus Formal Specifications for Aspen Raft Storage
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of Raft storage operations in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus          # Verify all specs
//! nix run .#verify-verus -- quick # Syntax check only
//! ```
//!
//! # Module Overview
//!
//! - `chain_hash_spec`: Chain hash computation and validity
//! - `storage_state_spec`: Storage state model and invariants
//! - `append_spec`: Append operation correctness
//! - `truncate_spec`: Truncate operation correctness
//! - `purge_spec`: Purge operation correctness
//! - `snapshot_spec`: Snapshot operations correctness
//!
//! # Invariants Verified
//!
//! 1. **Log-State Atomicity**: All operations are atomic via single transaction
//! 2. **Chain Continuity**: Each hash correctly links to predecessor
//! 3. **Chain Tip Sync**: chain_tip reflects the latest entry
//! 4. **Response Cache**: Only applied entries have cached responses
//! 5. **last_applied Monotonicity**: Never decreases
//! 6. **last_purged Monotonicity**: Never decreases
//! 7. **Snapshot Integrity**: Valid hashes and chain continuity

use vstd::prelude::*;

verus! {
    // Re-export core specifications
    pub use chain_hash_spec::*;
    pub use storage_state_spec::*;
    pub use append_spec::*;
    pub use truncate_spec::*;
    pub use purge_spec::*;
    pub use snapshot_spec::*;
}

mod append_spec;
mod chain_hash_spec;
mod purge_spec;
mod snapshot_spec;
mod storage_state_spec;
mod truncate_spec;
