//! Verus Formal Specifications for Aspen Cluster
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of gossip, federation, and cluster coordination primitives.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-cluster          # Verify all specs
//! nix run .#verify-verus-cluster -- quick # Syntax check only
//! nix run .#verify-verus                  # Verify all (Core + Raft + Coordination + Cluster)
//! ```
//!
//! # Module Overview
//!
//! ## Gossip Primitives
//! - `gossip_state_spec`: Gossip message state models
//! - `signed_announcement_spec`: Cryptographic signature verification specs
//! - `blob_announcement_spec`: Blob seeding announcement specs
//!
//! ## Federation Primitives
//! - `identity_spec`: Cluster identity and key derivation
//! - `trust_spec`: Trust manager and authorization specs
//!
//! # Invariants Verified
//!
//! ## Signed Peer Announcement
//!
//! 1. **GOSSIP-1: Signature Binding**: Signature covers all announcement fields
//!    - Tampering any field (node_id, timestamp, endpoint) invalidates signature
//!    - Ed25519 verification ensures cryptographic binding
//!
//! 2. **GOSSIP-2: Size Bounds**: Messages bounded by MAX_GOSSIP_MESSAGE_SIZE
//!    - Size check BEFORE deserialization prevents memory exhaustion
//!    - Explicit 4KB limit for all gossip messages
//!
//! 3. **GOSSIP-3: Version Compatibility**: Future versions rejected
//!    - Unknown versions filtered during deserialization
//!    - Prevents processing of incompatible messages
//!
//! 4. **GOSSIP-4: Key Consistency**: Signing key matches endpoint_addr.id
//!    - Public key in endpoint used for verification
//!    - Mismatched keys cause verification failure
//!
//! ## Signed Topology Announcement
//!
//! 5. **TOPO-1: Topology Version Integrity**: Version field immutable under signature
//!    - Topology version changes invalidate signature
//!    - Protects against stale topology injection
//!
//! 6. **TOPO-2: Hash Binding**: topology_hash protected by signature
//!    - Hash modifications detected via signature failure
//!    - Ensures topology content integrity
//!
//! ## Blob Announcement
//!
//! 7. **BLOB-1: Hash Immutability**: blob_hash protected by signature
//!    - Changing hash invalidates signature
//!    - Critical for content-addressed storage integrity
//!
//! 8. **BLOB-2: Size Truthfulness**: blob_size protected by signature
//!    - Size modifications detected
//!    - Prevents resource exhaustion attacks
//!
//! 9. **BLOB-3: Tag Bounds**: tag.len() <= MAX_TAG_LEN (64 bytes)
//!    - Enforced at construction time
//!    - Bounded payload size
//!
//! ## Cluster Identity
//!
//! 10. **IDENT-1: Name Bounds**: name.len() <= MAX_CLUSTER_NAME_LEN
//!     - Enforced during construction
//!
//! 11. **IDENT-2: Description Bounds**: description.len() <= MAX_CLUSTER_DESCRIPTION_LEN
//!     - Enforced during construction
//!
//! 12. **IDENT-3: Key Derivation**: public_key = secret_key.public()
//!     - Cryptographic binding between keypair components
//!
//! ## Trust Manager
//!
//! 13. **TRUST-1: Trusted Capacity**: trusted.len() <= MAX_TRUSTED_CLUSTERS
//!     - Bounded set prevents unbounded growth
//!
//! 14. **TRUST-2: Blocked Capacity**: blocked.len() <= MAX_BLOCKED_CLUSTERS
//!     - Bounded set prevents unbounded growth
//!
//! 15. **TRUST-3: Pending Capacity**: pending.len() <= MAX_PENDING_REQUESTS
//!     - Bounded queue prevents request flooding
//!
//! 16. **TRUST-4: Block Precedence**: blocked overrides trusted
//!     - If cluster in both sets, trust_level returns Blocked
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Ed25519 signatures are cryptographically secure (binding, unforgeability)
//! - System clock advances monotonically (standard OS assumption)
//! - Serialization produces canonical bytes (postcard determinism)

use vstd::prelude::*;

verus! {
    // Re-export gossip specifications
    pub use gossip_state_spec::*;
    pub use signed_announcement_spec::*;
    pub use blob_announcement_spec::*;

    // Re-export federation specifications
    pub use identity_spec::*;
    pub use trust_spec::*;
}

mod blob_announcement_spec;
mod gossip_state_spec;
mod identity_spec;
mod signed_announcement_spec;
mod trust_spec;
