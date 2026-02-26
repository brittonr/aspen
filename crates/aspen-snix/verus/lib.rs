//! Verus Formal Specifications for Aspen SNIX Storage
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of SNIX (Store NIX) storage operations in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! verus --crate-type=lib crates/aspen-snix/verus/lib.rs
//! ```
//!
//! # Module Overview
//!
//! ## Blob Storage
//! - `blob_spec`: Blob size and chunk limits
//!
//! ## Directory Storage
//! - `directory_spec`: Directory entry and depth limits
//!
//! ## Path Info
//! - `pathinfo_spec`: Store path references and signatures
//!
//! # Invariants Verified
//!
//! ## Blob Properties
//!
//! 1. **SNIX-1: Blob Size Bound**: blob_size <= MAX_BLOB_SIZE (1 GB)
//!    - Prevents memory exhaustion
//!
//! 2. **SNIX-2: Digest Valid**: digest.len() == B3_DIGEST_LENGTH (32)
//!    - Valid BLAKE3 hash format
//!
//! ## Directory Properties
//!
//! 3. **SNIX-3: Entry Bound**: entries <= MAX_DIRECTORY_ENTRIES (100,000)
//!    - Bounded directory size
//!
//! 4. **SNIX-4: Depth Bound**: depth <= MAX_DIRECTORY_DEPTH (256)
//!    - Prevents stack overflow in recursion
//!
//! ## Path Info Properties
//!
//! 5. **SNIX-5: Reference Bound**: refs <= MAX_PATH_REFERENCES (10,000)
//!    - Bounded dependency count
//!
//! 6. **SNIX-6: Signature Bound**: sigs <= MAX_SIGNATURES (100)
//!    - Bounded signature count
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - BLAKE3 hashes are collision-resistant
//! - Store path format follows Nix conventions
//! - Directory entries are unique by name

use vstd::prelude::*;

verus! {
    // Re-export blob specifications
    pub use blob_spec::blob_size_bounded;
    pub use blob_spec::digest_valid;

    // Re-export directory specifications
    pub use directory_spec::entry_bounded;
    pub use directory_spec::depth_bounded;

    // Re-export path info specifications
    pub use pathinfo_spec::reference_bounded;
    pub use pathinfo_spec::signature_bounded;
}

mod blob_spec;
mod directory_spec;
mod pathinfo_spec;
