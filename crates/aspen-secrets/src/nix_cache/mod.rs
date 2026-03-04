//! Nix binary cache signing key management.
//!
//! Manages Ed25519 signing keys for Nix binary cache narinfo signatures,
//! backed by the Transit secrets engine.
//!
//! Keys are stored as Transit Ed25519 keys. The public key is formatted
//! as `{cache_name}:{base64_public_key}` matching the Nix narinfo `Sig:` format.
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen_secrets::nix_cache::NixCacheKeyManager;
//!
//! let manager = NixCacheKeyManager::new(transit_store);
//! let info = manager.create_signing_key("my-cache").await?;
//! println!("Public key: {}", info.public_key_string);
//! ```

use std::sync::Arc;

use base64::Engine;

use crate::error::SecretsError;
use crate::transit::CreateKeyRequest;
use crate::transit::KeyType;
use crate::transit::TransitStore;

/// Information about a Nix cache signing key.
#[derive(Debug, Clone)]
pub struct NixCachePublicKeyInfo {
    /// Cache name (key name in Transit).
    pub cache_name: String,
    /// Formatted public key string: `{cache_name}:{base64_pubkey}`.
    pub public_key_string: String,
    /// Raw public key bytes.
    pub public_key_bytes: Vec<u8>,
}

/// Manages Nix cache signing keys via Transit.
pub struct NixCacheKeyManager {
    transit_store: Arc<dyn TransitStore>,
}

impl NixCacheKeyManager {
    /// Create a new key manager backed by the given Transit store.
    pub fn new(transit_store: Arc<dyn TransitStore>) -> Self {
        Self { transit_store }
    }

    /// Create a new Ed25519 signing key for a Nix cache.
    ///
    /// The key is created as a non-exportable, deletion-allowed Transit key.
    pub async fn create_signing_key(&self, cache_name: &str) -> Result<NixCachePublicKeyInfo, SecretsError> {
        let request = CreateKeyRequest {
            name: cache_name.to_string(),
            key_type: KeyType::Ed25519,
            exportable: false,
            deletion_allowed: true,
            convergent_encryption: false,
        };

        self.transit_store.create_key(request).await?;
        self.get_public_key(cache_name).await?.ok_or(SecretsError::TransitKeyNotFound {
            name: cache_name.to_string(),
        })
    }

    /// Get the public key for a cache signing key.
    ///
    /// Returns `None` if the key doesn't exist.
    pub async fn get_public_key(&self, cache_name: &str) -> Result<Option<NixCachePublicKeyInfo>, SecretsError> {
        let transit_key = match self.transit_store.read_key(cache_name).await? {
            Some(key) => key,
            None => return Ok(None),
        };

        let current_version = transit_key.versions.get(&transit_key.current_version).ok_or(SecretsError::Internal {
            reason: format!("current version {} not found for key '{cache_name}'", transit_key.current_version),
        })?;

        let public_key_bytes = current_version.public_key.as_ref().ok_or(SecretsError::Internal {
            reason: format!("key '{cache_name}' does not have a public key (not an asymmetric key)"),
        })?;

        let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);
        let public_key_string = format!("{cache_name}:{public_key_b64}");

        Ok(Some(NixCachePublicKeyInfo {
            cache_name: cache_name.to_string(),
            public_key_string,
            public_key_bytes: public_key_bytes.clone(),
        }))
    }

    /// Rotate the signing key for a cache.
    ///
    /// Creates a new key version while keeping previous versions for verification.
    pub async fn rotate_signing_key(&self, cache_name: &str) -> Result<NixCachePublicKeyInfo, SecretsError> {
        self.transit_store.rotate_key(cache_name).await?;
        self.get_public_key(cache_name).await?.ok_or(SecretsError::TransitKeyNotFound {
            name: cache_name.to_string(),
        })
    }

    /// Delete a cache signing key.
    ///
    /// Returns `true` if the key was deleted.
    pub async fn delete_signing_key(&self, cache_name: &str) -> Result<bool, SecretsError> {
        self.transit_store.delete_key(cache_name).await?;
        Ok(true)
    }

    /// List all signing key names.
    pub async fn list_signing_keys(&self) -> Result<Vec<String>, SecretsError> {
        self.transit_store.list_keys().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemorySecretsBackend;

    async fn test_manager() -> NixCacheKeyManager {
        let backend = Arc::new(InMemorySecretsBackend::new());
        let store: Arc<dyn TransitStore> = Arc::new(crate::DefaultTransitStore::new(backend));
        NixCacheKeyManager::new(store)
    }

    #[tokio::test]
    async fn test_create_signing_key() {
        let manager = test_manager().await;
        let info = manager.create_signing_key("test-cache").await.unwrap();
        assert_eq!(info.cache_name, "test-cache");
        assert!(info.public_key_string.starts_with("test-cache:"));
        assert!(!info.public_key_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_get_public_key() {
        let manager = test_manager().await;
        manager.create_signing_key("my-cache").await.unwrap();

        let info = manager.get_public_key("my-cache").await.unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().cache_name, "my-cache");
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let manager = test_manager().await;
        let result = manager.get_public_key("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_rotate_signing_key() {
        let manager = test_manager().await;
        let original = manager.create_signing_key("rotate-test").await.unwrap();
        let rotated = manager.rotate_signing_key("rotate-test").await.unwrap();

        // Public key should change after rotation
        assert_ne!(original.public_key_bytes, rotated.public_key_bytes);
        assert_eq!(rotated.cache_name, "rotate-test");
    }

    #[tokio::test]
    async fn test_delete_signing_key() {
        let manager = test_manager().await;
        manager.create_signing_key("delete-test").await.unwrap();

        let deleted = manager.delete_signing_key("delete-test").await.unwrap();
        assert!(deleted);

        let result = manager.get_public_key("delete-test").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_signing_keys() {
        let manager = test_manager().await;
        manager.create_signing_key("cache-a").await.unwrap();
        manager.create_signing_key("cache-b").await.unwrap();

        let keys = manager.list_signing_keys().await.unwrap();
        assert!(keys.contains(&"cache-a".to_string()));
        assert!(keys.contains(&"cache-b".to_string()));
    }
}
