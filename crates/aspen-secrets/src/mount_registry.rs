//! Multi-mount registry for secrets engines.
//!
//! Manages dynamic creation and caching of store instances per mount point.
//! Thread-safe for concurrent access using tokio RwLock with the double-check pattern.
//!
//! # Example
//!
//! ```rust,ignore
//! use aspen_secrets::MountRegistry;
//!
//! let registry = MountRegistry::new(kv_store);
//!
//! // Get or create PKI store for "pki-prod" mount (on-demand creation)
//! let store = registry.get_or_create_pki_store("pki-prod").await?;
//!
//! // Subsequent calls return the same store instance
//! let same_store = registry.get_or_create_pki_store("pki-prod").await?;
//! assert!(Arc::ptr_eq(&store, &same_store));
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::backend::AspenSecretsBackend;
use crate::constants::MAX_MOUNT_NAME_LENGTH;
use crate::constants::MAX_MOUNTS;
use crate::error::Result;
use crate::error::SecretsError;
use crate::kv::DefaultKvStore;
use crate::kv::KvStore;
use crate::pki::DefaultPkiStore;
use crate::pki::PkiStore;
use crate::transit::DefaultTransitStore;
use crate::transit::TransitStore;

/// Multi-mount registry for secrets engines.
///
/// Manages dynamic creation and caching of store instances per mount point.
/// Thread-safe for concurrent access across all engine types.
///
/// # Design
///
/// - Uses `tokio::sync::RwLock` for concurrent read access (most operations)
/// - Implements double-check pattern for thread-safe lazy initialization
/// - Each mount gets its own isolated storage via `AspenSecretsBackend` prefixing
/// - Mount count is bounded by `MAX_MOUNTS` (Tiger Style resource limit)
///
/// # Thread Safety
///
/// - Read operations: Parallel, O(1) HashMap lookup
/// - Mount creation: Brief exclusive lock, then parallel access
/// - No lock held across I/O operations
pub struct MountRegistry {
    /// PKI stores indexed by mount name.
    pki_stores: Arc<RwLock<HashMap<String, Arc<dyn PkiStore>>>>,

    /// Transit stores indexed by mount name.
    transit_stores: Arc<RwLock<HashMap<String, Arc<dyn TransitStore>>>>,

    /// KV stores indexed by mount name.
    kv_stores: Arc<RwLock<HashMap<String, Arc<dyn KvStore>>>>,

    /// Underlying KV store for all backends.
    kv: Arc<dyn aspen_core::KeyValueStore>,
}

impl MountRegistry {
    /// Create a new mount registry with the given KV store backend.
    pub fn new(kv: Arc<dyn aspen_core::KeyValueStore>) -> Self {
        Self {
            pki_stores: Arc::new(RwLock::new(HashMap::new())),
            transit_stores: Arc::new(RwLock::new(HashMap::new())),
            kv_stores: Arc::new(RwLock::new(HashMap::new())),
            kv,
        }
    }

    /// Get or create a PKI store for the given mount point.
    ///
    /// Creates the store on-demand if it doesn't exist. Subsequent calls
    /// return the same store instance.
    ///
    /// # Errors
    ///
    /// - `SecretsError::InvalidMount` if the mount name is invalid
    /// - `SecretsError::TooManyMounts` if the mount limit is exceeded
    pub async fn get_or_create_pki_store(&self, mount: &str) -> Result<Arc<dyn PkiStore>> {
        Self::validate_mount_name(mount)?;

        // Fast path: check read lock first
        {
            let stores = self.pki_stores.read().await;
            if let Some(store) = stores.get(mount) {
                return Ok(Arc::clone(store));
            }
        }

        // Slow path: acquire write lock for creation
        let mut stores = self.pki_stores.write().await;

        // Double-check: may have been created by another task
        if let Some(store) = stores.get(mount) {
            return Ok(Arc::clone(store));
        }

        // Check mount limit
        if stores.len() >= MAX_MOUNTS as usize {
            return Err(SecretsError::TooManyMounts {
                count: stores.len() as u32,
                max: MAX_MOUNTS,
            });
        }

        // Create new store with mount-specific backend
        let backend = Arc::new(AspenSecretsBackend::new(self.kv.clone(), mount));
        let store: Arc<dyn PkiStore> = Arc::new(DefaultPkiStore::with_mount(backend, mount));
        stores.insert(mount.to_string(), Arc::clone(&store));

        tracing::debug!(mount = %mount, "created PKI store for mount");

        Ok(store)
    }

