//! Verus Formal Specifications for Aspen Sharding
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of consistent hashing and shard routing in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-sharding          # Verify all specs
//! nix run .#verify-verus-sharding -- quick # Syntax check only
//! nix run .#verify-verus                   # Verify all specs
//! ```
//!
//! # Module Overview
//!
//! ## Consistent Hashing
//! - `hash_spec`: Jump consistent hash properties and bounds
//!
//! ## Shard Routing
//! - `router_spec`: Shard router invariants
//!
//! ## Node ID Encoding
//! - `encoding_spec`: Shard/physical node ID bit packing
//!
//! # Invariants Verified
//!
//! ## Hash Properties
//!
//! 1. **SHARD-1: Hash Bounds**: hash(key, n) always in [0, n)
//!    - Result never exceeds num_buckets - 1
//!    - Prevents invalid shard routing
//!
//! 2. **SHARD-2: Determinism**: Same key always hashes to same bucket
//!    - Required for consistent routing
//!
//! 3. **SHARD-3: Uniform Distribution**: Keys distribute evenly across buckets
//!    - Each bucket gets approximately 1/n of keys
//!
//! ## Shard Bounds
//!
//! 4. **SHARD-4: Shard Limits**: num_shards in [MIN_SHARDS, MAX_SHARDS]
//!    - MIN_SHARDS = 1, MAX_SHARDS = 256
//!    - Enforced at ShardConfig construction
//!
//! ## Bit Packing
//!
//! 5. **SHARD-5: Encoding Roundtrip**: decode(encode(shard, node)) == (shard, node)
//!    - 16 bits for shard ID, 48 bits for physical node ID
//!
//! 6. **SHARD-6: Backward Compatibility**: encode(node, 0) == node
//!    - Shard 0 produces unchanged physical IDs
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Rust's std::hash::DefaultHasher is deterministic
//! - Floating point operations are consistent across platforms
//! - Bit operations follow two's complement semantics

use vstd::prelude::*;

verus! {
    // Re-export hash specifications
    pub use hash_spec::hash_bounds;
    pub use hash_spec::hash_deterministic;
    pub use hash_spec::uniform_distribution;

    // Re-export router specifications
    pub use router_spec::ShardConfigSpec;
    pub use router_spec::shard_limits;
    pub use router_spec::router_invariant;

    // Re-export encoding specifications
    pub use encoding_spec::encoding_roundtrip;
    pub use encoding_spec::backward_compatible;
    pub use encoding_spec::shard_id_extractable;
}

mod encoding_spec;
mod hash_spec;
mod router_spec;
