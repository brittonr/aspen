//! Vault client helper for convenient vault operations.
//!
//! Provides a high-level API for working with vaults, automatically
//! handling key prefixing and parsing.
//!
//! # Example
//!
//! ```ignore
//! use aspen::client::VaultClient;
//!
//! // Create vault client from a cluster subscription or KV store
//! let vault = VaultClient::new("myapp", kv_store);
//!
//! // Write to vault (key is automatically prefixed with "vault:myapp:")
//! vault.write("config", "value").await?;
//!
//! // Read from vault
//! let value = vault.read("config").await?;
//!
//! // List all keys in vault
//! let keys = vault.list_keys().await?;
//! ```

use std::sync::Arc;

use crate::api::vault::{
    VaultError, make_vault_key, parse_vault_key, validate_vault_name, vault_scan_prefix,
};
use crate::api::{
    DeleteRequest, KeyValueStore, KeyValueStoreError, ReadRequest, ScanRequest, WriteCommand,
    WriteRequest,
};

/// A client for performing operations on a specific vault.
///
/// This helper automatically prefixes all keys with `vault:{name}:` and
/// strips the prefix from scan results.
pub struct VaultClient {
    /// The vault name.
    name: String,
    /// The underlying key-value store.
    kv: Arc<dyn KeyValueStore>,
}

impl VaultClient {
    /// Create a new vault client for the given vault name.
    ///
    /// # Errors
    ///
    /// Returns an error if the vault name is invalid.
    pub fn new(name: impl Into<String>, kv: Arc<dyn KeyValueStore>) -> Result<Self, VaultError> {
        let name = name.into();
        validate_vault_name(&name).map_err(|reason| VaultError::InvalidVaultName {
            name: name.clone(),
            reason,
        })?;
        Ok(Self { name, kv })
    }

    /// Get the vault name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Write a key-value pair to the vault.
    ///
    /// The key will be prefixed with `vault:{name}:`.
    pub async fn write(&self, key: &str, value: impl Into<String>) -> Result<(), VaultClientError> {
        let full_key = make_vault_key(&self.name, key);

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: full_key,
                    value: value.into(),
                },
            })
            .await
            .map_err(VaultClientError::Store)?;

        Ok(())
    }

    /// Write multiple key-value pairs to the vault atomically.
    pub async fn write_multi(&self, pairs: Vec<(String, String)>) -> Result<(), VaultClientError> {
        let prefixed_pairs: Vec<(String, String)> = pairs
            .into_iter()
            .map(|(k, v)| (make_vault_key(&self.name, &k), v))
            .collect();

        self.kv
            .write(WriteRequest {
                command: WriteCommand::SetMulti {
                    pairs: prefixed_pairs,
                },
            })
            .await
            .map_err(VaultClientError::Store)?;

        Ok(())
    }

    /// Read a value from the vault.
    ///
    /// Returns `None` if the key doesn't exist.
    pub async fn read(&self, key: &str) -> Result<Option<String>, VaultClientError> {
        let full_key = make_vault_key(&self.name, key);

        match self.kv.read(ReadRequest { key: full_key }).await {
            Ok(result) => Ok(Some(result.value)),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(VaultClientError::Store(e)),
        }
    }

    /// Delete a key from the vault.
    ///
    /// Returns true if the key existed and was deleted.
    pub async fn delete(&self, key: &str) -> Result<bool, VaultClientError> {
        let full_key = make_vault_key(&self.name, key);

        let result = self
            .kv
            .delete(DeleteRequest { key: full_key })
            .await
            .map_err(VaultClientError::Store)?;

        Ok(result.deleted)
    }

    /// Delete multiple keys from the vault atomically.
    pub async fn delete_multi(&self, keys: Vec<String>) -> Result<(), VaultClientError> {
        let prefixed_keys: Vec<String> = keys
            .into_iter()
            .map(|k| make_vault_key(&self.name, &k))
            .collect();

        self.kv
            .write(WriteRequest {
                command: WriteCommand::DeleteMulti {
                    keys: prefixed_keys,
                },
            })
            .await
            .map_err(VaultClientError::Store)?;

        Ok(())
    }

    /// List all keys in the vault.
    ///
    /// Returns key names without the vault prefix.
    pub async fn list_keys(&self) -> Result<Vec<String>, VaultClientError> {
        self.list_keys_with_prefix("").await
    }

    /// List keys in the vault with a prefix filter.
    ///
    /// The prefix is applied within the vault (not the full key).
    pub async fn list_keys_with_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<String>, VaultClientError> {
        let scan_prefix = format!("{}{}", vault_scan_prefix(&self.name), prefix);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: scan_prefix,
                limit: Some(10_000),
                continuation_token: None,
            })
            .await
            .map_err(VaultClientError::Store)?;

        let keys: Vec<String> = result
            .entries
            .into_iter()
            .filter_map(|entry| parse_vault_key(&entry.key).map(|(_, k)| k))
            .collect();

        Ok(keys)
    }

    /// List all key-value pairs in the vault.
    pub async fn list_all(&self) -> Result<Vec<(String, String)>, VaultClientError> {
        self.list_all_with_prefix("").await
    }

    /// List key-value pairs in the vault with a prefix filter.
    pub async fn list_all_with_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, String)>, VaultClientError> {
        let scan_prefix = format!("{}{}", vault_scan_prefix(&self.name), prefix);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: scan_prefix,
                limit: Some(10_000),
                continuation_token: None,
            })
            .await
            .map_err(VaultClientError::Store)?;

        let pairs: Vec<(String, String)> = result
            .entries
            .into_iter()
            .filter_map(|entry| parse_vault_key(&entry.key).map(|(_, k)| (k, entry.value)))
            .collect();

        Ok(pairs)
    }

    /// Check if a key exists in the vault.
    pub async fn exists(&self, key: &str) -> Result<bool, VaultClientError> {
        Ok(self.read(key).await?.is_some())
    }

    /// Get the count of keys in the vault.
    ///
    /// Note: This scans all keys, which may be slow for large vaults.
    pub async fn count(&self) -> Result<usize, VaultClientError> {
        Ok(self.list_keys().await?.len())
    }
}

