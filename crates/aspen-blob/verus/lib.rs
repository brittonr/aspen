//! Verus Formal Specifications for Aspen Blob Storage
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of blob storage operations in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! verus --crate-type=lib crates/aspen-blob/verus/lib.rs
//! ```
//!
//! # Module Overview
//!
//! ## Blob Storage
//! - `blob_spec`: Blob size limits and thresholds
//!
//! ## Concurrency
//! - `concurrency_spec`: Download/upload limits
//!
//! ## Garbage Collection
//! - `gc_spec`: GC timing and tag management
//!
//! # Invariants Verified
//!
//! ## Size Properties
//!
//! 1. **BLOB-1: Size Bounds**: blob_size <= MAX_BLOB_SIZE (1 GB)
//!    - Prevents memory exhaustion
//!    - Bounded resource usage
//!
//! 2. **BLOB-2: Threshold Routing**: size > BLOB_THRESHOLD -> offload to blob
//!    - Values > 1 MB stored as blob references
//!    - Prevents KV bloat
//!
//! 3. **BLOB-3: Reference Integrity**: blob_ref -> blob exists
//!    - References point to valid blobs
//!    - No dangling references
//!
//! ## Concurrency Properties
//!
//! 4. **BLOB-4: Download Limit**: concurrent_downloads <= MAX_CONCURRENT (10)
//!    - Prevents connection exhaustion
//!
//! 5. **BLOB-5: Upload Limit**: concurrent_uploads <= MAX_CONCURRENT (10)
//!    - Prevents bandwidth exhaustion
//!
//! ## GC Properties
//!
//! 6. **BLOB-6: GC Grace Period**: blob not collected until grace_period elapsed
//!    - In-flight operations complete before collection
//!
//! 7. **BLOB-7: Tag Protection**: tagged blobs not collected
//!    - Protected blobs persist
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - iroh-blobs correctly implements content-addressed storage
//! - BLAKE3 hashes are collision-resistant
//! - Semaphores correctly enforce concurrency limits

use vstd::prelude::*;

verus! {
    // Re-export blob specifications
    pub use blob_spec::blob_size_bounded;
    pub use blob_spec::threshold_routing;
    pub use blob_spec::reference_integrity;

    // Re-export concurrency specifications
    pub use concurrency_spec::download_bounded;
    pub use concurrency_spec::upload_bounded;

    // Re-export gc specifications
    pub use gc_spec::grace_period_respected;
    pub use gc_spec::tag_protection;
}

mod blob_spec;
mod concurrency_spec;
mod gc_spec;
