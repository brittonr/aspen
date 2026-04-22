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

use snafu::Snafu;

use super::AllocationError;
use crate::error::KeyValueStoreError;

// Submodules
mod layer;
mod operations;
mod subspace;
mod validation;

#[cfg(test)]
mod tests;

// Re-exports
pub use layer::DirectoryLayer;
pub use subspace::DirectorySubspace;

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
