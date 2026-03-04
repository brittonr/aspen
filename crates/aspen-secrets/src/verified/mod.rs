//! Verified pure functions for secrets operations.
//!
//! These functions are deterministic, have no I/O, and are candidates
//! for Verus formal verification.

#[cfg(feature = "sops")]
pub mod mac;