/// Errors that can occur during vault client operations.
#[derive(Debug, thiserror::Error)]
pub enum VaultClientError {
    /// Error from the underlying key-value store.
    #[error("store error: {0}")]
    Store(#[from] KeyValueStoreError),

    /// Vault-specific error.
    #[error("vault error: {0}")]
    Vault(#[from] VaultError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_vault_client_write_read() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        vault.write("config", "value123").await.unwrap();
        let result = vault.read("config").await.unwrap();
        assert_eq!(result, Some("value123".to_string()));
    }

    #[tokio::test]
    async fn test_vault_client_read_missing() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        let result = vault.read("missing").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_vault_client_delete() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        vault.write("key", "value").await.unwrap();
        assert!(vault.exists("key").await.unwrap());

        let deleted = vault.delete("key").await.unwrap();
        assert!(deleted);
        assert!(!vault.exists("key").await.unwrap());

        // Deleting again should return false
        let deleted_again = vault.delete("key").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_vault_client_list_keys() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        vault.write("a", "1").await.unwrap();
        vault.write("b", "2").await.unwrap();
        vault.write("c", "3").await.unwrap();

        let keys = vault.list_keys().await.unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    #[tokio::test]
    async fn test_vault_client_list_with_prefix() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        vault.write("config/a", "1").await.unwrap();
        vault.write("config/b", "2").await.unwrap();
        vault.write("other", "3").await.unwrap();

        let keys = vault.list_keys_with_prefix("config/").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"config/a".to_string()));
        assert!(keys.contains(&"config/b".to_string()));
    }

    #[tokio::test]
    async fn test_vault_client_write_multi() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        vault
            .write_multi(vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ])
            .await
            .unwrap();

        assert_eq!(vault.read("a").await.unwrap(), Some("1".to_string()));
        assert_eq!(vault.read("b").await.unwrap(), Some("2".to_string()));
    }

    #[tokio::test]
    async fn test_vault_client_invalid_name() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();

        let result = VaultClient::new("bad name", kv);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vault_client_count() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let vault = VaultClient::new("myapp", kv).unwrap();

        assert_eq!(vault.count().await.unwrap(), 0);

        vault.write("a", "1").await.unwrap();
        vault.write("b", "2").await.unwrap();

        assert_eq!(vault.count().await.unwrap(), 2);
    }
}
