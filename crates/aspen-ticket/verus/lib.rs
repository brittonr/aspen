//! Verus Formal Specifications for Aspen Ticket
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of cluster ticket operations in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-ticket          # Verify all specs
//! nix run .#verify-verus-ticket -- quick # Syntax check only
//! nix run .#verify-verus                 # Verify all specs
//! ```
//!
//! # Module Overview
//!
//! ## Cluster Tickets
//! - `ticket_state_spec`: Ticket state model and invariants
//! - `signature_spec`: Signed ticket signature verification
//! - `timestamp_spec`: Timestamp validation and expiration
//! - `bootstrap_spec`: Bootstrap peer bounds and constraints
//!
//! # Invariants Verified
//!
//! ## Ticket Bounds
//!
//! 1. **TICKET-1: Bootstrap Bounds**: bootstrap.len() <= MAX_BOOTSTRAP_PEERS (16)
//!    - Prevents unbounded ticket size
//!    - Enforced at add_bootstrap() and add_bootstrap_addr()
//!
//! 2. **TICKET-2: Address Bounds**: direct_addrs.len() <= MAX_DIRECT_ADDRS_PER_PEER (8)
//!    - Prevents per-peer unbounded addresses
//!    - Enforced via truncation at add_bootstrap_addr()
//!
//! 3. **TICKET-3: Serialization Determinism**: Postcard serialization is deterministic
//!    - Same ticket always produces same bytes
//!    - Required for signature verification
//!
//! ## Signed Ticket Security
//!
//! 4. **SIGNED-1: Signature Integrity**: Tampered data fails verification
//!    - Ed25519 signature binds all fields
//!    - Any modification invalidates signature
//!
//! 5. **SIGNED-2: Timestamp Validity**: Tickets have valid time bounds
//!    - issued_at <= now + CLOCK_SKEW_TOLERANCE
//!    - now < expires_at
//!
//! 6. **SIGNED-3: Nonce Uniqueness**: Each ticket has unique nonce
//!    - 128-bit random nonce prevents replay
//!    - Collision probability negligible
//!
//! 7. **SIGNED-4: Version Compatibility**: Unknown versions rejected
//!    - version <= SIGNED_TICKET_VERSION accepted
//!    - Future versions fail verification
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Ed25519 signatures are cryptographically secure
//! - Postcard serialization is deterministic and reversible
//! - System clock provides monotonically advancing time
//! - Random number generator provides uniform distribution

use vstd::prelude::*;

verus! {
    // Re-export ticket state specifications
    pub use ticket_state_spec::TicketState;
    pub use ticket_state_spec::BootstrapPeerSpec;
    pub use ticket_state_spec::bootstrap_bounds;
    pub use ticket_state_spec::address_bounds;
    pub use ticket_state_spec::ticket_invariant;

    // Re-export signature specifications
    pub use signature_spec::SignedTicketState;
    pub use signature_spec::signature_integrity;
    pub use signature_spec::signature_binds_all_fields;
    pub use signature_spec::signed_ticket_invariant;

    // Re-export timestamp specifications
    pub use timestamp_spec::timestamp_validity;
    pub use timestamp_spec::not_expired;
    pub use timestamp_spec::not_issued_in_future;
    pub use timestamp_spec::within_clock_skew;

    // Re-export bootstrap specifications
    pub use bootstrap_spec::add_bootstrap_pre;
    pub use bootstrap_spec::add_bootstrap_post;
    pub use bootstrap_spec::inject_addr_idempotent;
}

mod bootstrap_spec;
mod signature_spec;
mod ticket_state_spec;
mod timestamp_spec;
