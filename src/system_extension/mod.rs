//! Canonical system-extension artifacts and the executable callback host.
//!
//! Pure admission, lifecycle, dispatch, resource, and supervision laws are
//! re-exported from `molten-core`. This module adds canonical Preserves/BLAKE3
//! identity and a thin imperative executor shell. It is intentionally separate
//! from the ordinary receipt-first plugin host.

mod canonical;
mod fixture;
mod host;

#[cfg(test)]
mod tests;

pub use canonical::*;
pub use fixture::*;
pub use host::*;
pub use molten_core::system_extension::*;
