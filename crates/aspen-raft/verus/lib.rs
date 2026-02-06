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
//! ## Storage Operations
//! - `chain_hash_spec`: Chain hash computation and validity
//! - `storage_state_spec`: Storage state model and invariants
//! - `append_spec`: Append operation correctness
//! - `truncate_spec`: Truncate operation correctness
//! - `purge_spec`: Purge operation correctness
//! - `snapshot_spec`: Snapshot operations correctness
//!
//! ## Write Batcher
//! - `batcher_state_spec`: Batcher state model and invariants
//! - `batcher_add_spec`: Add write operation correctness
//! - `batcher_flush_spec`: Flush batch operation correctness
//!
//! ## Integrity & Verification
//! - `chain_verify_spec`: Chain hash verification and tamper detection
//! - `ttl_spec`: TTL expiration correctness
//! - `apply_request_spec`: State machine apply operations
//!
//! # Invariants Verified
//!
//! ## Storage Invariants
//! 1. **Log-State Atomicity**: All operations are atomic via single transaction
//! 2. **Chain Continuity**: Each hash correctly links to predecessor
//! 3. **Chain Tip Sync**: chain_tip reflects the latest entry
//! 4. **Response Cache**: Only applied entries have cached responses
//! 5. **last_applied Monotonicity**: Never decreases
//! 6. **last_purged Monotonicity**: Never decreases
//! 7. **Snapshot Integrity**: Valid hashes and chain continuity
//!
//! ## Batcher Invariants
//! 8. **BATCH-1: Size Bound**: pending.len() <= max_entries
//! 9. **BATCH-2: Bytes Bound**: current_bytes <= max_bytes
//! 10. **BATCH-3: Bytes Consistency**: current_bytes == sum of pending sizes
//! 11. **BATCH-4: Ordering Preservation**: FIFO order maintained
//! 12. **BATCH-5: No Write Loss**: Every write in pending is flushed
//! 13. **BATCH-6: Atomicity**: Batch submitted together as single Raft proposal
//!
//! ## Integrity Invariants
//! 14. **INTEG-1: Tamper Detection**: Modified data produces different hash
//! 15. **INTEG-2: Rollback Detection**: Chain can only grow (monotonic)
//! 16. **INTEG-3: Snapshot Binding**: Combined hash binds data and metadata
//!
//! ## TTL Invariants
//! 17. **TTL-1: Expired Not Returned**: Expired keys not returned by get()
//! 18. **TTL-2: Cleanup Correctness**: Cleanup removes only expired keys
//! 19. **TTL-3: TTL Monotonicity**: Expiration time is immutable after set
//!
//! ## Apply Invariants
//! 20. **APPLY-1: Version Increment**: Version increments on update
//! 21. **APPLY-2: Index Consistency**: mod_revision matches apply index
//! 22. **APPLY-3: Idempotency**: Re-applying same entry is no-op

use vstd::prelude::*;

verus! {
    // Re-export core specifications
    pub use chain_hash_spec::*;
    pub use storage_state_spec::*;
    pub use append_spec::*;
    pub use truncate_spec::*;
    pub use purge_spec::*;
    pub use snapshot_spec::*;

    // Re-export batcher specifications
    pub use batcher_state_spec::*;
    pub use batcher_add_spec::*;
    pub use batcher_flush_spec::*;

    // Re-export integrity and verification specs
    pub use chain_verify_spec::*;
    pub use ttl_spec::*;
    pub use apply_request_spec::*;
}

mod append_spec;
mod chain_hash_spec;
mod purge_spec;
mod snapshot_spec;
mod storage_state_spec;
mod truncate_spec;

// Write batcher specifications
mod batcher_add_spec;
mod batcher_flush_spec;
mod batcher_state_spec;

// Integrity and verification specifications
mod apply_request_spec;
mod chain_verify_spec;
mod ttl_spec;
