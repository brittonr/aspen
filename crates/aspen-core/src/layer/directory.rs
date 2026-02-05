//! FoundationDB-style Directory Layer for hierarchical namespace management.
//!
//! The Directory Layer provides a hierarchical namespace on top of the flat
//! key-value store, mapping human-readable paths to short binary prefixes.
//!
//! # Benefits
//!
//! - **Short prefixes**: 1-8 byte prefixes instead of long strings
//! - **Guaranteed isolation**: Each directory has a unique prefix
//! - **Discoverability**: List all directories at any level
//! - **Efficient operations**: O(1) path-to-prefix lookup
//!
//! # Example
//!
//! ```ignore
//! use aspen_core::layer::{DirectoryLayer, Tuple};
//!
//! let dir = DirectoryLayer::new(kv_store.clone());
//!
//! // Create application directories
//! let forge = dir.create_or_open(&["apps", "forge"]).await?;
//! let repos = dir.create_or_open(&["apps", "forge", "repos"]).await?;
//!
//! // Use the directory's subspace for keys
//! let key = repos.pack(&Tuple::new().push(repo_id).push("metadata"));
//! kv_store.write(WriteRequest::set(key, value)).await?;
//!
//! // List all app directories
//! let apps = dir.list(&["apps"]).await?;
//! // => ["forge", "ci", ...]
//! ```
//!
//! # References
//!
//! - [FoundationDB Directory Layer](https://apple.github.io/foundationdb/developer-guide.html)
//! - [Directory Layer Specification](https://github.com/apple/foundationdb/blob/main/design/directory-layer.md)

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use snafu::ResultExt;
use snafu::Snafu;

use super::allocator::AllocationError;
use super::allocator::HighContentionAllocator;
use super::subspace::Subspace;
use super::subspace::SubspaceError;
use super::tuple::Tuple;
use crate::constants::directory::DEFAULT_LAYER_TYPE;
use crate::constants::directory::DIR_FIELD_CREATED_AT_MS;
use crate::constants::directory::DIR_FIELD_LAYER;
use crate::constants::directory::DIR_FIELD_PREFIX;
use crate::constants::directory::DIR_NODE_PREFIX;
use crate::constants::directory::DIRECTORY_PREFIX;
use crate::constants::directory::MAX_DIRECTORY_DEPTH;
use crate::constants::directory::MAX_LIST_RESULTS;
use crate::constants::directory::MAX_PATH_COMPONENT_LENGTH_BYTES;
use crate::error::KeyValueStoreError;
use crate::kv::BatchCondition;
use crate::kv::BatchOperation;
use crate::kv::ReadRequest;
use crate::kv::ScanRequest;
use crate::kv::WriteCommand;
use crate::kv::WriteRequest;
use crate::traits::KeyValueStore;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during directory operations.
#[derive(Debug, Snafu)]
pub enum DirectoryError {
    /// Directory not found at the specified path.
    #[snafu(display("directory not found: /{}", path.join("/")))]
    NotFound {
        /// The path that was not found.
        path: Vec<String>,
    },

    /// Directory already exists at the specified path.
    #[snafu(display("directory already exists: /{}", path.join("/")))]
    AlreadyExists {
        /// The path that already exists.
        path: Vec<String>,
    },

