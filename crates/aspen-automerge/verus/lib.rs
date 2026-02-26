//! Verus Formal Specifications for Aspen Automerge
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of Automerge CRDT document management in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! verus --crate-type=lib crates/aspen-automerge/verus/lib.rs
//! ```
//!
//! # Module Overview
//!
//! ## Document Management
//! - `document_spec`: Document size limits and ID validation
//!
//! ## Change Management
//! - `change_spec`: Change size limits and batching
//!
//! ## Sync Protocol
//! - `sync_spec`: Sync message limits and timeout
//!
//! # Invariants Verified
//!
//! ## Document Properties
//!
//! 1. **AM-1: Document Size Bound**: doc_size <= MAX_DOCUMENT_SIZE (16 MB)
//!    - Prevents memory exhaustion
//!
//! 2. **AM-2: Document ID Valid**: ID is 16 bytes (128 bits)
//!    - Collision-resistant identification
//!
//! 3. **AM-3: Namespace Limits**: documents_per_namespace <= MAX (100,000)
//!    - Bounded growth per namespace
//!
//! ## Change Properties
//!
//! 4. **AM-4: Change Size Bound**: change_size <= MAX_CHANGE_SIZE (1 MB)
//!    - Prevents DoS via large changes
//!
//! 5. **AM-5: Batch Size Bound**: batch_size <= MAX_BATCH_CHANGES (1,000)
//!    - Bounded CPU usage per batch
//!
//! ## Sync Properties
//!
//! 6. **AM-6: Sync Message Bound**: msg_size <= MAX_SYNC_MESSAGE_SIZE
//!    - Prevents memory exhaustion during sync
//!
//! 7. **AM-7: Sync Buffer Bound**: buffer_size <= MAX_SYNC_BUFFER (100)
//!    - Bounded message buffering
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Automerge correctly implements CRDT semantics
//! - Document serialization is deterministic
//! - Random document IDs are collision-resistant

use vstd::prelude::*;

verus! {
    // Re-export document specifications
    pub use document_spec::document_size_bounded;
    pub use document_spec::document_id_valid;
    pub use document_spec::namespace_bounded;

    // Re-export change specifications
    pub use change_spec::change_size_bounded;
    pub use change_spec::batch_size_bounded;

    // Re-export sync specifications
    pub use sync_spec::sync_message_bounded;
    pub use sync_spec::sync_buffer_bounded;
}

mod change_spec;
mod document_spec;
mod sync_spec;
