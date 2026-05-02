//! Compatibility re-exports for Transit secrets engine data types.
//!
//! Portable Transit request/response and key state types are owned by
//! `aspen-secrets-core`; this module keeps historical
//! `aspen_secrets::transit::types::*` paths working.

pub use aspen_secrets_core::transit::*;
