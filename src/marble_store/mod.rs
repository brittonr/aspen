//! Bounded marble object-store pilot behind the local object storage seam.
//!
//! This module compiles only under the `marble-store` feature. The spike
//! backend stores whole objects in a marble heap keyed by allocated physical
//! handles, indexes fixed 32-byte BLAKE3 digests to those handles with an
//! adaptive radix trie, serves in-flight batch mappings from its write cache,
//! and owns maintenance scheduling. BLAKE3 remains the only content identity
//! and the current storage path stays the default.
//!
//! The pilot records canonical receipts and claims nothing beyond recorded
//! measurements. See `docs/marble-object-store-pilot.md` for the boundary.

mod backend;
mod canonical;
mod measure;

pub use backend::*;
pub use canonical::*;
pub use measure::*;

#[cfg(test)]
mod tests;