    /// Get or create a Transit store for the given mount point.
    ///
    /// Creates the store on-demand if it doesn't exist. Subsequent calls
    /// return the same store instance.
    ///
    /// # Errors
    ///
    /// - `SecretsError::InvalidMount` if the mount name is invalid
    /// - `SecretsError::TooManyMounts` if the mount limit is exceeded
    pub async fn get_or_create_transit_store(&self, mount: &str) -> Result<Arc<dyn TransitStore>> {
        Self::validate_mount_name(mount)?;

        // Fast path: check read lock first
        {
            let stores = self.transit_stores.read().await;
            if let Some(store) = stores.get(mount) {
                return Ok(Arc::clone(store));
            }
        }

        // Slow path: acquire write lock for creation
        let mut stores = self.transit_stores.write().await;

        // Double-check: may have been created by another task
        if let Some(store) = stores.get(mount) {
            return Ok(Arc::clone(store));
        }

        // Check mount limit
        if stores.len() >= MAX_MOUNTS as usize {
            return Err(SecretsError::TooManyMounts {
                count: stores.len() as u32,
                max: MAX_MOUNTS,
            });
        }

        // Create new store with mount-specific backend
        let backend = Arc::new(AspenSecretsBackend::new(self.kv.clone(), mount));
        let store: Arc<dyn TransitStore> = Arc::new(DefaultTransitStore::new(backend));
        stores.insert(mount.to_string(), Arc::clone(&store));

        tracing::debug!(mount = %mount, "created Transit store for mount");

        Ok(store)
    }

    /// Get or create a KV store for the given mount point.
    ///
    /// Creates the store on-demand if it doesn't exist. Subsequent calls
    /// return the same store instance.
    ///
    /// # Errors
    ///
    /// - `SecretsError::InvalidMount` if the mount name is invalid
    /// - `SecretsError::TooManyMounts` if the mount limit is exceeded
    pub async fn get_or_create_kv_store(&self, mount: &str) -> Result<Arc<dyn KvStore>> {
        Self::validate_mount_name(mount)?;

        // Fast path: check read lock first
        {
            let stores = self.kv_stores.read().await;
            if let Some(store) = stores.get(mount) {
                return Ok(Arc::clone(store));
            }
        }

        // Slow path: acquire write lock for creation
        let mut stores = self.kv_stores.write().await;

        // Double-check: may have been created by another task
        if let Some(store) = stores.get(mount) {
            return Ok(Arc::clone(store));
        }

        // Check mount limit
        if stores.len() >= MAX_MOUNTS as usize {
            return Err(SecretsError::TooManyMounts {
                count: stores.len() as u32,
                max: MAX_MOUNTS,
            });
        }

        // Create new store with mount-specific backend
        let backend = Arc::new(AspenSecretsBackend::new(self.kv.clone(), mount));
        let store: Arc<dyn KvStore> = Arc::new(DefaultKvStore::new(backend));
        stores.insert(mount.to_string(), Arc::clone(&store));

        tracing::debug!(mount = %mount, "created KV store for mount");

        Ok(store)
    }

    /// Validate a mount name against Tiger Style limits.
    ///
    /// # Rules
    ///
    /// - Must be non-empty
    /// - Must be <= MAX_MOUNT_NAME_LENGTH characters
    /// - Must only contain alphanumeric characters, hyphens, or underscores
    fn validate_mount_name(mount: &str) -> Result<()> {
        if mount.is_empty() {
            return Err(SecretsError::InvalidMount {
                name: mount.to_string(),
                reason: "empty mount name".to_string(),
            });
        }

        if mount.len() > MAX_MOUNT_NAME_LENGTH {
            return Err(SecretsError::InvalidMount {
                name: mount.to_string(),
                reason: format!("mount name too long: {} > {}", mount.len(), MAX_MOUNT_NAME_LENGTH),
            });
        }

        if !mount.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(SecretsError::InvalidMount {
                name: mount.to_string(),
                reason: "must only contain alphanumeric characters, hyphens, or underscores".to_string(),
            });
        }

