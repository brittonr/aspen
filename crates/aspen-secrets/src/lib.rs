//! SOPS-based secrets management and Vault-like secrets engine for Aspen.
//!
//! This crate provides comprehensive secrets management:
//!
//! ## SOPS Integration (Bootstrap Secrets)
//!
//! Load encrypted secrets from SOPS-formatted config files at node startup:
//! - Trusted root keys for token verification
//! - Token signing keys
//! - Pre-built capability tokens
//! - Bootstrap secrets (API keys, certificates, etc.)
//!
//! ## Vault-like Secrets Engines (Runtime)
//!
//! - **KV v2**: Versioned key-value secrets with soft/hard delete
//! - **Transit**: Encryption as a service (encrypt, decrypt, sign, verify)
//! - **PKI**: Certificate authority with role-based policies
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use aspen_secrets::{SecretsConfig, SecretsManager, decrypt_secrets_file, load_age_identity};
//!
//! // Load age identity for decryption
//! let identity = load_age_identity(
//!     Some(Path::new("/etc/aspen/age-key.txt")),
//!     "SOPS_AGE_KEY",
//! ).await?;
//!
//! // Decrypt secrets file
//! let secrets = decrypt_secrets_file(
//!     Path::new("/etc/aspen/secrets.sops.toml"),
//!     &identity,
//! ).await?;
//!
//! // Create secrets manager
//! let config = SecretsConfig::with_secrets_file("/etc/aspen/secrets.sops.toml");
//! let manager = SecretsManager::new(config, secrets)?;
//!
//! // Build token verifier with trusted roots
//! let verifier = manager.build_token_verifier();
//!
//! // Access secrets
//! let api_key = manager.get_string("api_key").await?;
//! ```

pub mod backend;
pub mod constants;
pub mod error;
pub mod kv;
pub mod mount_registry;
pub mod pki;
pub mod sops;
pub mod transit;

// Re-export main types
// Re-export backend types
pub use backend::AspenSecretsBackend;
pub use backend::InMemorySecretsBackend;
pub use backend::SecretsBackend;
pub use constants::*;
pub use error::Result;
pub use error::SecretsError;
// Re-export KV v2 types
pub use kv::DefaultKvStore;
pub use kv::KvConfig;
pub use kv::KvStore;
pub use kv::SecretData;
pub use kv::SecretMetadata;
// Re-export Mount Registry
pub use mount_registry::MountRegistry;
// Re-export PKI types
pub use pki::DefaultPkiStore;
pub use pki::PkiKeyType;
pub use pki::PkiRole;
pub use pki::PkiStore;
pub use sops::SecretsConfig;
pub use sops::SecretsData;
pub use sops::SecretsFile;
pub use sops::SecretsManager;
pub use sops::SecretsProvider;
pub use sops::SopsMetadata;
pub use sops::decrypt_secrets_file;
pub use sops::load_age_identity;
// Re-export Transit types
pub use transit::DefaultTransitStore;
pub use transit::KeyType;
pub use transit::TransitKey;
pub use transit::TransitStore;
