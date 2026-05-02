//! Unified secrets management and cryptographic primitives for Aspen.
//!
//! This crate is the single owner of all secrets lifecycle, crypto operations,
//! and key management in Aspen. It consolidates functionality that was previously
//! scattered across `aspen-sops`, `aspen-auth`, `aspen-cluster`, and others.
//!
//! ## Modules
//!
//! ### SOPS Integration (`sops`)
//!
//! - **Bootstrap**: Load encrypted secrets from SOPS-formatted config files at node startup using
//!   age decryption (`sops::decryptor`)
//! - **Native SOPS** (feature `sops`): Full encrypt/decrypt/edit/rotate using Aspen Transit as key
//!   backend — replaces the Go `sops` binary
//! - **Multi-format**: TOML, YAML, JSON file support (`sops::format`)
//!
//! ### Vault-like Secrets Engines (Runtime)
//!
//! - **KV** (`kv`): Versioned key-value secrets with soft/hard delete
//! - **Transit** (`transit`): Encryption as a service (encrypt, decrypt, sign, verify)
//! - **PKI** (`pki`): Certificate authority with role-based policies
//!
//! ### Cluster Security Primitives
//!
//! - **Cookie** (`cookie`): Cluster cookie validation and HMAC key derivation
//! - **Identity** (`identity`): Node identity key lifecycle (generate, load, derive)
//! - **Nix Cache** (`nix_cache`): Signing key management for Nix binary caches
//!
//! ## Feature Flags
//!
//! - `sops`: Native SOPS encrypt/decrypt/edit/rotate using Transit (pulls in `aspen-client`,
//!   `toml_edit`, `serde_yaml`, etc.)
//! - `auth-runtime`: TokenBuilder/TokenVerifier convenience helpers backed by `aspen-auth`
//! - `full`: All features enabled
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
//! // With the `auth-runtime` feature, build a token verifier with trusted roots
//! let verifier = manager.build_token_verifier();
//!
//! // Access secrets
//! let api_key = manager.get_string("api_key").await?;
//! ```

pub mod backend;
pub mod constants;
pub mod cookie;
pub mod error;
pub mod identity;
pub mod kv;
pub mod mount_registry;
pub mod nix_cache;
pub mod pki;
pub mod sops;
pub mod transit;
pub mod verified;

// Re-export main types
// Re-export backend types
pub use backend::AspenSecretsBackend;
pub use backend::InMemorySecretsBackend;
pub use backend::SecretsBackend;
#[cfg(feature = "trust")]
pub use backend::SecretsEncryptionProvider;
pub use constants::*;
pub use error::Result;
pub use error::SecretsError;
// Re-export KV types
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
