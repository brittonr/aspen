//! Change storage backed by iroh-blobs.
//!
//! This module implements Pijul change storage using Aspen's blob store,
//! enabling P2P distribution of changes via iroh-blobs.

use std::sync::Arc;

use aspen_blob::prelude::*;
use lru::LruCache;
use parking_lot::RwLock;
use tracing::debug;
use tracing::instrument;

use super::constants::CHANGE_CACHE_SIZE;
use super::constants::MAX_CHANGE_SIZE_BYTES;
use super::error::PijulError;
use super::error::PijulResult;
use super::types::ChangeHash;

/// Stores Pijul changes in iroh-blobs.
///
/// Changes are stored as compressed blobs in iroh-blobs, making them
/// automatically content-addressed (BLAKE3) and P2P distributable.
///
/// # Storage Format
///
/// Changes are stored as:
/// 1. Serialized using libpijul's format (bincode)
/// 2. Compressed with zstd-seekable
/// 3. Content-addressed by BLAKE3 hash (same as iroh-blobs)
///
/// # Caching
///
/// Recently accessed changes are cached in memory using an LRU cache
/// to avoid repeated decompression and deserialization.
pub struct AspenChangeStore<B: BlobStore> {
    /// The underlying blob store.
    blobs: Arc<B>,

    /// LRU cache of recently accessed change bytes (compressed).
    /// We cache the compressed bytes rather than deserialized changes
    /// to avoid libpijul type dependencies in this layer.
    cache: RwLock<LruCache<ChangeHash, Vec<u8>>>,
}

impl<B: BlobStore> AspenChangeStore<B> {
    /// Create a new change store backed by the given blob store.
    pub fn new(blobs: Arc<B>) -> Self {
        Self {
            blobs,
            cache: RwLock::new(LruCache::new(std::num::NonZeroUsize::new(CHANGE_CACHE_SIZE).unwrap())),
        }
    }

    /// Store a change and return its hash.
    ///
    /// The change should already be serialized and compressed in libpijul's format.
    #[instrument(skip(self, data), fields(size = data.len()))]
    pub async fn store_change(&self, data: &[u8]) -> PijulResult<ChangeHash> {
        // Validate size
        let size = data.len() as u64;
        if size > MAX_CHANGE_SIZE_BYTES {
            return Err(PijulError::ChangeTooLarge {
                size,
                max: MAX_CHANGE_SIZE_BYTES,
            });
        }

        // Store in iroh-blobs
        let result =
            self.blobs.add_bytes(data).await.map_err(|e| PijulError::BlobStorage { message: e.to_string() })?;

        // The BLAKE3 hash from iroh-blobs IS the change hash
        let hash = ChangeHash::from_iroh_hash(result.blob_ref.hash);

        // Cache the change
        {
            let mut cache = self.cache.write();
            cache.put(hash, data.to_vec());
        }

        debug!(hash = %hash, size, "change stored");
        Ok(hash)
    }

    /// Retrieve a change by hash.
    ///
    /// Returns the raw compressed bytes. The caller is responsible for
    /// decompressing and deserializing using libpijul.
    #[instrument(skip(self))]
    pub async fn get_change(&self, hash: &ChangeHash) -> PijulResult<Option<Vec<u8>>> {
        // Check cache first
        {
            let mut cache = self.cache.write();
            if let Some(data) = cache.get(hash) {
                debug!(hash = %hash, "change cache hit");
                return Ok(Some(data.clone()));
            }
        }

        // Fetch from blob store
        let iroh_hash = hash.to_iroh_hash();
        let bytes = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| PijulError::BlobStorage { message: e.to_string() })?;

