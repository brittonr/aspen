//! Verus Formal Specifications for Aspen Forge
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of Git hosting, content addressing, and authorization primitives.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-forge          # Verify all specs
//! nix run .#verify-verus-forge -- quick # Syntax check only
//! nix run .#verify-verus                 # Verify all (Core + Raft + Coordination + Forge)
//! ```
//!
//! # Module Overview
//!
//! ## Signed Object
//! - `signed_object_spec`: Content-addressed signed object invariants
//!
//! ## Ref Store
//! - `ref_store_spec`: Raft-backed ref storage invariants
//!
//! ## Repository Identity
//! - `repo_identity_spec`: Repository identity and delegate authorization
//!
//! # Invariants Verified
//!
//! ## Signed Object
//!
//! 1. **SIGNED-1: Signature Binding**: Signature covers payload + author + timestamp
//!    - Any modification to payload invalidates signature
//!    - Ed25519 verification ensures authenticity
//!
//! 2. **SIGNED-2: Content Addressing**: hash = BLAKE3(serialized_object)
//!    - Hash uniquely identifies object content
//!    - Same content produces same hash (deterministic)
//!
//! 3. **SIGNED-3: HLC Causality**: Timestamps maintain happened-before ordering
//!    - Objects created later have greater HLC timestamps
//!    - Enables deterministic conflict resolution
//!
//! 4. **SIGNED-4: Author Consistency**: author field matches signing key
//!    - Public key stored in object matches verifier expectation
//!
//! ## Ref Store
//!
//! 5. **REF-1: Linearizable Reads**: get() reads through Raft consensus
//!    - All nodes return the same value for the same key
//!    - Stale reads prevented via consistency flag
//!
//! 6. **REF-2: Atomic Writes**: set() writes via Raft proposal
//!    - All nodes agree on ref value after commit
//!    - Event emitted after successful write
//!
//! 7. **REF-3: CAS Semantics**: compare_and_set() is atomic
//!    - Only succeeds if current == expected
//!    - Prevents lost updates in concurrent pushes
//!
//! 8. **REF-4: Name Bounds**: ref_name.len() <= MAX_REF_NAME_LENGTH_BYTES
//!    - Enforced at write time
//!    - Prevents unbounded storage keys
//!
//! 9. **REF-5: Hash Encoding**: Hash stored as 64-char hex string
//!    - Roundtrip: hex(hash) -> hash is lossless
//!    - 32 bytes = 64 hex characters exactly
//!
//! ## Repository Identity
//!
//! 10. **REPO-1: Name Bounds**: name.len() > 0 && name.len() <= MAX_REPO_NAME_LENGTH_BYTES
//!     - Non-empty name required
//!     - Length bounded at construction
//!
//! 11. **REPO-2: Delegate Bounds**: delegates.len() in 1..=MAX_DELEGATES
//!     - At least one delegate required
//!     - Maximum enforced at construction
//!
//! 12. **REPO-3: Threshold Validity**: 1 <= threshold <= min(delegates.len(), MAX_THRESHOLD)
//!     - Threshold cannot be zero
//!     - Threshold cannot exceed delegate count
//!
//! 13. **REPO-4: Content-Addressed ID**: repo_id = BLAKE3(serialize(identity))
//!     - ID is deterministic from content
//!     - Same identity produces same ID
//!
//! 14. **REPO-5: Delegate Membership**: is_delegate(key) iff key in delegates
//!     - Membership check is accurate
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Ed25519 signatures are cryptographically secure
//! - BLAKE3 is collision-resistant
//! - Raft consensus provides linearizability
//! - HLC timestamps advance monotonically
//! - Serialization is deterministic (postcard)

use vstd::prelude::*;

verus! {
    // Re-export signed object specifications
    pub use signed_object_spec::*;

    // Re-export ref store specifications
    pub use ref_store_spec::*;

    // Re-export repo identity specifications
    pub use repo_identity_spec::*;
}

mod ref_store_spec;
mod repo_identity_spec;
mod signed_object_spec;
