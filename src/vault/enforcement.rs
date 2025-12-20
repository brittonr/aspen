//! Vault quota enforcement.
//!
//! Tracks key counts per vault and enforces limits via synchronous metadata updates.
//!
//! # Metadata Storage
//!
//! Vault metadata is stored at `_system:vault_meta:{vault_name}` and contains:
//! - `key_count`: Number of keys currently in the vault
//! - `created_at`: Unix timestamp when vault was first created
//!
//! # Quota Enforcement
//!
//! Before a write to a vault key, we:
//! 1. Read current metadata (or create if new vault)
//! 2. Check if quota would be exceeded
//! 3. Perform atomic write of key + updated metadata via SetMulti
//!
//! # Tiger Style
//!
//! - MAX_KEYS_PER_VAULT: 10,000 keys per vault
//! - MAX_VAULTS: 1,000 total vaults

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{debug, warn};

use crate::api::vault::{
    MAX_KEYS_PER_VAULT, MAX_VAULTS, VAULT_META_PREFIX, VaultError, VaultMetadata, parse_vault_key,
};
use crate::api::{
    KeyValueStore, KeyValueStoreError, ReadRequest, ScanRequest, WriteCommand, WriteRequest,
};

/// Enforces vault quotas using metadata stored in the system namespace.
///
/// This struct provides methods to check and update vault quotas before/after
/// key operations. It uses the provided KeyValueStore for metadata storage.
pub struct VaultEnforcer {
    /// The underlying KV store for metadata access.
    kv: Arc<dyn KeyValueStore>,
}

impl VaultEnforcer {
    /// Create a new vault enforcer.
    pub fn new(kv: Arc<dyn KeyValueStore>) -> Self {
        Self { kv }
    }

    /// Check if a vault write would exceed quotas.
    ///
    /// Returns Ok(()) if the write is allowed, or an error if:
    /// - MAX_KEYS_PER_VAULT would be exceeded
    /// - MAX_VAULTS would be exceeded (for new vaults)
    ///
    /// This should be called before performing the actual write.
    pub async fn check_write_quota(&self, key: &str) -> Result<(), VaultError> {
        // Only check quota for vault keys
        let Some((vault_name, _)) = parse_vault_key(key) else {
            return Ok(());
        };

        // Get current metadata
        let metadata = self.get_metadata(&vault_name).await;

        match metadata {
            Some(meta) => {
                // Existing vault - check key quota
                if meta.key_count >= MAX_KEYS_PER_VAULT as u64 {
                    return Err(VaultError::VaultQuotaExceeded {
                        vault: vault_name,
                        limit: MAX_KEYS_PER_VAULT,
                    });
                }
            }
            None => {
                // New vault - check total vault count
                let vault_count = self.count_vaults().await;
                if vault_count >= MAX_VAULTS {
                    return Err(VaultError::TooManyVaults { limit: MAX_VAULTS });
                }
            }
        }

        Ok(())
    }

