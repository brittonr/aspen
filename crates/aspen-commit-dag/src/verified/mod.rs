//! Pure, deterministic functions suitable for Verus formal verification.
//!
//! All functions in this module have no I/O, no async, and no time dependency.
//! Time, randomness, and configuration are passed as explicit parameters.
//!
//! Formally verified — see `verus/` for proofs.

pub mod commit_hash;
pub mod diff;