        Ok(())
    }

    /// Get the count of PKI mounts currently registered.
    pub async fn pki_mount_count(&self) -> usize {
        self.pki_stores.read().await.len()
    }

    /// Get the count of Transit mounts currently registered.
    pub async fn transit_mount_count(&self) -> usize {
        self.transit_stores.read().await.len()
    }

    /// Get the count of KV mounts currently registered.
    pub async fn kv_mount_count(&self) -> usize {
        self.kv_stores.read().await.len()
    }

    /// List all PKI mount names.
    pub async fn list_pki_mounts(&self) -> Vec<String> {
        self.pki_stores.read().await.keys().cloned().collect()
    }

    /// List all Transit mount names.
    pub async fn list_transit_mounts(&self) -> Vec<String> {
        self.transit_stores.read().await.keys().cloned().collect()
    }

    /// List all KV mount names.
    pub async fn list_kv_mounts(&self) -> Vec<String> {
        self.kv_stores.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InMemorySecretsBackend;

    /// In-memory KV store for testing.
    struct InMemoryKvStore {
        data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    }

    impl InMemoryKvStore {
        fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl aspen_core::KeyValueStore for InMemoryKvStore {
        async fn read(&self, key: &str) -> std::result::Result<Option<Vec<u8>>, aspen_core::KeyValueStoreError> {
            Ok(self.data.blocking_read().get(key).cloned())
        }

        async fn write(
            &self,
            request: aspen_core::WriteRequest,
        ) -> std::result::Result<(), aspen_core::KeyValueStoreError> {
            let mut data = self.data.blocking_write();
            match request {
                aspen_core::WriteRequest::Set { key, value } => {
                    data.insert(key, value);
                }
                aspen_core::WriteRequest::SetMulti { entries } => {
                    for (key, value) in entries {
                        data.insert(key, value);
                    }
                }
                aspen_core::WriteRequest::Delete { key } => {
                    data.remove(&key);
                }
                aspen_core::WriteRequest::DeleteMulti { keys } => {
                    for key in keys {
                        data.remove(&key);
                    }
                }
            }
            Ok(())
        }

        async fn delete(&self, key: &str) -> std::result::Result<(), aspen_core::KeyValueStoreError> {
            self.data.blocking_write().remove(key);
            Ok(())
        }

        async fn scan(
            &self,
            prefix: &str,
            limit: Option<usize>,
        ) -> std::result::Result<Vec<(String, Vec<u8>)>, aspen_core::KeyValueStoreError> {
            let data = self.data.blocking_read();
            let limit = limit.unwrap_or(usize::MAX);
            Ok(data
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .take(limit)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
        }
    }

    #[tokio::test]
    async fn test_pki_store_on_demand_creation() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        let store1 = registry.get_or_create_pki_store("pki").await.unwrap();
        let store2 = registry.get_or_create_pki_store("pki").await.unwrap();

        // Same store instance (Arc pointers equal)
        assert!(Arc::ptr_eq(&store1, &store2));

        // Mount count should be 1
        assert_eq!(registry.pki_mount_count().await, 1);
    }

    #[tokio::test]
    async fn test_different_mounts_different_stores() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        let store1 = registry.get_or_create_pki_store("pki").await.unwrap();
        let store2 = registry.get_or_create_pki_store("pki-prod").await.unwrap();

        // Different mount -> different store
        assert!(!Arc::ptr_eq(&store1, &store2));

        // Mount count should be 2
        assert_eq!(registry.pki_mount_count().await, 2);
    }

    #[tokio::test]
    async fn test_mount_name_validation_empty() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        let result = registry.get_or_create_pki_store("").await;
        assert!(matches!(result, Err(SecretsError::InvalidMount { .. })));
    }

    #[tokio::test]
    async fn test_mount_name_validation_too_long() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        let long_name = "x".repeat(MAX_MOUNT_NAME_LENGTH + 1);
        let result = registry.get_or_create_pki_store(&long_name).await;
        assert!(matches!(result, Err(SecretsError::InvalidMount { .. })));
    }

    #[tokio::test]
    async fn test_mount_name_validation_invalid_chars() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        let result = registry.get_or_create_pki_store("pki@prod").await;
        assert!(matches!(result, Err(SecretsError::InvalidMount { .. })));

        let result = registry.get_or_create_pki_store("pki/prod").await;
        assert!(matches!(result, Err(SecretsError::InvalidMount { .. })));

        let result = registry.get_or_create_pki_store("pki prod").await;
        assert!(matches!(result, Err(SecretsError::InvalidMount { .. })));
    }

    #[tokio::test]
    async fn test_mount_name_validation_valid() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        // All these should succeed
        assert!(registry.get_or_create_pki_store("pki").await.is_ok());
        assert!(registry.get_or_create_pki_store("pki-prod").await.is_ok());
        assert!(registry.get_or_create_pki_store("pki_staging").await.is_ok());
        assert!(registry.get_or_create_pki_store("PKI123").await.is_ok());
    }

    #[tokio::test]
    async fn test_list_mounts() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        registry.get_or_create_pki_store("pki").await.unwrap();
        registry.get_or_create_pki_store("pki-prod").await.unwrap();

        let mounts = registry.list_pki_mounts().await;
        assert_eq!(mounts.len(), 2);
        assert!(mounts.contains(&"pki".to_string()));
        assert!(mounts.contains(&"pki-prod".to_string()));
    }

    #[tokio::test]
    async fn test_all_engine_types() {
        let registry = MountRegistry::new(Arc::new(InMemoryKvStore::new()));

        // Create stores for each engine type
        registry.get_or_create_pki_store("pki").await.unwrap();
        registry.get_or_create_transit_store("transit").await.unwrap();
        registry.get_or_create_kv_store("secret").await.unwrap();

        assert_eq!(registry.pki_mount_count().await, 1);
        assert_eq!(registry.transit_mount_count().await, 1);
        assert_eq!(registry.kv_mount_count().await, 1);
    }
}
