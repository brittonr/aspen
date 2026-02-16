//! DirectoryLayer - public API for hierarchical namespace management.

use std::sync::Arc;

use snafu::ResultExt;

use super::super::HighContentionAllocator;
use super::super::Subspace;
use super::super::Tuple;
use super::DirectoryError;
use super::StorageSnafu;
use super::subspace::DirectorySubspace;
use super::validation::extract_subdirectory_name;
use super::validation::validate_path;
use super::validation::validate_path_allow_empty;
use crate::constants::directory::DEFAULT_LAYER_TYPE;
use crate::constants::directory::DIR_FIELD_CREATED_AT_MS;
use crate::constants::directory::DIR_FIELD_LAYER;
use crate::constants::directory::DIR_FIELD_PREFIX;
use crate::constants::directory::DIR_NODE_PREFIX;
use crate::constants::directory::DIRECTORY_PREFIX;
use crate::constants::directory::MAX_LIST_RESULTS;
use crate::error::KeyValueStoreError;
use crate::kv::BatchCondition;
use crate::kv::BatchOperation;
use crate::kv::ReadRequest;
use crate::kv::ScanRequest;
use crate::kv::WriteCommand;
use crate::kv::WriteRequest;
use crate::traits::KeyValueStore;

/// Directory Layer for hierarchical namespace management.
///
/// The Directory Layer maps hierarchical paths to short binary prefixes,
/// enabling efficient multi-tenant namespace isolation.
///
/// # Thread Safety
///
/// `DirectoryLayer` is `Send + Sync` and can be shared across threads.
/// All operations are async and use the underlying `KeyValueStore`.
pub struct DirectoryLayer<KV: KeyValueStore + ?Sized> {
    /// Underlying KV store.
    pub(super) store: Arc<KV>,
    /// Subspace for directory metadata.
    pub(super) node_subspace: Subspace,
    /// High-contention allocator for prefix allocation.
    pub(super) allocator: HighContentionAllocator<KV>,
}

impl<KV: KeyValueStore + ?Sized> DirectoryLayer<KV> {
    /// Create a new Directory Layer.
    ///
    /// The directory layer stores its metadata under the reserved prefix 0xFE.
    pub fn new(store: Arc<KV>) -> Self {
        // Create node subspace under (0xFE, "node")
        let node_subspace = Subspace::new(Tuple::new().push(vec![DIRECTORY_PREFIX]).push(DIR_NODE_PREFIX));

        let allocator = HighContentionAllocator::new(store.clone());

        Self {
            store,
            node_subspace,
            allocator,
        }
    }

    /// Create or open a directory at the given path.
    ///
    /// If the directory exists, returns it. Otherwise creates it with
    /// the default layer type ("default").
    ///
    /// # Arguments
    ///
    /// * `path` - Path components (e.g., `&["apps", "forge"]`)
    ///
    /// # Errors
    ///
    /// Returns an error if path validation fails or a storage error occurs.
    pub async fn create_or_open(&self, path: &[&str]) -> Result<DirectorySubspace, DirectoryError> {
        self.create_or_open_with_layer(path, DEFAULT_LAYER_TYPE).await
    }

    /// Create or open a directory with a specific layer type.
    ///
    /// # Arguments
    ///
    /// * `path` - Path components
    /// * `layer` - Layer type for the directory
    ///
    /// # Errors
    ///
    /// Returns `DirectoryError::LayerMismatch` if the directory exists with
    /// a different layer type.
    pub async fn create_or_open_with_layer(
        &self,
        path: &[&str],
        layer: &str,
    ) -> Result<DirectorySubspace, DirectoryError> {
        // Validate path
        validate_path(path)?;

        // Try to open existing directory first
        match self.open_internal(path).await {
            Ok(dir) => {
                // Check layer type matches
                if dir.layer_ref() != layer {
                    return Err(DirectoryError::LayerMismatch {
                        path: path.iter().map(|s| s.to_string()).collect(),
                        expected: layer.to_string(),
                        actual: dir.layer_ref().to_string(),
                    });
                }
                Ok(dir)
            }
            Err(DirectoryError::NotFound { .. }) => {
                // Create new directory
                self.create_internal(path, layer).await
            }
            Err(e) => Err(e),
        }
    }

    /// Open an existing directory.
    ///
    /// # Errors
    ///
    /// Returns `DirectoryError::NotFound` if the directory doesn't exist.
    pub async fn open(&self, path: &[&str]) -> Result<DirectorySubspace, DirectoryError> {
        validate_path(path)?;
        self.open_internal(path).await
    }

    /// Check if a directory exists at the given path.
    pub async fn exists(&self, path: &[&str]) -> Result<bool, DirectoryError> {
        validate_path(path)?;

        let layer_key = self.metadata_key(path, DIR_FIELD_LAYER);
        match self.store.read(ReadRequest::new(layer_key)).await {
            Ok(_) => Ok(true),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(DirectoryError::Storage { source: e }),
        }
    }

