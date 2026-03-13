//! Pure, deterministic functions suitable for Verus formal verification.
//!
//! All functions in this module have no I/O, no async, and no time dependency.
//! Time, randomness, and configuration are passed as explicit parameters.
//!
//! Formally verified — see `verus/scan_merge_spec.rs` for proofs.

pub mod scan_merge;
