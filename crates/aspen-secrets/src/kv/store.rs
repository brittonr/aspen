//! KV v2 secrets store implementation.
//!
//! Provides versioned key-value storage with soft delete, hard delete (destroy),
//! and check-and-set semantics following HashiCorp Vault patterns.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use async_trait::async_trait;
use tracing::debug;
use tracing::warn;

use crate::backend::SecretsBackend;
use crate::constants::MAX_KV_PAIRS_PER_SECRET;
use crate::constants::MAX_KV_SECRET_SIZE;
use crate::constants::MAX_SECRET_KEY_NAME_LENGTH;
use crate::constants::MAX_SECRET_PATH_LENGTH;
use crate::constants::MAX_VERSIONS_PER_SECRET;
use crate::error::Result;
use crate::error::SecretsError;
use crate::kv::types::DeleteSecretRequest;
use crate::kv::types::DestroySecretRequest;
use crate::kv::types::KvConfig;
use crate::kv::types::ListSecretsRequest;
use crate::kv::types::ListSecretsResponse;
use crate::kv::types::ReadMetadataRequest;
use crate::kv::types::ReadSecretRequest;
use crate::kv::types::ReadSecretResponse;
use crate::kv::types::SecretData;
use crate::kv::types::SecretMetadata;
use crate::kv::types::UndeleteSecretRequest;
use crate::kv::types::UpdateMetadataRequest;
use crate::kv::types::WriteSecretRequest;
use crate::kv::types::WriteSecretResponse;

/// KV v2 secrets engine store.
///
/// Provides versioned key-value storage with:
/// - Multiple versions per secret (configurable max)
/// - Soft delete (can be undeleted)
/// - Hard delete / destroy (permanent)
/// - Check-and-set writes
/// - Secret metadata (custom key-values)
/// - Time-based version expiration
#[async_trait]
pub trait KvStore: Send + Sync {
    /// Read a secret at a specific path/version.
    async fn read(&self, request: ReadSecretRequest) -> Result<Option<ReadSecretResponse>>;

    /// Write a secret at a specific path.
    async fn write(&self, request: WriteSecretRequest) -> Result<WriteSecretResponse>;

    /// Soft-delete secret versions (can be undeleted).
    async fn delete(&self, request: DeleteSecretRequest) -> Result<()>;

    /// Permanently destroy secret versions (cannot be recovered).
    async fn destroy(&self, request: DestroySecretRequest) -> Result<()>;

    /// Undelete soft-deleted versions.
    async fn undelete(&self, request: UndeleteSecretRequest) -> Result<()>;

    /// Read secret metadata.
    async fn read_metadata(&self, request: ReadMetadataRequest) -> Result<Option<SecretMetadata>>;

    /// Update secret metadata.
    async fn update_metadata(&self, request: UpdateMetadataRequest) -> Result<SecretMetadata>;

    /// Delete secret and all its versions (metadata delete).
    async fn delete_metadata(&self, path: &str) -> Result<bool>;

    /// List secrets under a path prefix.
    async fn list(&self, request: ListSecretsRequest) -> Result<ListSecretsResponse>;

    /// Update engine configuration.
    async fn update_config(&self, config: KvConfig) -> Result<()>;

    /// Read the engine configuration (async version).
    ///
    /// Tiger Style: Prefer async method over sync method that would need to
    /// either panic or use unsafe sync locks for data behind an async lock.
    async fn read_config(&self) -> KvConfig;
}

/// Default KV v2 store implementation using SecretsBackend.
pub struct DefaultKvStore {
    /// Storage backend.
    backend: Arc<dyn SecretsBackend>,
    /// Engine configuration.
    config: tokio::sync::RwLock<KvConfig>,
}

impl DefaultKvStore {
    /// Create a new KV store with the given backend.
    pub fn new(backend: Arc<dyn SecretsBackend>) -> Self {
        Self {
            backend,
            config: tokio::sync::RwLock::new(KvConfig::default()),
        }
    }

    /// Create a new KV store with custom configuration.
    pub fn with_config(backend: Arc<dyn SecretsBackend>, config: KvConfig) -> Self {
        Self {
            backend,
            config: tokio::sync::RwLock::new(config),
        }
    }

