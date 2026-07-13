//! Pure cryptographic identity admission and currentness laws.
//!
//! This module never receives private key bytes, backend clients, random
//! generators, files, clocks, Iroh objects, or cryptographic library types.

mod admission;
mod model;
mod rotation;

pub use admission::*;
pub use model::*;
pub use rotation::*;

#[cfg(test)]
mod tests;