    /// Update metadata after a successful vault key write.
    ///
    /// Increments the key count for the vault. If this is a new vault,
    /// creates the metadata entry.
    ///
    /// Note: In the full implementation, this should be done atomically
    /// with the key write using SetMulti. This method is for updating
    /// after separate writes (less safe, but simpler for initial impl).
    pub async fn increment_key_count(&self, key: &str) -> Result<(), KeyValueStoreError> {
        let Some((vault_name, _)) = parse_vault_key(key) else {
            return Ok(());
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let new_meta = match self.get_metadata(&vault_name).await {
            Some(mut meta) => {
                meta.key_count = meta.key_count.saturating_add(1);
                meta
            }
            None => VaultMetadata::new(1, now),
        };

        self.set_metadata(&vault_name, &new_meta).await
    }

    /// Update metadata after a successful vault key delete.
    ///
    /// Decrements the key count for the vault. If the count reaches zero,
    /// the metadata is removed.
    pub async fn decrement_key_count(&self, key: &str) -> Result<(), KeyValueStoreError> {
        let Some((vault_name, _)) = parse_vault_key(key) else {
            return Ok(());
        };

        let Some(mut meta) = self.get_metadata(&vault_name).await else {
            // No metadata means no vault, nothing to decrement
            return Ok(());
        };

        if meta.key_count <= 1 {
            // Last key deleted, remove metadata
            self.delete_metadata(&vault_name).await
        } else {
            meta.key_count = meta.key_count.saturating_sub(1);
            self.set_metadata(&vault_name, &meta).await
        }
    }

    /// Check if a key exists in the store.
    ///
    /// Used to determine if a write is creating a new key or updating existing.
    pub async fn key_exists(&self, key: &str) -> bool {
        self.kv
            .read(ReadRequest {
                key: key.to_string(),
            })
            .await
            .is_ok()
    }

    /// Get metadata for a vault.
    async fn get_metadata(&self, vault_name: &str) -> Option<VaultMetadata> {
        let meta_key = VaultMetadata::storage_key(vault_name);

        match self.kv.read(ReadRequest { key: meta_key }).await {
            Ok(result) => match serde_json::from_str(&result.value) {
                Ok(meta) => Some(meta),
                Err(e) => {
                    warn!(vault = %vault_name, error = %e, "failed to parse vault metadata");
                    None
                }
            },
            Err(KeyValueStoreError::NotFound { .. }) => None,
            Err(e) => {
                warn!(vault = %vault_name, error = %e, "failed to read vault metadata");
                None
            }
        }
    }

    /// Set metadata for a vault.
    async fn set_metadata(
        &self,
        vault_name: &str,
        metadata: &VaultMetadata,
    ) -> Result<(), KeyValueStoreError> {
        let meta_key = VaultMetadata::storage_key(vault_name);
        let value = serde_json::to_string(metadata).map_err(|e| KeyValueStoreError::Failed {
            reason: format!("failed to serialize vault metadata: {}", e),
        })?;

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: meta_key,
                    value,
                },
            })
            .await?;

        debug!(vault = %vault_name, key_count = %metadata.key_count, "updated vault metadata");
        Ok(())
    }

    /// Delete metadata for a vault.
    async fn delete_metadata(&self, vault_name: &str) -> Result<(), KeyValueStoreError> {
        let meta_key = VaultMetadata::storage_key(vault_name);

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Delete { key: meta_key },
            })
            .await?;

        debug!(vault = %vault_name, "deleted vault metadata");
        Ok(())
    }

    /// Count the number of vaults by scanning metadata keys.
    async fn count_vaults(&self) -> usize {
        match self
            .kv
            .scan(ScanRequest {
                prefix: VAULT_META_PREFIX.to_string(),
                limit: Some(MAX_VAULTS as u32 + 1),
                continuation_token: None,
            })
            .await
        {
            Ok(result) => result.entries.len(),
            Err(e) => {
                warn!(error = %e, "failed to count vaults");
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_enforcer_allows_vault_write() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let enforcer = VaultEnforcer::new(kv);

        // Should allow first write to new vault
        assert!(
            enforcer
                .check_write_quota("vault:myapp:config")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_enforcer_allows_non_vault_write() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let enforcer = VaultEnforcer::new(kv);

        // Non-vault keys should always be allowed (no quota)
        assert!(enforcer.check_write_quota("regular_key").await.is_ok());
        assert!(enforcer.check_write_quota("any:other:format").await.is_ok());
    }

    #[tokio::test]
    async fn test_enforcer_increments_key_count() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let kv_clone = kv.clone();
        let enforcer = VaultEnforcer::new(kv);

        // Increment for new vault
        enforcer
            .increment_key_count("vault:myapp:key1")
            .await
            .unwrap();

        // Check metadata was created
        let meta_key = VaultMetadata::storage_key("myapp");
        let result = kv_clone.read(ReadRequest { key: meta_key }).await.unwrap();
        let meta: VaultMetadata = serde_json::from_str(&result.value).unwrap();
        assert_eq!(meta.key_count, 1);

        // Increment again
        enforcer
            .increment_key_count("vault:myapp:key2")
            .await
            .unwrap();

        let result = kv_clone
            .read(ReadRequest {
                key: VaultMetadata::storage_key("myapp"),
            })
            .await
            .unwrap();
        let meta: VaultMetadata = serde_json::from_str(&result.value).unwrap();
        assert_eq!(meta.key_count, 2);
    }

    #[tokio::test]
    async fn test_enforcer_decrements_key_count() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let kv_clone = kv.clone();
        let enforcer = VaultEnforcer::new(kv);

        // Set up vault with 2 keys
        enforcer
            .increment_key_count("vault:myapp:key1")
            .await
            .unwrap();
        enforcer
            .increment_key_count("vault:myapp:key2")
            .await
            .unwrap();

        // Decrement
        enforcer
            .decrement_key_count("vault:myapp:key1")
            .await
            .unwrap();

        let result = kv_clone
            .read(ReadRequest {
                key: VaultMetadata::storage_key("myapp"),
            })
            .await
            .unwrap();
        let meta: VaultMetadata = serde_json::from_str(&result.value).unwrap();
        assert_eq!(meta.key_count, 1);

        // Decrement to zero - should delete metadata
        enforcer
            .decrement_key_count("vault:myapp:key2")
            .await
            .unwrap();

        let result = kv_clone
            .read(ReadRequest {
                key: VaultMetadata::storage_key("myapp"),
            })
            .await;
        assert!(matches!(result, Err(KeyValueStoreError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_enforcer_ignores_non_vault_keys() {
        let kv: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
        let enforcer = VaultEnforcer::new(kv);

        // Non-vault keys should not affect metadata
        enforcer.increment_key_count("regular_key").await.unwrap();
        enforcer.decrement_key_count("regular_key").await.unwrap();

        // No error, no metadata created
    }
}
