//! Write operations for blob storage.
//!
//! The `BlobWrite` trait provides methods for adding and protecting blobs.

use std::path::Path;

use async_trait::async_trait;
use iroh_blobs::Hash;

use crate::store::BlobStoreError;
use crate::types::AddBlobResult;

/// Trait for write operations on blob storage.
///
/// Provides methods for adding blobs and managing their protection tags
/// to prevent garbage collection.
#[async_trait]
pub trait BlobWrite: Send + Sync {
    /// Add bytes to the store, returns a reference to the stored blob.
    async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError>;

    /// Add from a file path.
    async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError>;

    /// Create a persistent tag to prevent GC.
    async fn protect(&self, hash: &Hash, tag_name: &str) -> Result<(), BlobStoreError>;

    /// Remove protection tag.
    async fn unprotect(&self, tag_name: &str) -> Result<(), BlobStoreError>;
}
