//! In-memory blob store for testing.

use std::io::Cursor;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use iroh_blobs::ticket::BlobTicket;

use super::async_read_seek::AsyncReadSeek;
use super::errors::BlobStoreError;
use crate::constants::MAX_BLOB_SIZE;
use crate::traits::BlobQuery;
use crate::traits::BlobRead;
use crate::traits::BlobTransfer;
use crate::traits::BlobWrite;
use crate::types::AddBlobResult;
use crate::types::BlobListEntry;
use crate::types::BlobListResult;
use crate::types::BlobRef;
use crate::types::BlobStatus;

/// In-memory blob store for testing.
///
/// This is a simple hash-based store that keeps blobs in memory.
/// Useful for unit tests that don't need full iroh-blobs functionality.
///
/// This type is Clone-able - clones share the same underlying storage.
///
/// Uses `parking_lot::RwLock` instead of `std::sync::RwLock` for:
/// - No lock poisoning (panics in critical section don't poison the lock)
/// - Better performance (faster lock acquisition)
/// - Smaller memory footprint
#[derive(Clone, Default)]
pub struct InMemoryBlobStore {
    blobs: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<Hash, Bytes>>>,
}

impl InMemoryBlobStore {
    /// Create a new in-memory blob store.
    pub fn new() -> Self {
        Self::default()
    }
}

// =============================================================================
// InMemoryBlobStore trait implementations
// =============================================================================

#[async_trait]
impl BlobWrite for InMemoryBlobStore {
    async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError> {
        let size = data.len() as u64;
        if size > MAX_BLOB_SIZE {
            return Err(BlobStoreError::TooLarge {
                size,
                max: MAX_BLOB_SIZE,
            });
        }

        let hash = Hash::new(data);
        let bytes = Bytes::copy_from_slice(data);

        let was_new = {
            let mut blobs = self.blobs.write();
            blobs.insert(hash, bytes).is_none()
        };

        Ok(AddBlobResult {
            blob_ref: BlobRef::new(hash, size, BlobFormat::Raw),
            was_new,
        })
    }

    async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError> {
        let data = std::fs::read(path).map_err(|e| BlobStoreError::Storage {
            message: format!("failed to read file: {}", e),
        })?;
        self.add_bytes(&data).await
    }

    async fn protect(&self, _hash: &Hash, _tag_name: &str) -> Result<(), BlobStoreError> {
        // No-op for in-memory store
        Ok(())
    }

    async fn unprotect(&self, _tag_name: &str) -> Result<(), BlobStoreError> {
        // No-op for in-memory store
        Ok(())
    }
}

#[async_trait]
impl BlobRead for InMemoryBlobStore {
    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError> {
        let blobs = self.blobs.read();
        Ok(blobs.get(hash).cloned())
    }

    async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError> {
        let blobs = self.blobs.read();
        Ok(blobs.contains_key(hash))
    }

    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError> {
        let blobs = self.blobs.read();
        Ok(blobs.get(hash).map(|b| BlobStatus {
            hash: *hash,
            size_bytes: Some(b.len() as u64),
            complete: true,
            tags: Vec::new(),
        }))
    }

    async fn reader(&self, hash: &Hash) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError> {
        // For in-memory store, we wrap the bytes in a Cursor
        let blobs = self.blobs.read();
        match blobs.get(hash).cloned() {
            Some(bytes) => {
                let cursor = Cursor::new(bytes);
                Ok(Some(Box::pin(cursor)))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl BlobTransfer for InMemoryBlobStore {
    async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError> {
        // In-memory store can't create tickets since there's no endpoint
        Err(BlobStoreError::Storage {
            message: format!("in-memory store cannot create tickets for {}", hash),
        })
    }

    async fn download(&self, _ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError> {
        // In-memory store can't download from peers
        Err(BlobStoreError::Storage {
            message: "in-memory store cannot download from peers".to_string(),
        })
    }
}

#[async_trait]
impl BlobQuery for InMemoryBlobStore {
    async fn list(&self, limit: u32, _continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError> {
        let blobs = self.blobs.read();
        let entries: Vec<_> = blobs
            .iter()
            .take(limit as usize)
            .map(|(hash, bytes)| BlobListEntry {
                hash: *hash,
                size_bytes: bytes.len() as u64,
                format: BlobFormat::Raw,
            })
            .collect();

        Ok(BlobListResult {
            blobs: entries,
            continuation_token: None,
        })
    }

    async fn wait_available(&self, hash: &Hash, _timeout: Duration) -> Result<bool, BlobStoreError> {
        // In-memory store: blobs are immediately available after add_bytes
        // No need to wait - just check if it exists
        self.has(hash).await
    }

    async fn wait_available_all(&self, hashes: &[Hash], _timeout: Duration) -> Result<Vec<Hash>, BlobStoreError> {
        // In-memory store: blobs are immediately available after add_bytes
        // Return any that don't exist
        let mut missing = Vec::new();
        for hash in hashes {
            if !self.has(hash).await? {
                missing.push(*hash);
            }
        }
        Ok(missing)
    }
}
