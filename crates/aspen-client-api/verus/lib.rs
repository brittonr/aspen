//! Verus Formal Specifications for Aspen Client API
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of client RPC protocol message handling and bounds.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-client-api          # Verify all specs
//! nix run .#verify-verus-client-api -- quick # Syntax check only
//! nix run .#verify-verus                      # Verify all
//! ```
//!
//! # Module Overview
//!
//! ## Protocol Messages
//! - `message_spec`: Message size bounds and protocol invariants
//!
//! # Invariants Verified
//!
//! ## Message Bounds
//!
//! 1. **MSG-1: Size Bounds**: message.len() <= MAX_CLIENT_MESSAGE_SIZE
//!    - All messages bounded by 4MB limit
//!    - Size check BEFORE deserialization
//!
//! 2. **MSG-2: Chunk Bounds**: chunk.len() <= MAX_GIT_CHUNK_SIZE
//!    - Git chunks bounded by 4MB
//!    - Prevents memory exhaustion in streaming operations
//!
//! 3. **MSG-3: Cluster State Bounds**: nodes.len() <= MAX_CLUSTER_NODES
//!    - Response bounded by 16 nodes
//!    - Prevents unbounded cluster state responses
//!
//! ## CAS Semantics
//!
//! 4. **CAS-1: CompareAndSwap Atomicity**: Expected value matching is atomic
//!    - None expected = key must not exist
//!    - Some(val) expected = key must have exactly that value
//!
//! 5. **CAS-2: CompareAndDelete Atomicity**: Delete requires value match
//!    - Key must exist with exactly the expected value
//!    - Prevents accidental deletion of updated values
//!
//! ## Token Handling
//!
//! 6. **TOKEN-1: Wire Format**: Legacy vs authenticated distinction
//!    - Tag 0 = legacy (no token)
//!    - Tag 1 = authenticated (token present)
//!
//! 7. **TOKEN-2: Token Presence**: Authenticated requests have tokens
//!    - new() always sets token to Some
//!    - unauthenticated() always sets token to None
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Serialization produces bounded output for bounded input
//! - Network transport preserves message integrity
//! - Raft consensus provides linearizable CAS semantics

use vstd::prelude::*;

verus! {
    // Re-export message specifications
    pub use message_spec::*;
}

mod message_spec;