    /// Layer type mismatch when opening directory with specific layer.
    #[snafu(display(
        "layer mismatch for /{}: expected '{}', found '{}'",
        path.join("/"),
        expected,
        actual
    ))]
    LayerMismatch {
        /// The path with mismatched layer.
        path: Vec<String>,
        /// Expected layer type.
        expected: String,
        /// Actual layer type found.
        actual: String,
    },

    /// Path component is invalid (empty or too long).
    #[snafu(display("invalid path component: '{}' - {}", component, reason))]
    InvalidPath {
        /// The invalid component.
        component: String,
        /// Why it's invalid.
        reason: String,
    },

    /// Path exceeds maximum depth.
    #[snafu(display("path depth {} exceeds maximum of {}", depth, max))]
    PathTooDeep {
        /// Actual depth.
        depth: u32,
        /// Maximum allowed depth.
        max: u32,
    },

    /// Cannot move directory to a subdirectory of itself.
    #[snafu(display("cannot move directory to a subdirectory of itself"))]
    MoveCycle,

    /// Prefix allocation failed.
    #[snafu(display("prefix allocation failed: {}", source))]
    AllocationFailed {
        /// The underlying allocation error.
        source: AllocationError,
    },

    /// Directory metadata is corrupted.
    #[snafu(display("corrupted directory metadata for /{}: {}", path.join("/"), reason))]
    CorruptedMetadata {
        /// The path with corrupted metadata.
        path: Vec<String>,
        /// Description of the corruption.
        reason: String,
    },

    /// Storage error during directory operation.
    #[snafu(display("storage error: {}", source))]
    Storage {
        /// The underlying KV store error.
        source: KeyValueStoreError,
    },
}

// =============================================================================
// DirectorySubspace
// =============================================================================

/// A directory with an allocated prefix, extending Subspace with metadata.
///
/// `DirectorySubspace` provides all the functionality of a `Subspace` for key
/// encoding, plus metadata about the directory (path, layer type, creation time).
///
/// # Example
///
/// ```ignore
/// let repos = dir.create_or_open(&["apps", "forge", "repos"]).await?;
///
/// // Use subspace operations
/// let key = repos.pack(&Tuple::new().push(repo_id).push("name"));
/// let (start, end) = repos.range();
///
/// // Access metadata
/// println!("Path: {:?}", repos.path());
/// println!("Layer: {}", repos.layer_type());
/// ```
#[derive(Debug, Clone)]
pub struct DirectorySubspace {
    /// The underlying subspace with the allocated prefix.
    subspace: Subspace,
    /// Full path to this directory.
    path: Vec<String>,
    /// Layer type (e.g., "default", "partition").
    layer: String,
    /// Creation timestamp in milliseconds since Unix epoch.
    created_at_ms: u64,
}

impl DirectorySubspace {
    /// Create a new directory subspace.
    fn new(subspace: Subspace, path: Vec<String>, layer: String, created_at_ms: u64) -> Self {
        Self {
            subspace,
            path,
            layer,
            created_at_ms,
        }
    }

    /// Get the underlying subspace.
    pub fn subspace(&self) -> &Subspace {
        &self.subspace
    }

    /// Get the directory path.
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Get the layer type.
    pub fn layer_type(&self) -> &str {
        &self.layer
    }

    /// Get the creation timestamp in milliseconds since Unix epoch.
    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    /// Get the raw prefix bytes.
    pub fn prefix(&self) -> &[u8] {
        self.subspace.raw_prefix()
    }

    /// Pack a key tuple within this directory's subspace.
    ///
    /// The resulting bytes are the concatenation of this directory's prefix
    /// and the packed key tuple.
    pub fn pack(&self, key: &Tuple) -> Vec<u8> {
        self.subspace.pack(key)
    }

    /// Unpack a key from this directory's subspace.
    ///
    /// Returns the key tuple without this directory's prefix.
    pub fn unpack(&self, key: &[u8]) -> Result<Tuple, SubspaceError> {
        self.subspace.unpack(key)
    }

    /// Get the range of all keys in this directory.
    ///
    /// Returns `(start_key, end_key)` where start is inclusive and end is exclusive.
    pub fn range(&self) -> (Vec<u8>, Vec<u8>) {
        self.subspace.range()
    }

    /// Check if a key belongs to this directory.
    pub fn contains(&self, key: &[u8]) -> bool {
        self.subspace.contains(key)
    }

    /// Create a nested subspace within this directory.
    pub fn subspace_at(&self, suffix: &Tuple) -> Subspace {
        self.subspace.subspace(suffix)
    }
}

