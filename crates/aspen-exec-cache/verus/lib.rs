//! Verus specifications for aspen-exec-cache.
//!
//! # Invariants Verified
//!
//! ## Cache Key Computation
//!
//! 1. **KEY-1: Determinism**: Same inputs always produce the same cache key, regardless of input
//!    file access order.
//! 2. **KEY-2: Sorted Input Invariant**: Input hashes are sorted before hashing, ensuring
//!    access-order independence.
//! 3. **KEY-3: Environment Inclusion**: The environment hash is included in the cache key, so
//!    environment changes invalidate cached results.
//! 4. **KEY-4: Length-Prefixed Framing**: All variable-length fields use length prefixes to prevent
//!    concatenation collisions.

mod cache_key_spec;

pub use cache_key_spec::*;
