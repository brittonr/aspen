//! Horizontal sharding and consistent hash routing for Aspen.
//!
//! This module re-exports the `aspen-sharding` crate for backward compatibility.
//! New code should depend on `aspen-sharding` directly.
//!
//! See [`aspen_sharding`] for full documentation.

// Re-export everything from aspen-sharding
pub use aspen_sharding::*;

// Re-export submodules for direct access
pub use aspen_sharding::automation;
pub use aspen_sharding::consistent_hash;
pub use aspen_sharding::metrics;
pub use aspen_sharding::router;
pub use aspen_sharding::sharded;
pub use aspen_sharding::topology;
