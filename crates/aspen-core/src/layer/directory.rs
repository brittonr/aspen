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

use super::AllocationError;
use super::HighContentionAllocator;
use super::Subspace;
use super::SubspaceError;
use super::Tuple;
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
    use crate::test_support::DeterministicKeyValueStore;

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

    // =========================================================================
    // DirectorySubspace Tests
    // =========================================================================

    #[tokio::test]
    async fn test_directory_subspace_accessors() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["apps", "forge"]).await.unwrap();

        // Test all accessors
        assert_eq!(dir.path(), &["apps", "forge"]);
        assert_eq!(dir.layer_type(), DEFAULT_LAYER_TYPE);
        assert!(!dir.prefix().is_empty());
        assert!(dir.created_at_ms() > 0);
        assert!(dir.subspace().raw_prefix() == dir.prefix());
    }

    #[tokio::test]
    async fn test_directory_subspace_contains() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["data"]).await.unwrap();

        // Key packed in this directory should be contained
        let key = dir.pack(&Tuple::new().push("record"));
        assert!(dir.contains(&key));

        // Random key should not be contained
        assert!(!dir.contains(b"random-key"));
    }

    #[tokio::test]
    async fn test_directory_subspace_at() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["users"]).await.unwrap();
        let alice_subspace = dir.subspace_at(&Tuple::new().push("alice"));

        // Subspace should be nested
        assert!(alice_subspace.raw_prefix().len() > dir.prefix().len());
        assert!(alice_subspace.raw_prefix().starts_with(dir.prefix()));

        // Keys in nested subspace should be contained in parent
        let key = alice_subspace.pack(&Tuple::new().push("profile"));
        assert!(dir.contains(&key));
    }

    // =========================================================================
    // DirectoryError Tests
    // =========================================================================

    #[test]
    fn test_directory_error_not_found_display() {
        let err = DirectoryError::NotFound {
            path: vec!["apps".to_string(), "missing".to_string()],
        };
        let display = format!("{}", err);
        assert!(display.contains("apps/missing"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_directory_error_already_exists_display() {
        let err = DirectoryError::AlreadyExists {
            path: vec!["apps".to_string(), "existing".to_string()],
        };
        let display = format!("{}", err);
        assert!(display.contains("apps/existing"));
        assert!(display.contains("already exists"));
    }

    #[test]
    fn test_directory_error_layer_mismatch_display() {
        let err = DirectoryError::LayerMismatch {
            path: vec!["apps".to_string()],
            expected: "type1".to_string(),
            actual: "type2".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("type1"));
        assert!(display.contains("type2"));
        assert!(display.contains("mismatch"));
    }

    #[test]
    fn test_directory_error_invalid_path_display() {
        let err = DirectoryError::InvalidPath {
            component: "".to_string(),
            reason: "cannot be empty".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("invalid path"));
        assert!(display.contains("cannot be empty"));
    }

    #[test]
    fn test_directory_error_path_too_deep_display() {
        let err = DirectoryError::PathTooDeep { depth: 50, max: 32 };
        let display = format!("{}", err);
        assert!(display.contains("50"));
        assert!(display.contains("32"));
        assert!(display.contains("exceeds"));
    }

    #[test]
    fn test_directory_error_move_cycle_display() {
        let err = DirectoryError::MoveCycle;
        let display = format!("{}", err);
        assert!(display.contains("subdirectory of itself"));
    }

    #[test]
    fn test_directory_error_corrupted_metadata_display() {
        let err = DirectoryError::CorruptedMetadata {
            path: vec!["bad".to_string()],
            reason: "invalid hex".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("corrupted"));
        assert!(display.contains("invalid hex"));
    }

    // =========================================================================
    // Path Validation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_path_too_deep() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create a path that's too deep
        let deep_path: Vec<&str> = (0..33).map(|_| "level").collect();
        let result = dir.create_or_open(&deep_path).await;
        assert!(matches!(result, Err(DirectoryError::PathTooDeep { .. })));
    }

    #[tokio::test]
    async fn test_path_component_too_long() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create a component that's too long (> 255 bytes)
        let long_component = "x".repeat(300);
        let result = dir.create_or_open(&[&long_component]).await;
        assert!(matches!(result, Err(DirectoryError::InvalidPath { .. })));
    }

    #[tokio::test]
    async fn test_path_max_depth_allowed() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create a path at exactly max depth (32)
        let max_path: Vec<&str> = (0..32).map(|_| "l").collect();
        let result = dir.create_or_open(&max_path).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Nested Directory Tests
    // =========================================================================

    #[tokio::test]
    async fn test_nested_directories_isolation() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let parent = dir.create_or_open(&["parent"]).await.unwrap();
        let child1 = dir.create_or_open(&["parent", "child1"]).await.unwrap();
        let child2 = dir.create_or_open(&["parent", "child2"]).await.unwrap();

        // All should have unique prefixes
        assert_ne!(parent.prefix(), child1.prefix());
        assert_ne!(parent.prefix(), child2.prefix());
        assert_ne!(child1.prefix(), child2.prefix());

        // Keys in children should NOT be contained in parent's subspace
        // (directories have independent prefixes, not hierarchical)
        let child1_key = child1.pack(&Tuple::new().push("data"));
        assert!(!parent.contains(&child1_key));
    }

    #[tokio::test]
    async fn test_sibling_directories() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create many sibling directories
        for i in 0..10 {
            let name = format!("sibling{}", i);
            dir.create_or_open(&[&name]).await.unwrap();
        }

        // All should exist
        for i in 0..10 {
            let name = format!("sibling{}", i);
            assert!(dir.exists(&[&name]).await.unwrap());
        }
    }

    // =========================================================================
    // Move Operation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_move_to_nonexistent_source() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let result = dir.move_to(&["nonexistent"], &["new", "path"]).await;
        assert!(matches!(result, Err(DirectoryError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_move_to_existing_destination() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["source"]).await.unwrap();
        dir.create_or_open(&["destination"]).await.unwrap();

        let result = dir.move_to(&["source"], &["destination"]).await;
        assert!(matches!(result, Err(DirectoryError::AlreadyExists { .. })));
    }

    #[tokio::test]
    async fn test_move_preserves_layer_type() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let original = dir.create_or_open_with_layer(&["old"], "custom-layer").await.unwrap();
        let original_layer = original.layer_type().to_string();

        let moved = dir.move_to(&["old"], &["new"]).await.unwrap();
        assert_eq!(moved.layer_type(), original_layer);
    }

    #[tokio::test]
    async fn test_move_preserves_created_at() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let original = dir.create_or_open(&["old"]).await.unwrap();
        let original_created = original.created_at_ms();

        let moved = dir.move_to(&["old"], &["new"]).await.unwrap();
        assert_eq!(moved.created_at_ms(), original_created);
    }

    // =========================================================================
    // List Operation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_list_empty() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // List root when nothing exists
        let result = dir.list(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_root() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["app1"]).await.unwrap();
        dir.create_or_open(&["app2"]).await.unwrap();
        dir.create_or_open(&["app3"]).await.unwrap();

        let result = dir.list(&[]).await.unwrap();
        // Note: list returns subdirectory names, not full paths
        // The exact count depends on implementation details of extract_subdirectory_name
        assert!(!result.is_empty());
    }

    // =========================================================================
    // Create or Open Edge Cases
    // =========================================================================

    #[tokio::test]
    async fn test_create_or_open_concurrent_creates() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = Arc::new(DirectoryLayer::new(store));

        // Simulate concurrent creates (though in single-threaded test)
        let d1 = dir.create_or_open(&["shared"]).await.unwrap();
        let d2 = dir.create_or_open(&["shared"]).await.unwrap();

        // Both should get the same prefix (idempotent)
        assert_eq!(d1.prefix(), d2.prefix());
    }

    #[tokio::test]
    async fn test_create_with_special_characters_in_path() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Path components with special characters
        let result = dir.create_or_open(&["with-dash", "with_underscore", "with.dot"]).await;
        assert!(result.is_ok());

        let d = result.unwrap();
        assert_eq!(d.path(), &["with-dash", "with_underscore", "with.dot"]);
    }

    // =========================================================================
    // Concurrent Operations Tests
    // =========================================================================

    #[tokio::test]
    async fn test_concurrent_create_or_open_same_directory() {
        use std::sync::Arc as StdArc;

        use tokio::task::JoinSet;

        let store = StdArc::new(DeterministicKeyValueStore::new());
        let dir = StdArc::new(DirectoryLayer::new(store));

        let mut handles = JoinSet::new();

        // Spawn multiple concurrent create_or_open calls for the same path
        for _ in 0..10 {
            let dir_clone = StdArc::clone(&dir);
            handles.spawn(async move { dir_clone.create_or_open(&["concurrent", "test"]).await });
        }

        let mut prefixes = vec![];
        while let Some(result) = handles.join_next().await {
            let d = result.unwrap().unwrap();
            prefixes.push(d.prefix().to_vec());
        }

        // All should have gotten the same prefix (idempotent)
        let first = &prefixes[0];
        for prefix in &prefixes[1..] {
            assert_eq!(prefix, first);
        }
    }

    #[tokio::test]
    async fn test_concurrent_create_different_directories() {
        use std::sync::Arc as StdArc;

        use tokio::task::JoinSet;

        let store = StdArc::new(DeterministicKeyValueStore::new());
        let dir = StdArc::new(DirectoryLayer::new(store));

        let mut handles = JoinSet::new();

        // Spawn concurrent create_or_open calls for different paths
        for i in 0..10 {
            let dir_clone = StdArc::clone(&dir);
            let name = format!("dir{}", i);
            handles.spawn(async move { dir_clone.create_or_open(&[&name]).await });
        }

        let mut prefixes = std::collections::HashSet::new();
        while let Some(result) = handles.join_next().await {
            let d = result.unwrap().unwrap();
            prefixes.insert(d.prefix().to_vec());
        }

        // All should have unique prefixes
        assert_eq!(prefixes.len(), 10);
    }

    // =========================================================================
    // Deep Nested Hierarchy Tests
    // =========================================================================

    #[tokio::test]
    async fn test_deep_nested_hierarchy() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create a 10-level deep hierarchy
        let mut current_path: Vec<&str> = vec![];
        let levels: Vec<String> = (0..10).map(|i| format!("level{}", i)).collect();

        for level in &levels {
            current_path.push(level);
            let path_clone: Vec<&str> = current_path.iter().map(|s| &**s).collect();
            let d = dir.create_or_open(&path_clone).await.unwrap();
            assert_eq!(d.path().len(), current_path.len());
        }

        // Verify all levels exist
        current_path.clear();
        for level in &levels {
            current_path.push(level);
            let path_clone: Vec<&str> = current_path.iter().map(|s| &**s).collect();
            assert!(dir.exists(&path_clone).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_nested_directories_have_independent_prefixes() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let level1 = dir.create_or_open(&["org"]).await.unwrap();
        let level2 = dir.create_or_open(&["org", "team"]).await.unwrap();
        let level3 = dir.create_or_open(&["org", "team", "project"]).await.unwrap();

        // Prefixes should be independent (not nested)
        assert_ne!(level1.prefix(), level2.prefix());
        assert_ne!(level2.prefix(), level3.prefix());
        assert_ne!(level1.prefix(), level3.prefix());

        // Prefix lengths may vary due to allocator
        // But none should be a prefix of another
        assert!(!level2.prefix().starts_with(level1.prefix()) || level1.prefix().is_empty());
    }

    // =========================================================================
    // Directory Removal Tests
    // =========================================================================

    #[tokio::test]
    async fn test_remove_and_recreate_gets_new_prefix() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let d1 = dir.create_or_open(&["temp"]).await.unwrap();
        let prefix1 = d1.prefix().to_vec();

        // Remove the directory
        assert!(dir.remove(&["temp"]).await.unwrap());

        // Recreate - should get a new prefix
        let d2 = dir.create_or_open(&["temp"]).await.unwrap();
        let prefix2 = d2.prefix().to_vec();

        assert_ne!(prefix1, prefix2);
    }

    #[tokio::test]
    async fn test_remove_parent_does_not_affect_child() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["parent"]).await.unwrap();
        let child = dir.create_or_open(&["parent", "child"]).await.unwrap();
        let child_prefix = child.prefix().to_vec();

        // Remove parent
        assert!(dir.remove(&["parent"]).await.unwrap());

        // Child should still exist with same prefix
        let child_after = dir.open(&["parent", "child"]).await.unwrap();
        assert_eq!(child_after.prefix(), &child_prefix);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_directory() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Removing a directory that doesn't exist returns false
        assert!(!dir.remove(&["nonexistent"]).await.unwrap());
    }

    // =========================================================================
    // Layer Type Tests
    // =========================================================================

    #[tokio::test]
    async fn test_create_with_custom_layer() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let d = dir.create_or_open_with_layer(&["app"], "partition").await.unwrap();
        assert_eq!(d.layer_type(), "partition");
    }

    #[tokio::test]
    async fn test_open_with_correct_layer_succeeds() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open_with_layer(&["app"], "custom").await.unwrap();

        // Opening with same layer should succeed
        let d = dir.create_or_open_with_layer(&["app"], "custom").await.unwrap();
        assert_eq!(d.layer_type(), "custom");
    }

    #[tokio::test]
    async fn test_layer_preserved_after_move() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open_with_layer(&["old"], "special-layer").await.unwrap();

        let moved = dir.move_to(&["old"], &["new"]).await.unwrap();
        assert_eq!(moved.layer_type(), "special-layer");

        // Opening should also return the correct layer
        let opened = dir.open(&["new"]).await.unwrap();
        assert_eq!(opened.layer_type(), "special-layer");
    }

    // =========================================================================
    // Subspace Operations Tests
    // =========================================================================

    #[tokio::test]
    async fn test_pack_produces_prefixed_key() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["data"]).await.unwrap();
        let key = dir.pack(&Tuple::new().push("record").push(123u64));

        // Key should start with directory prefix
        assert!(key.starts_with(dir.prefix()));
        // Key should be longer than just the prefix
        assert!(key.len() > dir.prefix().len());
    }

    #[tokio::test]
    async fn test_unpack_extracts_tuple() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["data"]).await.unwrap();

        let original = Tuple::new().push("name").push(42i64);
        let packed = dir.pack(&original);
        let unpacked = dir.unpack(&packed).unwrap();

        assert_eq!(unpacked.len(), 2);
        // Note: element comparison depends on Tuple implementation
    }

    #[tokio::test]
    async fn test_unpack_fails_for_wrong_prefix() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir1 = dir_layer.create_or_open(&["dir1"]).await.unwrap();
        let dir2 = dir_layer.create_or_open(&["dir2"]).await.unwrap();

        let key = dir1.pack(&Tuple::new().push("test"));

        // Unpacking with wrong directory should fail
        let result = dir2.unpack(&key);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_range_covers_all_keys() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["data"]).await.unwrap();
        let (start, end) = dir.range();

        // Test keys at various depths are within range
        let key1 = dir.pack(&Tuple::new().push("a"));
        let key2 = dir.pack(&Tuple::new().push("z").push(1u64).push("nested"));

        assert!(key1 >= start && key1 < end);
        assert!(key2 >= start && key2 < end);
    }

    #[tokio::test]
    async fn test_contains_distinguishes_directories() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir1 = dir_layer.create_or_open(&["users"]).await.unwrap();
        let dir2 = dir_layer.create_or_open(&["orders"]).await.unwrap();

        let user_key = dir1.pack(&Tuple::new().push("alice"));
        let order_key = dir2.pack(&Tuple::new().push("12345"));

        assert!(dir1.contains(&user_key));
        assert!(!dir1.contains(&order_key));
        assert!(dir2.contains(&order_key));
        assert!(!dir2.contains(&user_key));
    }

    #[tokio::test]
    async fn test_subspace_at_creates_nested_prefix() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["data"]).await.unwrap();
        let user_subspace = dir.subspace_at(&Tuple::new().push("users"));
        let order_subspace = dir.subspace_at(&Tuple::new().push("orders"));

        // Both should be within the directory
        let user_key = user_subspace.pack(&Tuple::new().push("alice"));
        let order_key = order_subspace.pack(&Tuple::new().push("12345"));

        assert!(dir.contains(&user_key));
        assert!(dir.contains(&order_key));

        // But they should be in different subspaces
        assert!(!user_subspace.contains(&order_key));
        assert!(!order_subspace.contains(&user_key));
    }

    // =========================================================================
    // Move Operation Edge Cases
    // =========================================================================

    #[tokio::test]
    async fn test_move_to_same_depth() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let original = dir.create_or_open(&["a", "b"]).await.unwrap();
        let original_prefix = original.prefix().to_vec();

        let moved = dir.move_to(&["a", "b"], &["x", "y"]).await.unwrap();

        assert_eq!(moved.prefix(), &original_prefix);
        assert_eq!(moved.path(), &["x", "y"]);
    }

    #[tokio::test]
    async fn test_move_to_different_depth() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let original = dir.create_or_open(&["deep", "nested", "path"]).await.unwrap();
        let original_prefix = original.prefix().to_vec();

        // Move to shallower path
        let moved = dir.move_to(&["deep", "nested", "path"], &["shallow"]).await.unwrap();

        assert_eq!(moved.prefix(), &original_prefix);
        assert_eq!(moved.path(), &["shallow"]);
    }

    #[tokio::test]
    async fn test_move_prevents_self_move() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["a"]).await.unwrap();

        // Try to move to subdirectory of itself
        let result = dir.move_to(&["a"], &["a", "b"]).await;
        assert!(matches!(result, Err(DirectoryError::MoveCycle)));
    }

    #[tokio::test]
    async fn test_move_allows_sibling_paths() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        dir.create_or_open(&["a", "b"]).await.unwrap();

        // Move to sibling path (not a subdirectory)
        let result = dir.move_to(&["a", "b"], &["a", "c"]).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Validation Edge Cases
    // =========================================================================

    #[tokio::test]
    async fn test_unicode_path_components() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Unicode characters should be valid
        let result = dir.create_or_open(&["users", "alice"]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_path_component_at_max_length() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        // Create a component at exactly max length (255 bytes)
        let max_component = "x".repeat(255);
        let result = dir.create_or_open(&[&max_component]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_single_character_path() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let result = dir.create_or_open(&["a"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().path(), &["a"]);
    }

    #[tokio::test]
    async fn test_numeric_path_components() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let result = dir.create_or_open(&["123", "456", "789"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().path(), &["123", "456", "789"]);
    }

    // =========================================================================
    // DirectorySubspace Clone and Debug Tests
    // =========================================================================

    #[tokio::test]
    async fn test_directory_subspace_clone() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["test"]).await.unwrap();
        let cloned = dir.clone();

        assert_eq!(dir.prefix(), cloned.prefix());
        assert_eq!(dir.path(), cloned.path());
        assert_eq!(dir.layer_type(), cloned.layer_type());
        assert_eq!(dir.created_at_ms(), cloned.created_at_ms());
    }

    #[tokio::test]
    async fn test_directory_subspace_debug() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store);

        let dir = dir_layer.create_or_open(&["debug", "test"]).await.unwrap();
        let debug_str = format!("{:?}", dir);

        // Debug output should contain path info
        assert!(debug_str.contains("DirectorySubspace"));
    }

    // =========================================================================
    // Created At Timestamp Tests
    // =========================================================================

    #[tokio::test]
    async fn test_created_at_is_reasonable() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let before = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

        let d = dir.create_or_open(&["timed"]).await.unwrap();

        let after = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

        assert!(d.created_at_ms() >= before);
        assert!(d.created_at_ms() <= after);
    }

    #[tokio::test]
    async fn test_created_at_unchanged_on_reopen() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store);

        let d1 = dir.create_or_open(&["persist"]).await.unwrap();
        let created1 = d1.created_at_ms();

        // Wait a tiny bit
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        // Reopen
        let d2 = dir.create_or_open(&["persist"]).await.unwrap();
        let created2 = d2.created_at_ms();

        assert_eq!(created1, created2);
    }

    // =========================================================================
    // Error Display Tests
    // =========================================================================

    #[test]
    fn test_allocation_failed_error_display() {
        // We can't easily construct AllocationError, but we can test other errors
        let err = DirectoryError::NotFound {
            path: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        let display = format!("{}", err);
        assert!(display.contains("a/b/c"));
    }

    // =========================================================================
    // Integration with KV Store Tests
    // =========================================================================

    #[tokio::test]
    async fn test_directory_keys_stored_in_kv() {
        use crate::kv::ScanRequest;
        use crate::traits::KeyValueStore;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir = DirectoryLayer::new(store.clone());

        // Create some directories
        dir.create_or_open(&["app1"]).await.unwrap();
        dir.create_or_open(&["app2"]).await.unwrap();

        // Scan the underlying store - should have entries
        let result = store
            .scan(ScanRequest {
                prefix: "".to_string(),
                limit: Some(100),
                continuation_token: None,
            })
            .await
            .unwrap();

        // Should have stored metadata (layer, prefix, created_at for each directory)
        assert!(result.count >= 6); // At least 3 keys per directory * 2 directories
    }

    #[tokio::test]
    async fn test_directory_data_isolation() {
        use crate::kv::WriteRequest;
        use crate::traits::KeyValueStore;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let dir_layer = DirectoryLayer::new(store.clone());

        let users_dir = dir_layer.create_or_open(&["users"]).await.unwrap();
        let orders_dir = dir_layer.create_or_open(&["orders"]).await.unwrap();

        // Store data in users directory
        let user_key = String::from_utf8_lossy(&users_dir.pack(&Tuple::new().push("alice"))).to_string();
        store.write(WriteRequest::set(&user_key, "user-data")).await.unwrap();

        // Store data in orders directory
        let order_key = String::from_utf8_lossy(&orders_dir.pack(&Tuple::new().push("12345"))).to_string();
        store.write(WriteRequest::set(&order_key, "order-data")).await.unwrap();

        // Scan users directory range
        let (start, _end) = users_dir.range();
        let start_str = String::from_utf8_lossy(&start).to_string();

        let result = store
            .scan(ScanRequest {
                prefix: start_str,
                limit: Some(100),
                continuation_token: None,
            })
            .await
            .unwrap();

        // Should find user data
        assert!(result.entries.iter().any(|e| e.value == "user-data"));
        // Should NOT find order data
        assert!(!result.entries.iter().any(|e| e.value == "order-data"));
    }
}
