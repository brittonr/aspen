//! Pure cryptographic identity admission and currentness laws.
//!
//! This module never receives private key bytes, backend clients, random
//! generators, files, clocks, Iroh objects, or cryptographic library types.

mod admission;
mod artifact_auth;
mod model;
mod rotation;

pub use admission::*;
pub use artifact_auth::*;
pub use model::*;
pub use rotation::*;

#[cfg(test)]
mod tests;
