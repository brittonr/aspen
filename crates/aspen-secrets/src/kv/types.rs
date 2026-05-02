//! Compatibility re-exports for KV secrets engine data types.
//!
//! Portable KV request/response and state types are owned by
//! `aspen-secrets-core`; this module keeps historical
//! `aspen_secrets::kv::types::*` paths working.

pub use aspen_secrets_core::kv::*;
