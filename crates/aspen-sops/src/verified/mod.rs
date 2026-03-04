//! Verified pure functions for SOPS operations.
//!
//! These functions are deterministic, have no I/O, and are candidates
//! for Verus formal verification. See `verus/` directory for specs.

pub mod mac;
