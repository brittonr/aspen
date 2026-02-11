//! Secrets engine RPC handler for Aspen.
//!
//! This crate provides the secrets management handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles Vault-compatible secrets management:
//! - KV v2: Versioned key-value secrets with soft/hard delete
//! - Transit: Encryption-as-a-service (encrypt, decrypt, sign, verify)
//! - PKI: Certificate authority with role-based issuance
//! - Nix Cache: Signing key management for Nix binary caches

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::SecretsHandler;
pub use handler::SecretsService;