        match bytes {
            Some(data) => {
                let data_vec = data.to_vec();

                // Cache for future access
                {
                    let mut cache = self.cache.write();
                    cache.put(*hash, data_vec.clone());
                }

                debug!(hash = %hash, size = data_vec.len(), "change retrieved");
                Ok(Some(data_vec))
            }
            None => {
                debug!(hash = %hash, "change not found");
                Ok(None)
            }
        }
    }

    /// Check if a change exists in the store.
    #[instrument(skip(self))]
    pub async fn has_change(&self, hash: &ChangeHash) -> PijulResult<bool> {
        // Check cache first
        {
            let cache = self.cache.read();
            if cache.contains(hash) {
                return Ok(true);
            }
        }

        // Check blob store
        let iroh_hash = hash.to_iroh_hash();
        self.blobs.has(&iroh_hash).await.map_err(|e| PijulError::BlobStorage { message: e.to_string() })
    }

    /// Download a change from a remote peer.
    ///
    /// Uses iroh-blobs P2P to fetch the change from the specified peer.
    #[instrument(skip(self))]
    pub async fn download_from_peer(&self, hash: &ChangeHash, provider: iroh::PublicKey) -> PijulResult<()> {
        let iroh_hash = hash.to_iroh_hash();

        self.blobs.download_from_peer(&iroh_hash, provider).await.map_err(|e| PijulError::FetchFailed {
            hash: hash.to_hex(),
            message: e.to_string(),
        })?;

        debug!(hash = %hash, provider = %provider.fmt_short(), "change downloaded from peer");
        Ok(())
    }

    /// Protect a change from garbage collection.
    ///
    /// Creates a persistent tag in iroh-blobs to prevent the change
    /// from being garbage collected.
    #[instrument(skip(self))]
    pub async fn protect(&self, hash: &ChangeHash, tag_name: &str) -> PijulResult<()> {
        let iroh_hash = hash.to_iroh_hash();
        self.blobs
            .protect(&iroh_hash, tag_name)
            .await
            .map_err(|e| PijulError::BlobStorage { message: e.to_string() })
    }

    /// Remove protection from a change.
    #[instrument(skip(self))]
    pub async fn unprotect(&self, tag_name: &str) -> PijulResult<()> {
        self.blobs.unprotect(tag_name).await.map_err(|e| PijulError::BlobStorage { message: e.to_string() })
    }

    /// Clear the change cache.
    ///
    /// Useful for testing or when memory pressure is high.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        debug!("change cache cleared");
    }

    /// Get current cache size.
    pub fn cache_size(&self) -> usize {
        let cache = self.cache.read();
        cache.len()
    }
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;

    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve_change() {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let store = AspenChangeStore::new(blobs);

        // Store some test data (simulating a compressed change)
        let data = b"test change data - pretend this is compressed pijul change";
        let hash = store.store_change(data).await.unwrap();

        // Retrieve it
        let retrieved = store.get_change(&hash).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);

        // Check existence
        assert!(store.has_change(&hash).await.unwrap());

        // Check non-existent
        let fake_hash = ChangeHash([0u8; 32]);
        assert!(!store.has_change(&fake_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_behavior() {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let store = AspenChangeStore::new(blobs);

        let data = b"cached change data";
        let hash = store.store_change(data).await.unwrap();

        // Should be cached after store
        assert_eq!(store.cache_size(), 1);

        // Clear cache
        store.clear_cache();
        assert_eq!(store.cache_size(), 0);

        // Retrieve should re-cache
        let _ = store.get_change(&hash).await.unwrap();
        assert_eq!(store.cache_size(), 1);
    }

    #[tokio::test]
    async fn test_change_too_large() {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let store = AspenChangeStore::new(blobs);

        // Create data larger than MAX_CHANGE_SIZE_BYTES
        let large_data = vec![0u8; (MAX_CHANGE_SIZE_BYTES + 1) as usize];
        let result = store.store_change(&large_data).await;

        let err = result.expect_err("expected ChangeTooLarge error");
        assert!(
            matches!(
                err,
                PijulError::ChangeTooLarge { size, max }
                if size == MAX_CHANGE_SIZE_BYTES + 1 && max == MAX_CHANGE_SIZE_BYTES
            ),
            "expected ChangeTooLarge with size {} and max {}, got {:?}",
            MAX_CHANGE_SIZE_BYTES + 1,
            MAX_CHANGE_SIZE_BYTES,
            err
        );
    }
}
