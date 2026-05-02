//! Portable secrets engine types, state, and resource limits for Aspen.
//!
//! This crate owns dependency-light data contracts for Aspen secrets engines.
//! Runtime storage, cryptographic execution, SOPS IO, and handler adapters stay
//! in `aspen-secrets` and integration crates.

pub mod constants;
pub mod kv;
pub mod pki;
pub mod transit;

pub use constants::*;
