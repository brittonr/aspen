//! KV secrets engine.
//!
//! Provides versioned key-value secrets storage with:
//! - Multiple versions per secret
//! - Soft delete and undelete
//! - Hard delete (destroy)
//! - Check-and-set writes
//! - Metadata tracking
//! - Time-based version expiration
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use aspen_secrets::kv::{KvStore, DefaultKvStore, WriteSecretRequest, ReadSecretRequest, SecretData};
//! use aspen_secrets::backend::InMemorySecretsBackend;
//! use std::sync::Arc;
//!
//! // Create a store
//! let backend = Arc::new(InMemorySecretsBackend::new());
//! let store = DefaultKvStore::new(backend);
//!
//! // Write a secret
//! let data = SecretData::new([("username".into(), "admin".into())].into());
//! store.write(WriteSecretRequest::new("db/creds", data)).await?;
//!
//! // Read it back
//! let secret = store.read(ReadSecretRequest::new("db/creds")).await?.unwrap();
//! println!("username: {}", secret.data.get("username").unwrap());
//! ```

mod store;
mod types;

pub use store::DefaultKvStore;
pub use store::KvStore;
pub use types::DeleteSecretRequest;
pub use types::DestroySecretRequest;
pub use types::KvConfig;
pub use types::ListSecretsRequest;
pub use types::ListSecretsResponse;
pub use types::ReadMetadataRequest;
pub use types::ReadSecretRequest;
pub use types::ReadSecretResponse;
pub use types::SecretData;
pub use types::SecretMetadata;
pub use types::UndeleteSecretRequest;
pub use types::UpdateMetadataRequest;
pub use types::VersionMetadata;
pub use types::WriteSecretRequest;
pub use types::WriteSecretResponse;
