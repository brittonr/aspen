//! Verus Formal Specifications for Aspen Binary Cache
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of Nix binary cache metadata in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! verus --crate-type=lib crates/aspen-cache/verus/lib.rs
//! ```
//!
//! # Module Overview
//!
//! ## Cache Entry
//! - `entry_spec`: Cache entry bounds and validation
//!
//! ## Cache Statistics
//! - `stats_spec`: Hit rate tracking and consistency
//!
//! # Invariants Verified
//!
//! ## Entry Properties
//!
//! 1. **CACHE-1: Reference Bound**: references.len() <= MAX_REFERENCES (1,000)
//!    - Bounded dependency tracking
//!
//! 2. **CACHE-2: Deriver Bound**: deriver.len() <= MAX_DERIVER_LENGTH (1,024)
//!    - Bounded deriver path
//!
//! 3. **CACHE-3: Store Path Bound**: store_path.len() <= MAX_STORE_PATH_LENGTH
//!    - Bounded store path
//!
//! 4. **CACHE-4: Hash Valid**: store_hash.len() == STORE_HASH_LENGTH (32)
//!    - Valid hash format
//!
//! ## Stats Properties
//!
//! 5. **CACHE-5: Query Count**: query_count == hit_count + miss_count
//!    - Consistent counting
//!
//! 6. **CACHE-6: Hit Rate**: 0 <= hit_rate <= 100
//!    - Valid percentage
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - SHA256 hashes are valid
//! - BLAKE3 blob hashes are collision-resistant
//! - JSON serialization is deterministic

use vstd::prelude::*;

verus! {
    // Re-export entry specifications
    pub use entry_spec::reference_bounded;
    pub use entry_spec::deriver_bounded;
    pub use entry_spec::store_path_bounded;
    pub use entry_spec::hash_valid;

    // Re-export stats specifications
    pub use stats_spec::query_count_consistent;
    pub use stats_spec::hit_rate_valid;
}

mod entry_spec;
mod stats_spec;
