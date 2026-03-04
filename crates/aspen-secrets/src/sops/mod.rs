//! SOPS-based secrets management.
//!
//! This module provides:
//! - Configuration for secrets management
//! - SOPS file decryption using age (bootstrap)
//! - SecretsProvider trait for unified access
//! - Native SOPS encrypt/decrypt/edit/rotate using Aspen Transit (feature-gated)
//! - SOPS MAC computation and verification
//! - Multi-format support (TOML, YAML, JSON)

pub mod config;
pub mod decryptor;
pub mod provider;

// Native SOPS operations using Aspen Transit as key backend.
// Gated behind the `sops` feature to avoid pulling in aspen-client/transport deps.
#[cfg(feature = "sops")]
pub mod client;
#[cfg(feature = "sops")]
pub mod decrypt;
#[cfg(feature = "sops")]
pub mod edit;
#[cfg(feature = "sops")]
pub mod encrypt;
#[cfg(feature = "sops")]
pub mod format;
#[cfg(feature = "sops")]
pub mod mac;
#[cfg(feature = "sops")]
pub mod metadata;
#[cfg(feature = "sops")]
pub mod rotate;
#[cfg(feature = "sops")]
pub mod sops_constants;
#[cfg(feature = "sops")]
pub mod sops_error;
#[cfg(feature = "sops")]
pub mod updatekeys;

// Re-exports: existing (always available)
// Re-exports: SOPS native operations (feature-gated)
#[cfg(feature = "sops")]
pub use client::TransitClient;
pub use config::SecretsConfig;
pub use config::SecretsData;
pub use config::SecretsFile;
pub use config::SopsMetadata;
#[cfg(feature = "sops")]
pub use decrypt::DecryptConfig;
#[cfg(feature = "sops")]
pub use decrypt::decrypt_file;
pub use decryptor::decrypt_secrets_file;
pub use decryptor::load_age_identity;
#[cfg(feature = "sops")]
pub use encrypt::EncryptConfig;
#[cfg(feature = "sops")]
pub use encrypt::encrypt_file;
#[cfg(feature = "sops")]
pub use metadata::AspenTransitRecipient;
#[cfg(feature = "sops")]
pub use metadata::SopsFileMetadata;
pub use provider::SecretsManager;
pub use provider::SecretsProvider;
#[cfg(feature = "sops")]
pub use rotate::RotateConfig;
#[cfg(feature = "sops")]
pub use rotate::rotate_file;
#[cfg(feature = "sops")]
pub use sops_constants::*;
#[cfg(feature = "sops")]
pub use sops_error::Result as SopsResult;
#[cfg(feature = "sops")]
pub use sops_error::SopsError;
#[cfg(feature = "sops")]
pub use updatekeys::UpdateKeysConfig;
#[cfg(feature = "sops")]
pub use updatekeys::update_keys;
