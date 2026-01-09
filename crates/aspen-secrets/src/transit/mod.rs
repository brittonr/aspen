//! Transit secrets engine (encryption as a service).
//!
//! Provides encryption operations without exposing keys:
//! - Symmetric encryption (XChaCha20-Poly1305, AES-256-GCM)
//! - Signing/verification (Ed25519)
//! - Key rotation with versioning
//! - Data key generation for envelope encryption
//! - Rewrap operations for key rotation
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use aspen_secrets::transit::{TransitStore, DefaultTransitStore, CreateKeyRequest, KeyType, EncryptRequest};
//! use aspen_secrets::backend::InMemorySecretsBackend;
//! use std::sync::Arc;
//!
//! // Create a store
//! let backend = Arc::new(InMemorySecretsBackend::new());
//! let store = DefaultTransitStore::new(backend);
//!
//! // Create an encryption key
//! store.create_key(CreateKeyRequest::new("my-key").with_type(KeyType::XChaCha20Poly1305)).await?;
//!
//! // Encrypt data
//! let response = store.encrypt(EncryptRequest::new("my-key", b"secret".to_vec())).await?;
//! println!("Ciphertext: {}", response.ciphertext);
//!
//! // Decrypt data
//! let decrypted = store.decrypt(DecryptRequest::new("my-key", response.ciphertext)).await?;
//! ```

mod store;
mod types;

pub use store::DefaultTransitStore;
pub use store::TransitStore;
pub use types::BatchDecryptInput;
pub use types::BatchEncryptInput;
pub use types::BatchResultItem;
pub use types::CreateKeyRequest;
pub use types::DataKeyRequest;
pub use types::DataKeyResponse;
pub use types::DecryptRequest;
pub use types::DecryptResponse;
pub use types::EncryptRequest;
pub use types::EncryptResponse;
pub use types::KeyType;
pub use types::KeyVersion;
pub use types::RewrapRequest;
pub use types::RewrapResponse;
pub use types::SignRequest;
pub use types::SignResponse;
pub use types::TransitKey;
pub use types::UpdateKeyConfigRequest;
pub use types::VerifyRequest;
pub use types::VerifyResponse;
