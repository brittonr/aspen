//! Read operations for blob storage.
//!
//! The `BlobRead` trait provides methods for retrieving and checking blob status.

use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use iroh_blobs::Hash;

use crate::store::AsyncReadSeek;
use crate::store::BlobStoreError;
use crate::types::BlobStatus;

/// Trait for read operations on blob storage.
///
/// Provides methods for retrieving blob data and checking blob status.
#[async_trait]
pub trait BlobRead: Send + Sync {
    /// Get bytes by hash.
    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError>;

    /// Check if blob exists and is complete.
    async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError>;

    /// Get blob status.
    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError>;

    /// Get a streaming reader for a blob.
    ///
    /// Returns an async reader positioned at the start of the blob.
    /// This enables streaming large blobs without loading them entirely into memory,
    /// which is critical for serving large NAR files over HTTP/3.
    ///
    /// The returned reader implements `AsyncRead + AsyncSeek + Unpin + Send`,
    /// allowing efficient partial reads and range request support.
    ///
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the blob to read
    ///
    /// # Returns
    /// * `Ok(Some(reader))` - Blob exists, reader is ready
    /// * `Ok(None)` - Blob not found
    /// * `Err(_)` - Storage error
    ///
    /// # Tiger Style
    /// Streaming operation - does not buffer entire blob in memory.
    async fn reader(&self, hash: &Hash) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError>;
}