    /// Get current timestamp in milliseconds.
    fn now_unix_ms() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    /// Validate a secret path.
    fn validate_path(path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(SecretsError::PathTooLong { length: 0, max: 1 });
        }
        if path.len() > MAX_SECRET_PATH_LENGTH {
            return Err(SecretsError::PathTooLong {
                length: path.len(),
                max: MAX_SECRET_PATH_LENGTH,
            });
        }
        Ok(())
    }

    /// Validate secret data.
    fn validate_data(data: &SecretData) -> Result<()> {
        if data.data.len() > MAX_KV_PAIRS_PER_SECRET {
            return Err(SecretsError::ValueTooLarge {
                size: data.data.len(),
                max: MAX_KV_PAIRS_PER_SECRET,
            });
        }

        for k in data.data.keys() {
            if k.len() > MAX_SECRET_KEY_NAME_LENGTH {
                return Err(SecretsError::PathTooLong {
                    length: k.len(),
                    max: MAX_SECRET_KEY_NAME_LENGTH,
                });
            }
        }

        let size = data.size_bytes();
        if size > MAX_KV_SECRET_SIZE {
            return Err(SecretsError::ValueTooLarge {
                size,
                max: MAX_KV_SECRET_SIZE,
            });
        }

        Ok(())
    }

    /// Get storage path for metadata.
    fn metadata_path(path: &str) -> String {
        format!("metadata/{}", path)
    }

    /// Get storage path for version data.
    fn data_path(path: &str, version: u64) -> String {
        format!("data/{}/{}", path, version)
    }

    /// Load metadata from storage.
    async fn load_metadata(&self, path: &str) -> Result<Option<SecretMetadata>> {
        let storage_path = Self::metadata_path(path);
        let data = self.backend.get(&storage_path).await?;

        match data {
            Some(bytes) => {
                let metadata: SecretMetadata = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted metadata: {e}"),
                })?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    /// Save metadata to storage.
    async fn save_metadata(&self, path: &str, metadata: &SecretMetadata) -> Result<()> {
        let storage_path = Self::metadata_path(path);
        let bytes =
            postcard::to_allocvec(metadata).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put(&storage_path, &bytes).await
    }

    /// Load version data from storage.
    async fn load_data(&self, path: &str, version: u64) -> Result<Option<SecretData>> {
        let storage_path = Self::data_path(path, version);
        let data = self.backend.get(&storage_path).await?;

        match data {
            Some(bytes) => {
                let secret: SecretData = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted secret data: {e}"),
                })?;
                Ok(Some(secret))
            }
            None => Ok(None),
        }
    }

    /// Save version data to storage.
    async fn save_data(&self, path: &str, version: u64, data: &SecretData) -> Result<()> {
        let storage_path = Self::data_path(path, version);
        let bytes = postcard::to_allocvec(data).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put(&storage_path, &bytes).await
    }

    /// Delete version data from storage.
    async fn delete_data(&self, path: &str, version: u64) -> Result<bool> {
        let storage_path = Self::data_path(path, version);
        self.backend.delete(&storage_path).await
    }

    /// Get the effective max_versions for a path.
    async fn effective_max_versions(&self, metadata: &SecretMetadata) -> u32 {
        if metadata.max_versions > 0 {
            metadata.max_versions
        } else {
            let config = self.config.read().await;
            config.max_versions
        }
    }

    /// Prune old versions to stay within limits.
    async fn prune_old_versions(&self, path: &str, metadata: &mut SecretMetadata) -> Result<()> {
        let max_versions = self.effective_max_versions(metadata).await;

        if max_versions > 0 && metadata.versions.len() as u32 > max_versions {
            let to_remove = metadata.prune_versions(max_versions);
            for v in to_remove {
                if let Err(e) = self.delete_data(path, v).await {
                    warn!(path = %path, version = %v, error = %e, "Failed to delete pruned version data");
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl KvStore for DefaultKvStore {
    async fn read(&self, request: ReadSecretRequest) -> Result<Option<ReadSecretResponse>> {
        Self::validate_path(&request.path)?;

        let metadata = match self.load_metadata(&request.path).await? {
            Some(m) => m,
            None => return Ok(None),
        };

        // Determine which version to read
        let version = request.version.unwrap_or(metadata.current_version);

        // Check version metadata
        let version_meta = match metadata.version(version) {
            Some(m) => m,
            None => {
                return Err(SecretsError::VersionNotFound {
                    path: request.path,
                    version,
                });
            }
        };

        // Check if destroyed
        if version_meta.destroyed {
            return Err(SecretsError::VersionDestroyed {
                path: request.path,
                version,
            });
        }

        // Check if soft-deleted
        if version_meta.deletion_time_unix_ms.is_some() {
            // Return None for soft-deleted versions (consistent with Vault behavior)
            return Ok(None);
        }

        // Load the data
        let data = match self.load_data(&request.path, version).await? {
            Some(d) => d,
            None => {
                return Err(SecretsError::Internal {
                    reason: format!("version {} metadata exists but data missing", version),
                });
            }
        };

        debug!(path = %request.path, version = %version, "Read secret");

        Ok(Some(ReadSecretResponse {
            data,
            metadata: version_meta.clone(),
        }))
    }

    async fn write(&self, request: WriteSecretRequest) -> Result<WriteSecretResponse> {
        Self::validate_path(&request.path)?;
        Self::validate_data(&request.data)?;

        let now = Self::now_unix_ms();
        let config = self.config.read().await;

        // Load or create metadata
        let mut metadata = self.load_metadata(&request.path).await?.unwrap_or_else(|| SecretMetadata::new(now));

        // Check CAS if required
        let cas_required = metadata.cas_required || config.cas_required;
        if cas_required && request.cas.is_none() {
            return Err(SecretsError::CasFailed {
                path: request.path,
                expected: 0,
                actual: metadata.current_version,
            });
        }

        if let Some(expected) = request.cas
            && expected != metadata.current_version
        {
            return Err(SecretsError::CasFailed {
                path: request.path,
                expected,
                actual: metadata.current_version,
            });
        }

        // Check version count limit
        if metadata.versions.len() as u32 >= MAX_VERSIONS_PER_SECRET {
            return Err(SecretsError::TooManyVersions {
                count: metadata.versions.len() as u32,
                max: MAX_VERSIONS_PER_SECRET,
            });
        }

        drop(config); // Release read lock

        // Create new version
        let version = metadata.next_version(now);

        // Save the data first
        self.save_data(&request.path, version, &request.data).await?;

        // Prune old versions
        self.prune_old_versions(&request.path, &mut metadata).await?;

        // Save metadata
        self.save_metadata(&request.path, &metadata).await?;

        debug!(path = %request.path, version = %version, "Wrote secret");

        let version_meta = metadata
            .versions
            .get(&version)
            .ok_or(SecretsError::Internal {
                reason: format!("version {} missing after creation", version),
            })?
            .clone();

        Ok(WriteSecretResponse {
            version,
            metadata: version_meta,
        })
    }

    async fn delete(&self, request: DeleteSecretRequest) -> Result<()> {
        Self::validate_path(&request.path)?;

        let mut metadata = match self.load_metadata(&request.path).await? {
            Some(m) => m,
            None => return Ok(()), // Nothing to delete
        };

        let now = Self::now_unix_ms();
        let versions = if request.versions.is_empty() {
            vec![metadata.current_version]
        } else {
            request.versions
        };

        for version in versions {
            if let Some(meta) = metadata.version_mut(version)
                && !meta.destroyed
                && meta.deletion_time_unix_ms.is_none()
            {
                meta.deletion_time_unix_ms = Some(now);
                debug!(path = %request.path, version = %version, "Soft-deleted secret version");
            }
        }

        metadata.updated_time_unix_ms = now;
        self.save_metadata(&request.path, &metadata).await
    }

    async fn destroy(&self, request: DestroySecretRequest) -> Result<()> {
        Self::validate_path(&request.path)?;

        if request.versions.is_empty() {
            return Ok(());
        }

        let mut metadata = match self.load_metadata(&request.path).await? {
            Some(m) => m,
            None => return Ok(()),
        };

        let now = Self::now_unix_ms();

        for version in &request.versions {
            // Delete the actual data
            let _ = self.delete_data(&request.path, *version).await;

            // Mark as destroyed in metadata
            if let Some(meta) = metadata.version_mut(*version) {
                meta.destroyed = true;
                debug!(path = %request.path, version = %version, "Destroyed secret version");
            }
        }

        metadata.updated_time_unix_ms = now;
        self.save_metadata(&request.path, &metadata).await
    }

    async fn undelete(&self, request: UndeleteSecretRequest) -> Result<()> {
        Self::validate_path(&request.path)?;

        if request.versions.is_empty() {
            return Ok(());
        }

        let mut metadata = match self.load_metadata(&request.path).await? {
            Some(m) => m,
            None => {
                return Err(SecretsError::SecretNotFound { key: request.path });
            }
        };

        let now = Self::now_unix_ms();

        for version in &request.versions {
            if let Some(meta) = metadata.version_mut(*version) {
                if meta.destroyed {
                    // Cannot undelete destroyed versions
                    return Err(SecretsError::VersionDestroyed {
                        path: request.path.clone(),
                        version: *version,
                    });
                }
                meta.deletion_time_unix_ms = None;
                debug!(path = %request.path, version = %version, "Undeleted secret version");
            }
        }

        metadata.updated_time_unix_ms = now;
        self.save_metadata(&request.path, &metadata).await
    }

    async fn read_metadata(&self, request: ReadMetadataRequest) -> Result<Option<SecretMetadata>> {
        Self::validate_path(&request.path)?;
        self.load_metadata(&request.path).await
    }

    async fn update_metadata(&self, request: UpdateMetadataRequest) -> Result<SecretMetadata> {
        Self::validate_path(&request.path)?;

        let mut metadata = match self.load_metadata(&request.path).await? {
            Some(m) => m,
            None => {
                return Err(SecretsError::SecretNotFound { key: request.path });
            }
        };

        let now = Self::now_unix_ms();

        if let Some(max_versions) = request.max_versions {
            metadata.max_versions = max_versions;
        }

        if let Some(cas_required) = request.cas_required {
            metadata.cas_required = cas_required;
        }

        if let Some(delete_after) = request.delete_version_after_secs {
            metadata.delete_version_after_secs = delete_after;
        }

        if let Some(custom) = request.custom_metadata {
            metadata.custom_metadata = custom;
        }

        metadata.updated_time_unix_ms = now;
        self.save_metadata(&request.path, &metadata).await?;

        Ok(metadata)
    }

    async fn delete_metadata(&self, path: &str) -> Result<bool> {
        Self::validate_path(path)?;

        let metadata = match self.load_metadata(path).await? {
            Some(m) => m,
            None => return Ok(false),
        };

        // Delete all version data
        // Tiger Style: Log deletion errors but continue to best-effort cleanup
        for version in metadata.versions.keys() {
            if let Err(e) = self.delete_data(path, *version).await {
                warn!(path = %path, version = %version, error = %e, "failed to delete secret version data during metadata delete");
            }
        }

        // Delete metadata
        let storage_path = Self::metadata_path(path);
        self.backend.delete(&storage_path).await?;

        debug!(path = %path, "Deleted secret and all versions");

        Ok(true)
    }

    async fn list(&self, request: ListSecretsRequest) -> Result<ListSecretsResponse> {
        // List keys under the metadata prefix
        let prefix = format!("metadata/{}", request.path);
        let keys = self.backend.list(&prefix).await?;

        Ok(ListSecretsResponse { keys })
    }

    async fn update_config(&self, config: KvConfig) -> Result<()> {
        let mut cfg = self.config.write().await;
        *cfg = config;
        Ok(())
    }

    async fn read_config(&self) -> KvConfig {
        self.config.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::backend::InMemorySecretsBackend;

    fn make_store() -> DefaultKvStore {
        let backend = Arc::new(InMemorySecretsBackend::new());
        DefaultKvStore::new(backend)
    }

    fn make_data(pairs: &[(&str, &str)]) -> SecretData {
        let mut data = HashMap::new();
        for (k, v) in pairs {
            data.insert(k.to_string(), v.to_string());
        }
        SecretData::new(data)
    }

    #[tokio::test]
    async fn test_write_and_read() {
        let store = make_store();

        let request = WriteSecretRequest::new("test/secret", make_data(&[("key", "value")]));
        let response = store.write(request).await.unwrap();
        assert_eq!(response.version, 1);

        let read_req = ReadSecretRequest::new("test/secret");
        let read_res = store.read(read_req).await.unwrap().unwrap();
        assert_eq!(read_res.data.get("key"), Some("value"));
        assert_eq!(read_res.metadata.version, 1);
    }

    #[tokio::test]
    async fn test_versioning() {
        let store = make_store();

        // Write version 1
        store.write(WriteSecretRequest::new("secret", make_data(&[("v", "1")]))).await.unwrap();

        // Write version 2
        store.write(WriteSecretRequest::new("secret", make_data(&[("v", "2")]))).await.unwrap();

        // Read current (v2)
        let current = store.read(ReadSecretRequest::new("secret")).await.unwrap().unwrap();
        assert_eq!(current.data.get("v"), Some("2"));
        assert_eq!(current.metadata.version, 2);

        // Read v1
        let v1 = store.read(ReadSecretRequest::with_version("secret", 1)).await.unwrap().unwrap();
        assert_eq!(v1.data.get("v"), Some("1"));
    }

    #[tokio::test]
    async fn test_cas() {
        let store = make_store();

        // First write without CAS
        store.write(WriteSecretRequest::new("secret", make_data(&[("v", "1")]))).await.unwrap();

        // CAS with correct version
        let res = store.write(WriteSecretRequest::with_cas("secret", make_data(&[("v", "2")]), 1)).await;
        assert!(res.is_ok());

        // CAS with wrong version
        let res = store.write(WriteSecretRequest::with_cas("secret", make_data(&[("v", "3")]), 1)).await;
        assert!(matches!(res.unwrap_err(), SecretsError::CasFailed { .. }));
    }

    #[tokio::test]
    async fn test_soft_delete_and_undelete() {
        let store = make_store();

        // Write
        store.write(WriteSecretRequest::new("secret", make_data(&[("k", "v")]))).await.unwrap();

        // Read succeeds
        assert!(store.read(ReadSecretRequest::new("secret")).await.unwrap().is_some());

        // Soft delete
        store.delete(DeleteSecretRequest::current("secret")).await.unwrap();

        // Read returns None (soft deleted)
        assert!(store.read(ReadSecretRequest::new("secret")).await.unwrap().is_none());

        // Undelete
        store.undelete(UndeleteSecretRequest::new("secret", vec![1])).await.unwrap();

        // Read succeeds again
        assert!(store.read(ReadSecretRequest::new("secret")).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_destroy() {
        let store = make_store();

        // Write two versions
        store.write(WriteSecretRequest::new("secret", make_data(&[("v", "1")]))).await.unwrap();
        store.write(WriteSecretRequest::new("secret", make_data(&[("v", "2")]))).await.unwrap();

        // Destroy v1
        store.destroy(DestroySecretRequest::new("secret", vec![1])).await.unwrap();

        // Reading v1 returns error
        let res = store.read(ReadSecretRequest::with_version("secret", 1)).await;
        assert!(matches!(res.unwrap_err(), SecretsError::VersionDestroyed { .. }));

        // v2 still readable
        assert!(store.read(ReadSecretRequest::with_version("secret", 2)).await.unwrap().is_some());

        // Attempting to undelete destroyed version fails
        let res = store.undelete(UndeleteSecretRequest::new("secret", vec![1])).await;
        assert!(matches!(res.unwrap_err(), SecretsError::VersionDestroyed { .. }));
    }

    #[tokio::test]
    async fn test_metadata() {
        let store = make_store();

        // Write a secret
        store.write(WriteSecretRequest::new("secret", make_data(&[("k", "v")]))).await.unwrap();

        // Read metadata
        let meta = store.read_metadata(ReadMetadataRequest::new("secret")).await.unwrap().unwrap();
        assert_eq!(meta.current_version, 1);

        // Update metadata
        let updated = store
            .update_metadata(UpdateMetadataRequest::new("secret").with_max_versions(5).with_cas_required(true))
            .await
            .unwrap();
        assert_eq!(updated.max_versions, 5);
        assert!(updated.cas_required);
    }

    #[tokio::test]
    async fn test_delete_metadata() {
        let store = make_store();

        // Write
        store.write(WriteSecretRequest::new("secret", make_data(&[("k", "v")]))).await.unwrap();

        // Delete all
        assert!(store.delete_metadata("secret").await.unwrap());

        // Secret no longer exists
        assert!(store.read(ReadSecretRequest::new("secret")).await.unwrap().is_none());
        assert!(store.read_metadata(ReadMetadataRequest::new("secret")).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list() {
        let store = make_store();

        // Write some secrets
        store.write(WriteSecretRequest::new("a/b/c", make_data(&[("k", "v")]))).await.unwrap();
        store.write(WriteSecretRequest::new("a/b/d", make_data(&[("k", "v")]))).await.unwrap();
        store.write(WriteSecretRequest::new("a/e", make_data(&[("k", "v")]))).await.unwrap();

        // List root
        let root = store.list(ListSecretsRequest::new("a/")).await.unwrap();
        assert!(root.keys.contains(&"b/".to_string()));
        assert!(root.keys.contains(&"e".to_string()));
    }
}
