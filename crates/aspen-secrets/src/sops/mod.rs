//! SOPS-based secrets management.
//!
//! This module provides:
//! - Configuration for secrets management
//! - SOPS file decryption using age
//! - SecretsProvider trait for unified access

pub mod config;
pub mod decryptor;
pub mod provider;

pub use config::SecretsConfig;
pub use config::SecretsData;
pub use config::SecretsFile;
pub use config::SopsMetadata;
pub use decryptor::decrypt_secrets_file;
pub use decryptor::load_age_identity;
pub use provider::SecretsManager;
pub use provider::SecretsProvider;