    /// List all subdirectories at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Parent path (empty slice for root)
    ///
    /// # Returns
    ///
    /// Names of direct subdirectories.
    pub async fn list(&self, path: &[&str]) -> Result<Vec<String>, DirectoryError> {
        validate_path_allow_empty(path)?;

        // Build prefix for scanning subdirectories
        let prefix_tuple = self.path_to_tuple(path);
        let prefix = self.node_subspace.pack(&prefix_tuple);
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();

        // Scan for subdirectories
        let result = self
            .store
            .scan(ScanRequest {
                prefix: prefix_str.clone(),
                limit_results: Some(MAX_LIST_RESULTS),
                continuation_token: None,
            })
            .await
            .context(StorageSnafu)?;

        // Extract unique subdirectory names
        let mut subdirs = std::collections::BTreeSet::new();
        let path_depth = path.len();

        for entry in result.entries {
            // Parse the key to extract subdirectory name
            if let Some(subdir) = extract_subdirectory_name(&entry.key, &prefix_str, path_depth) {
                subdirs.insert(subdir);
            }
        }

        Ok(subdirs.into_iter().collect())
    }

    /// Remove a directory.
    ///
    /// This removes the directory metadata. It does NOT delete data stored
    /// under the directory's prefix - that must be done separately.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Directory was removed
    /// * `Ok(false)` - Directory did not exist
    pub async fn remove(&self, path: &[&str]) -> Result<bool, DirectoryError> {
        validate_path(path)?;

        let layer_key = self.metadata_key(path, DIR_FIELD_LAYER);
        let prefix_key = self.metadata_key(path, DIR_FIELD_PREFIX);
        let created_key = self.metadata_key(path, DIR_FIELD_CREATED_AT_MS);

        // Check if directory exists
        match self.store.read(ReadRequest::new(layer_key.clone())).await {
            Ok(_) => {
                // Delete all metadata keys
                self.store
                    .write(WriteRequest {
                        command: WriteCommand::DeleteMulti {
                            keys: vec![layer_key, prefix_key, created_key],
                        },
                    })
                    .await
                    .context(StorageSnafu)?;
                Ok(true)
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(DirectoryError::Storage { source: e }),
        }
    }

    /// Move a directory to a new path.
    ///
    /// This is an efficient operation that only updates metadata;
    /// the actual prefix and data remain unchanged.
    ///
    /// # Errors
    ///
    /// * `DirectoryError::NotFound` - Source doesn't exist
    /// * `DirectoryError::AlreadyExists` - Destination exists
    /// * `DirectoryError::MoveCycle` - Would create a cycle
    pub async fn move_to(&self, old_path: &[&str], new_path: &[&str]) -> Result<DirectorySubspace, DirectoryError> {
        validate_path(old_path)?;
        validate_path(new_path)?;

        // Check for cycle (new_path is subdirectory of old_path)
        if new_path.len() > old_path.len() && new_path.starts_with(old_path) {
            return Err(DirectoryError::MoveCycle);
        }

        // Read old directory metadata
        let old_dir = self.open_internal(old_path).await?;

        // Check destination doesn't exist
        if self.exists(new_path).await? {
            return Err(DirectoryError::AlreadyExists {
                path: new_path.iter().map(|s| s.to_string()).collect(),
            });
        }

        // Create new metadata entries
        let new_layer_key = self.metadata_key(new_path, DIR_FIELD_LAYER);
        let new_prefix_key = self.metadata_key(new_path, DIR_FIELD_PREFIX);
        let new_created_key = self.metadata_key(new_path, DIR_FIELD_CREATED_AT_MS);

        let old_layer_key = self.metadata_key(old_path, DIR_FIELD_LAYER);
        let old_prefix_key = self.metadata_key(old_path, DIR_FIELD_PREFIX);
        let old_created_key = self.metadata_key(old_path, DIR_FIELD_CREATED_AT_MS);

        // Atomic move: conditional batch
        self.store
            .write(WriteRequest {
                command: WriteCommand::ConditionalBatch {
                    conditions: vec![
                        BatchCondition::KeyExists {
                            key: old_layer_key.clone(),
                        },
                        BatchCondition::KeyNotExists {
                            key: new_layer_key.clone(),
                        },
                    ],
                    operations: vec![
                        // Delete old
                        BatchOperation::Delete { key: old_layer_key },
                        BatchOperation::Delete { key: old_prefix_key },
                        BatchOperation::Delete { key: old_created_key },
                        // Create new (preserving prefix)
                        BatchOperation::Set {
                            key: new_layer_key,
                            value: old_dir.layer_ref().to_string(),
                        },
                        BatchOperation::Set {
                            key: new_prefix_key,
                            value: hex::encode(old_dir.prefix()),
                        },
                        BatchOperation::Set {
                            key: new_created_key,
                            value: old_dir.created_at_ms().to_string(),
                        },
                    ],
                },
            })
            .await
            .context(StorageSnafu)?;

        Ok(DirectorySubspace::new(
            Subspace::from_bytes(old_dir.prefix().to_vec()),
            new_path.iter().map(|s| s.to_string()).collect(),
            old_dir.layer_ref().to_string(),
            old_dir.created_at_ms(),
        ))
    }
}
