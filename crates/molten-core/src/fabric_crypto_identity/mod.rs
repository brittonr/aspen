//! Pure cryptographic identity admission and currentness laws.
//!
//! This module never receives private key bytes, backend clients, random
//! generators, files, clocks, Iroh objects, or cryptographic library types.

mod admission;
#[path = "artifact/auth.rs"]
mod artifact_auth;
mod model;
mod rotation;
mod tpm2;

pub use admission::*;
pub use artifact_auth::*;
pub use model::*;
pub use rotation::*;
pub use tpm2::*;

#[cfg(test)]
mod tests;
