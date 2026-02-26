//! Verus Formal Specifications for Aspen Auth
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of capability-based authorization in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-auth          # Verify all specs
//! nix run .#verify-verus-auth -- quick # Syntax check only
//! nix run .#verify-verus               # Verify all specs
//! ```
//!
//! # Module Overview
//!
//! ## Token Verification
//! - `token_state_spec`: Token state model and invariants
//! - `signature_spec`: Signature verification correctness
//! - `delegation_spec`: Delegation chain depth bounds
//! - `revocation_spec`: Revocation list invariants
//!
//! # Invariants Verified
//!
//! ## Token Bounds
//!
//! 1. **AUTH-1: Capability Bounds**: capabilities.len() <= MAX_CAPABILITIES_PER_TOKEN
//!    - Prevents unbounded token size
//!    - MAX_CAPABILITIES_PER_TOKEN = 32
//!
//! 2. **AUTH-2: Token Size Bounds**: encoded_size <= MAX_TOKEN_SIZE
//!    - Prevents oversized tokens (DoS prevention)
//!    - MAX_TOKEN_SIZE = 8KB
//!
//! ## Delegation Chain
//!
//! 3. **AUTH-3: Delegation Depth Bound**: depth <= MAX_DELEGATION_DEPTH
//!    - Prevents unbounded proof chains
//!    - MAX_DELEGATION_DEPTH = 8
//!
//! 4. **AUTH-4: Chain Termination**: All chains lead to a root or reject
//!    - Recursive verification terminates
//!
//! ## Signature Security
//!
//! 5. **AUTH-5: Signature Correctness**: Valid tokens have valid signatures
//!    - Ed25519 signature binds all fields
//!    - Tampering invalidates signature
//!
//! 6. **AUTH-6: Issuer Binding**: Signature proves issuer identity
//!    - Only holder of private key can sign
//!
//! ## Revocation
//!
//! 7. **AUTH-7: Revocation Atomicity**: Once revoked, token stays revoked
//!    - No race conditions in revocation check
//!
//! 8. **AUTH-8: Revocation List Bounds**: list.len() <= MAX_REVOCATION_LIST_SIZE
//!    - Prevents unbounded memory growth
//!    - MAX_REVOCATION_LIST_SIZE = 10,000
//!
//! ## Timestamp Validation
//!
//! 9. **AUTH-9: Expiration Check**: Token expires at expires_at + tolerance
//!    - Clock skew tolerance = 60 seconds
//!
//! 10. **AUTH-10: Future Token Rejection**: issued_at <= now + tolerance
//!    - Rejects tokens from the future
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Ed25519 signatures are cryptographically secure
//! - BLAKE3 hashing is collision-resistant
//! - System clock advances monotonically
//! - Postcard serialization is deterministic

use vstd::prelude::*;

verus! {
    // Re-export token state specifications
    pub use token_state_spec::TokenState;
    pub use token_state_spec::capability_bounds;
    pub use token_state_spec::token_size_bounds;
    pub use token_state_spec::token_invariant;

    // Re-export delegation specifications
    pub use delegation_spec::DelegationChainState;
    pub use delegation_spec::delegation_depth_bounded;
    pub use delegation_spec::chain_terminates;
    pub use delegation_spec::verify_pre;
    pub use delegation_spec::verify_post;

    // Re-export signature specifications
    pub use signature_spec::SignatureState;
    pub use signature_spec::signature_valid;
    pub use signature_spec::issuer_bound;
    pub use signature_spec::tampering_detected;

    // Re-export revocation specifications
    pub use revocation_spec::RevocationState;
    pub use revocation_spec::revocation_atomicity;
    pub use revocation_spec::revocation_list_bounded;
    pub use revocation_spec::revocation_invariant;

    // Re-export timestamp specifications
    pub use timestamp_spec::timestamp_valid;
    pub use timestamp_spec::not_expired;
    pub use timestamp_spec::not_from_future;
}

mod delegation_spec;
mod revocation_spec;
mod signature_spec;
mod timestamp_spec;
mod token_state_spec;
