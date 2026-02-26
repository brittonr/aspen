//! Query operations for blob storage.
//!
//! The `BlobQuery` trait provides methods for listing and waiting for blobs.

use std::time::Duration;

use async_trait::async_trait;
use iroh_blobs::Hash;

use crate::store::BlobStoreError;
use crate::types::BlobListResult;

/// Trait for query operations on blob storage.
///
/// Provides methods for listing blobs and waiting for blobs to become available.
#[async_trait]
pub trait BlobQuery: Send + Sync {
    /// List blobs with pagination.
    async fn list(&self, limit: u32, continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError>;

    /// Wait for a blob to be available locally with bounded timeout.
    ///
    /// Tiger Style: Explicit timeout prevents unbounded waiting.
    ///
    /// # Arguments
    /// * `hash` - The blob hash to wait for
    /// * `timeout` - Maximum time to wait
    ///
    /// # Returns
    /// * `Ok(true)` - Blob is available
    /// * `Ok(false)` - Timeout reached, blob not available
    /// * `Err(_)` - Storage error
    async fn wait_available(&self, hash: &Hash, timeout: Duration) -> Result<bool, BlobStoreError>;

    /// Wait for multiple blobs to be available locally.
    ///
    /// Tiger Style: Bounded batch operation with single timeout for all.
    ///
    /// # Arguments
    /// * `hashes` - The blob hashes to wait for
    /// * `timeout` - Maximum time to wait for ALL blobs
    ///
    /// # Returns
    /// * `Ok(missing)` - Vec of hashes that are still missing (empty = all available)
    /// * `Err(_)` - Storage error
    async fn wait_available_all(&self, hashes: &[Hash], timeout: Duration) -> Result<Vec<Hash>, BlobStoreError>;
}
