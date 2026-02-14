//! DirectorySubspace - a directory with an allocated prefix.

use super::super::Subspace;
use super::super::SubspaceError;
use super::super::Tuple;

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
    pub(super) fn new(subspace: Subspace, path: Vec<String>, layer: String, created_at_ms: u64) -> Self {
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

    /// Get the layer (internal access for sibling modules).
    pub(super) fn layer_ref(&self) -> &str {
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