// =============================================================================
// DirectoryLayer
// =============================================================================

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
    store: Arc<KV>,
    /// Subspace for directory metadata.
    node_subspace: Subspace,
    /// High-contention allocator for prefix allocation.
    allocator: HighContentionAllocator<KV>,
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
                if dir.layer != layer {
                    return Err(DirectoryError::LayerMismatch {
                        path: path.iter().map(|s| s.to_string()).collect(),
                        expected: layer.to_string(),
                        actual: dir.layer,
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
                limit: Some(MAX_LIST_RESULTS),
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
                            value: old_dir.layer.clone(),
                        },
                        BatchOperation::Set {
                            key: new_prefix_key,
                            value: hex::encode(old_dir.prefix()),
                        },
                        BatchOperation::Set {
                            key: new_created_key,
                            value: old_dir.created_at_ms.to_string(),
                        },
                    ],
                },
            })
            .await
            .context(StorageSnafu)?;

        Ok(DirectorySubspace::new(
            Subspace::from_bytes(old_dir.prefix().to_vec()),
            new_path.iter().map(|s| s.to_string()).collect(),
            old_dir.layer,
            old_dir.created_at_ms,
        ))
    }

    // -------------------------------------------------------------------------
    // Internal Methods
    // -------------------------------------------------------------------------

    /// Open an existing directory (internal implementation).
    async fn open_internal(&self, path: &[&str]) -> Result<DirectorySubspace, DirectoryError> {
        let layer_key = self.metadata_key(path, DIR_FIELD_LAYER);
        let prefix_key = self.metadata_key(path, DIR_FIELD_PREFIX);
        let created_key = self.metadata_key(path, DIR_FIELD_CREATED_AT_MS);

        // Read layer type
        let layer = match self.store.read(ReadRequest::new(layer_key)).await {
            Ok(result) => result.kv.map(|kv| kv.value).unwrap_or_default(),
            Err(KeyValueStoreError::NotFound { .. }) => {
                return Err(DirectoryError::NotFound {
                    path: path.iter().map(|s| s.to_string()).collect(),
                });
            }
            Err(e) => return Err(DirectoryError::Storage { source: e }),
        };

        // Read prefix
        let prefix_hex = match self.store.read(ReadRequest::new(prefix_key)).await {
            Ok(result) => result.kv.map(|kv| kv.value).unwrap_or_default(),
            Err(e) => return Err(DirectoryError::Storage { source: e }),
        };

        let prefix = hex::decode(&prefix_hex).map_err(|_| DirectoryError::CorruptedMetadata {
            path: path.iter().map(|s| s.to_string()).collect(),
            reason: format!("invalid prefix hex: {prefix_hex}"),
        })?;

        // Read creation time
        let created_at_ms = match self.store.read(ReadRequest::new(created_key)).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                value_str.parse::<u64>().unwrap_or(0)
            }
            Err(KeyValueStoreError::NotFound { .. }) => 0,
            Err(e) => return Err(DirectoryError::Storage { source: e }),
        };

        Ok(DirectorySubspace::new(
            Subspace::from_bytes(prefix),
            path.iter().map(|s| s.to_string()).collect(),
            layer,
            created_at_ms,
        ))
    }

    /// Create a new directory (internal implementation).
    async fn create_internal(&self, path: &[&str], layer: &str) -> Result<DirectorySubspace, DirectoryError> {
        // Allocate a unique prefix
        let prefix_int = self.allocator.allocate().await.context(AllocationFailedSnafu)?;
        let prefix = self.allocator.encode_prefix(prefix_int);

        let created_at_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        // Build metadata keys
        let layer_key = self.metadata_key(path, DIR_FIELD_LAYER);
        let prefix_key = self.metadata_key(path, DIR_FIELD_PREFIX);
        let created_key = self.metadata_key(path, DIR_FIELD_CREATED_AT_MS);

        // Atomic create: conditional batch
        self.store
            .write(WriteRequest {
                command: WriteCommand::ConditionalBatch {
                    conditions: vec![BatchCondition::KeyNotExists { key: layer_key.clone() }],
                    operations: vec![
                        BatchOperation::Set {
                            key: layer_key,
                            value: layer.to_string(),
                        },
                        BatchOperation::Set {
                            key: prefix_key,
                            value: hex::encode(&prefix),
                        },
                        BatchOperation::Set {
                            key: created_key,
                            value: created_at_ms.to_string(),
                        },
                    ],
                },
            })
            .await
            .context(StorageSnafu)?;

        Ok(DirectorySubspace::new(
            Subspace::from_bytes(prefix),
            path.iter().map(|s| s.to_string()).collect(),
            layer.to_string(),
            created_at_ms,
        ))
    }

    /// Convert a path to a tuple for key construction.
    fn path_to_tuple(&self, path: &[&str]) -> Tuple {
        let mut tuple = Tuple::new();
        for component in path {
            tuple.push_mut(*component);
        }
        tuple
    }

    /// Generate a metadata key for a path and field.
    fn metadata_key(&self, path: &[&str], field: &str) -> String {
        let mut tuple = self.path_to_tuple(path);
        tuple.push_mut(field);
        String::from_utf8_lossy(&self.node_subspace.pack(&tuple)).to_string()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Validate a directory path.
fn validate_path(path: &[&str]) -> Result<(), DirectoryError> {
    // Path cannot be empty
    if path.is_empty() {
        return Err(DirectoryError::InvalidPath {
            component: String::new(),
            reason: "path cannot be empty".to_string(),
        });
    }

    validate_path_components(path)
}

/// Validate a directory path, allowing empty for list operations.
fn validate_path_allow_empty(path: &[&str]) -> Result<(), DirectoryError> {
    if path.is_empty() {
        return Ok(());
    }
    validate_path_components(path)
}

/// Validate path components.
fn validate_path_components(path: &[&str]) -> Result<(), DirectoryError> {
    // Check depth
    if path.len() as u32 > MAX_DIRECTORY_DEPTH {
        return Err(DirectoryError::PathTooDeep {
            depth: path.len() as u32,
            max: MAX_DIRECTORY_DEPTH,
        });
    }

    // Validate each component
    for component in path {
        if component.is_empty() {
            return Err(DirectoryError::InvalidPath {
                component: component.to_string(),
                reason: "path component cannot be empty".to_string(),
            });
        }

        if component.len() as u32 > MAX_PATH_COMPONENT_LENGTH_BYTES {
            return Err(DirectoryError::InvalidPath {
                component: component.to_string(),
                reason: format!(
                    "component length {} exceeds maximum {}",
                    component.len(),
                    MAX_PATH_COMPONENT_LENGTH_BYTES
                ),
            });
        }
    }

    Ok(())
}

/// Extract subdirectory name from a metadata key.
fn extract_subdirectory_name(key: &str, prefix: &str, parent_depth: usize) -> Option<String> {
    // Remove the prefix
    let suffix = key.strip_prefix(prefix)?;

    // The suffix should start with a tuple-encoded string
    // We need to find the first path component after the parent path
    // This is a simplified extraction - in production you'd use proper tuple parsing

    // For now, we'll scan for metadata field names and extract the component before them
    let fields = [DIR_FIELD_LAYER, DIR_FIELD_PREFIX, DIR_FIELD_CREATED_AT_MS];

    for field in fields {
        if suffix.contains(field) {
            // Find the component before this field
            // This is imprecise but works for ASCII paths
            let parts: Vec<&str> = suffix.split('\x00').collect();
            if parts.len() > parent_depth {
                // Extract the string content (skip type byte)
                let component = parts.first()?;
                // Remove type prefix (0x02 for string)
                if component.len() > 1 {
                    return Some(component[1..].to_string());
                }
            }
        }
    }

    None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inmemory::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_create_and_open() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create directory
        let forge = dir.create_or_open(&["apps", "forge"]).await.unwrap();
        assert_eq!(forge.path(), &["apps", "forge"]);
        assert_eq!(forge.layer_type(), DEFAULT_LAYER_TYPE);
        assert!(!forge.prefix().is_empty());

        // Open existing directory
        let forge2 = dir.open(&["apps", "forge"]).await.unwrap();
        assert_eq!(forge2.prefix(), forge.prefix());
    }

    #[tokio::test]
    async fn test_create_or_open_idempotent() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let d1 = dir.create_or_open(&["myapp"]).await.unwrap();
        let d2 = dir.create_or_open(&["myapp"]).await.unwrap();

        // Same prefix
        assert_eq!(d1.prefix(), d2.prefix());
    }

    #[tokio::test]
    async fn test_open_nonexistent_fails() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let result = dir.open(&["nonexistent"]).await;
        assert!(matches!(result, Err(DirectoryError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_exists() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        assert!(!dir.exists(&["myapp"]).await.unwrap());

        dir.create_or_open(&["myapp"]).await.unwrap();

        assert!(dir.exists(&["myapp"]).await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_directory() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["myapp"]).await.unwrap();
        assert!(dir.exists(&["myapp"]).await.unwrap());

        let removed = dir.remove(&["myapp"]).await.unwrap();
        assert!(removed);

        assert!(!dir.exists(&["myapp"]).await.unwrap());

        // Remove nonexistent returns false
        let removed2 = dir.remove(&["myapp"]).await.unwrap();
        assert!(!removed2);
    }

    #[tokio::test]
    async fn test_layer_mismatch() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open_with_layer(&["myapp"], "type1").await.unwrap();

        let result = dir.create_or_open_with_layer(&["myapp"], "type2").await;
        assert!(matches!(result, Err(DirectoryError::LayerMismatch { .. })));
    }

    #[tokio::test]
    async fn test_path_validation() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Empty path
        let result = dir.create_or_open(&[]).await;
        assert!(matches!(result, Err(DirectoryError::InvalidPath { .. })));

        // Empty component
        let result = dir.create_or_open(&["app", ""]).await;
        assert!(matches!(result, Err(DirectoryError::InvalidPath { .. })));
    }

    #[tokio::test]
    async fn test_move_directory() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let original = dir.create_or_open(&["old", "path"]).await.unwrap();
        let original_prefix = original.prefix().to_vec();

        let moved = dir.move_to(&["old", "path"], &["new", "path"]).await.unwrap();

        // Prefix should be preserved
        assert_eq!(moved.prefix(), &original_prefix);
        assert_eq!(moved.path(), &["new", "path"]);

        // Old path should not exist
        assert!(!dir.exists(&["old", "path"]).await.unwrap());

        // New path should exist
        assert!(dir.exists(&["new", "path"]).await.unwrap());
    }

    #[tokio::test]
    async fn test_move_prevents_cycle() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["parent", "child"]).await.unwrap();

        let result = dir.move_to(&["parent"], &["parent", "child", "grandchild"]).await;
        assert!(matches!(result, Err(DirectoryError::MoveCycle)));
    }

    #[tokio::test]
    async fn test_unique_prefixes() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let d1 = dir.create_or_open(&["app1"]).await.unwrap();
        let d2 = dir.create_or_open(&["app2"]).await.unwrap();
        let d3 = dir.create_or_open(&["app3"]).await.unwrap();

        // All prefixes should be unique
        assert_ne!(d1.prefix(), d2.prefix());
        assert_ne!(d2.prefix(), d3.prefix());
        assert_ne!(d1.prefix(), d3.prefix());
    }

    #[tokio::test]
    async fn test_pack_unpack() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["users"]).await.unwrap();

        // Pack a key
        let key = dir.pack(&Tuple::new().push("alice").push("profile"));
        assert!(key.starts_with(dir.prefix()));

        // Unpack the key
        let unpacked = dir.unpack(&key).unwrap();
        assert_eq!(unpacked.len(), 2);
    }

    #[tokio::test]
    async fn test_range() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["users"]).await.unwrap();
        let (start, end) = dir.range();

        // Start should be the prefix
        assert_eq!(start, dir.prefix());

        // End should be prefix + 0xFF
        assert!(end.len() == start.len() + 1);
        assert_eq!(&end[..start.len()], &start[..]);
        assert_eq!(end[start.len()], 0xFF);

        // Keys in directory should be in range
        let key = dir.pack(&Tuple::new().push("test"));
        assert!(key >= start && key < end);
    }
}
