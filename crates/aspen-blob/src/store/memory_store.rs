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
            is_complete: true,
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

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::traits::BlobQuery;
    use crate::traits::BlobRead;
    use crate::traits::BlobTransfer;
    use crate::traits::BlobWrite;

    #[tokio::test]
    async fn test_new_store_is_empty() {
        let store = InMemoryBlobStore::new();
        let result = store.list(100, None).await.unwrap();
        assert!(result.blobs.is_empty());
        assert_eq!(result.continuation_token, None);
    }

    #[tokio::test]
    async fn test_add_bytes_returns_hash() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();

        assert_eq!(result.blob_ref.size_bytes, data.len() as u64);
        assert!(result.was_new);
        assert_eq!(result.blob_ref.format, BlobFormat::Raw);

        // Verify the hash is correct
        let expected_hash = Hash::new(data);
        assert_eq!(result.blob_ref.hash, expected_hash);
    }

    #[tokio::test]
    async fn test_add_bytes_duplicate_not_new() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result1 = store.add_bytes(data).await.unwrap();
        assert!(result1.was_new);

        let result2 = store.add_bytes(data).await.unwrap();
        assert!(!result2.was_new);
        assert_eq!(result1.blob_ref.hash, result2.blob_ref.hash);
    }

    #[tokio::test]
    async fn test_get_bytes_returns_data() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();
        let retrieved = store.get_bytes(&result.blob_ref.hash).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().as_ref(), data);
    }

    #[tokio::test]
    async fn test_get_bytes_missing_returns_none() {
        let store = InMemoryBlobStore::new();
        let fake_hash = Hash::new(b"nonexistent");

        let result = store.get_bytes(&fake_hash).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_has_existing_blob() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();
        let exists = store.has(&result.blob_ref.hash).await.unwrap();

        assert!(exists);
    }

    #[tokio::test]
    async fn test_has_missing_blob() {
        let store = InMemoryBlobStore::new();
        let fake_hash = Hash::new(b"nonexistent");

        let exists = store.has(&fake_hash).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_status_existing_blob() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();
        let status = store.status(&result.blob_ref.hash).await.unwrap();

        assert!(status.is_some());
        let status = status.unwrap();
        assert_eq!(status.hash, result.blob_ref.hash);
        assert_eq!(status.size_bytes, Some(data.len() as u64));
        assert!(status.is_complete);
        assert!(status.tags.is_empty());
    }

    #[tokio::test]
    async fn test_status_missing_blob() {
        let store = InMemoryBlobStore::new();
        let fake_hash = Hash::new(b"nonexistent");

        let status = store.status(&fake_hash).await.unwrap();
        assert!(status.is_none());
    }

    #[tokio::test]
    async fn test_reader_returns_data() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();
        let reader = store.reader(&result.blob_ref.hash).await.unwrap();

        assert!(reader.is_some());
        let mut reader = reader.unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf.as_slice(), data);
    }

    #[tokio::test]
    async fn test_reader_missing_returns_none() {
        let store = InMemoryBlobStore::new();
        let fake_hash = Hash::new(b"nonexistent");

        let reader = store.reader(&fake_hash).await.unwrap();
        assert!(reader.is_none());
    }

    #[tokio::test]
    async fn test_list_blobs() {
        let store = InMemoryBlobStore::new();

        // Add 3 blobs
        let data1 = b"blob one";
        let data2 = b"blob two";
        let data3 = b"blob three";

        let result1 = store.add_bytes(data1).await.unwrap();
        let result2 = store.add_bytes(data2).await.unwrap();
        let result3 = store.add_bytes(data3).await.unwrap();

        let list_result = store.list(100, None).await.unwrap();

        assert_eq!(list_result.blobs.len(), 3);
        assert_eq!(list_result.continuation_token, None);

        // Verify all hashes are in the list
        let hashes: Vec<_> = list_result.blobs.iter().map(|e| e.hash).collect();
        assert!(hashes.contains(&result1.blob_ref.hash));
        assert!(hashes.contains(&result2.blob_ref.hash));
        assert!(hashes.contains(&result3.blob_ref.hash));
    }

    #[tokio::test]
    async fn test_list_with_limit() {
        let store = InMemoryBlobStore::new();

        // Add 5 blobs
        for i in 0..5 {
            let data = format!("blob {}", i);
            store.add_bytes(data.as_bytes()).await.unwrap();
        }

        let list_result = store.list(2, None).await.unwrap();
        assert_eq!(list_result.blobs.len(), 2);
    }

    #[tokio::test]
    async fn test_add_bytes_too_large() {
        let store = InMemoryBlobStore::new();

        // Create data larger than MAX_BLOB_SIZE
        let size = (MAX_BLOB_SIZE + 1) as usize;
        let large_data = vec![0u8; size];

        let result = store.add_bytes(&large_data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            BlobStoreError::TooLarge { size: s, max } => {
                assert_eq!(s, size as u64);
                assert_eq!(max, MAX_BLOB_SIZE);
            }
            _ => panic!("expected TooLarge error"),
        }
    }

    #[tokio::test]
    async fn test_ticket_returns_error() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();
        let ticket_result = store.ticket(&result.blob_ref.hash).await;

        assert!(ticket_result.is_err());
        match ticket_result.unwrap_err() {
            BlobStoreError::Storage { message } => {
                assert!(message.contains("cannot create tickets"));
            }
            _ => panic!("expected Storage error"),
        }
    }

    // Note: test_download_returns_error omitted - requires creating a BlobTicket
    // which needs iroh endpoint setup. The download() method is documented to
    // return an error for in-memory store, which is verified manually.

    #[tokio::test]
    async fn test_wait_available_existing() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();
        let available = store.wait_available(&result.blob_ref.hash, Duration::from_secs(1)).await.unwrap();

        assert!(available);
    }

    #[tokio::test]
    async fn test_wait_available_missing() {
        let store = InMemoryBlobStore::new();
        let fake_hash = Hash::new(b"nonexistent");

        let available = store.wait_available(&fake_hash, Duration::from_secs(1)).await.unwrap();

        assert!(!available);
    }

    #[tokio::test]
    async fn test_wait_available_all_none_missing() {
        let store = InMemoryBlobStore::new();

        // Add 3 blobs
        let result1 = store.add_bytes(b"blob one").await.unwrap();
        let result2 = store.add_bytes(b"blob two").await.unwrap();
        let result3 = store.add_bytes(b"blob three").await.unwrap();

        let hashes = vec![result1.blob_ref.hash, result2.blob_ref.hash, result3.blob_ref.hash];

        let missing = store.wait_available_all(&hashes, Duration::from_secs(1)).await.unwrap();

        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn test_wait_available_all_some_missing() {
        let store = InMemoryBlobStore::new();

        let result1 = store.add_bytes(b"blob one").await.unwrap();
        let fake_hash = Hash::new(b"nonexistent");
        let result3 = store.add_bytes(b"blob three").await.unwrap();

        let hashes = vec![result1.blob_ref.hash, fake_hash, result3.blob_ref.hash];

        let missing = store.wait_available_all(&hashes, Duration::from_secs(1)).await.unwrap();

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], fake_hash);
    }

    #[tokio::test]
    async fn test_clone_shares_storage() {
        let store1 = InMemoryBlobStore::new();
        let store2 = store1.clone();

        let data = b"test data";
        let result = store1.add_bytes(data).await.unwrap();

        // Verify the blob is accessible from the cloned store
        let retrieved = store2.get_bytes(&result.blob_ref.hash).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().as_ref(), data);

        // Add to store2 and verify it's visible in store1
        let data2 = b"another blob";
        let result2 = store2.add_bytes(data2).await.unwrap();

        let retrieved2 = store1.get_bytes(&result2.blob_ref.hash).await.unwrap();
        assert!(retrieved2.is_some());
        assert_eq!(retrieved2.unwrap().as_ref(), data2);
    }

    #[tokio::test]
    async fn test_protect_unprotect_noop() {
        let store = InMemoryBlobStore::new();
        let data = b"test data";

        let result = store.add_bytes(data).await.unwrap();

        // These should succeed (no-op for in-memory store)
        store.protect(&result.blob_ref.hash, "test_tag").await.unwrap();
        store.unprotect("test_tag").await.unwrap();

        // Blob should still be there
        let exists = store.has(&result.blob_ref.hash).await.unwrap();
        assert!(exists);
    }
}
