use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use aspen_blob::AddBlobResult;
use aspen_blob::BlobListEntry;
use aspen_blob::BlobListResult;
use aspen_blob::BlobQuery;
use aspen_blob::BlobRead;
use aspen_blob::BlobRef;
use aspen_blob::BlobStatus;
use aspen_blob::BlobStore;
use aspen_blob::BlobStoreError;
use aspen_blob::BlobTransfer;
use aspen_blob::BlobWrite;
use async_trait::async_trait;
use bytes::Bytes;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;

#[derive(Clone, Default)]
pub struct FixtureBlobStore {
    blobs: Arc<Mutex<BTreeMap<Hash, Vec<u8>>>>,
}

impl FixtureBlobStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BlobWrite for FixtureBlobStore {
    async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError> {
        let hash = Hash::new(*blake3::hash(data).as_bytes());
        self.blobs.lock().expect("fixture lock poisoned").insert(hash, data.to_vec());
        Ok(AddBlobResult {
            blob_ref: BlobRef::new(hash, data.len() as u64, BlobFormat::Raw),
            was_new: true,
        })
    }

    async fn add_path(&self, path: &std::path::Path) -> Result<AddBlobResult, BlobStoreError> {
        let data = std::fs::read(path).map_err(|err| BlobStoreError::AddPath {
            message: err.to_string(),
        })?;
        self.add_bytes(&data).await
    }

    async fn protect(&self, _hash: &Hash, _tag_name: &str) -> Result<(), BlobStoreError> {
        Ok(())
    }

    async fn unprotect(&self, _tag_name: &str) -> Result<(), BlobStoreError> {
        Ok(())
    }
}

#[async_trait]
impl BlobRead for FixtureBlobStore {
    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError> {
        Ok(self.blobs.lock().expect("fixture lock poisoned").get(hash).cloned().map(Bytes::from))
    }

    async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError> {
        Ok(self.blobs.lock().expect("fixture lock poisoned").contains_key(hash))
    }

    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError> {
        Ok(self.blobs.lock().expect("fixture lock poisoned").get(hash).map(|data| BlobStatus {
            hash: *hash,
            size_bytes: Some(data.len() as u64),
            is_complete: true,
            tags: Vec::new(),
        }))
    }

    async fn reader(&self, _hash: &Hash) -> Result<Option<Pin<Box<dyn aspen_blob::AsyncReadSeek>>>, BlobStoreError> {
        Ok(None)
    }
}

#[async_trait]
impl BlobQuery for FixtureBlobStore {
    async fn list(&self, limit: u32, _continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError> {
        let blobs = self
            .blobs
            .lock()
            .expect("fixture lock poisoned")
            .iter()
            .take(limit as usize)
            .map(|(hash, data)| BlobListEntry {
                hash: *hash,
                size_bytes: data.len() as u64,
                format: BlobFormat::Raw,
            })
            .collect();
        Ok(BlobListResult {
            blobs,
            continuation_token: None,
        })
    }

    async fn wait_available(&self, hash: &Hash, _timeout: Duration) -> Result<bool, BlobStoreError> {
        self.has(hash).await
    }

    async fn wait_available_all(&self, hashes: &[Hash], _timeout: Duration) -> Result<Vec<Hash>, BlobStoreError> {
        let mut missing = Vec::new();
        for hash in hashes {
            if !self.has(hash).await? {
                missing.push(*hash);
            }
        }
        Ok(missing)
    }
}

#[async_trait]
impl BlobTransfer for FixtureBlobStore {
    async fn ticket(&self, _hash: &Hash) -> Result<iroh_blobs::ticket::BlobTicket, BlobStoreError> {
        Err(BlobStoreError::Storage {
            message: "fixture does not mint tickets".to_string(),
        })
    }

    async fn download(&self, _ticket: &iroh_blobs::ticket::BlobTicket) -> Result<BlobRef, BlobStoreError> {
        Err(BlobStoreError::Download {
            message: "fixture does not download".to_string(),
        })
    }
}

pub fn assert_full_store<T: BlobStore>(_store: &T) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn downstream_can_use_public_blob_traits() {
        let store = FixtureBlobStore::new();
        let result = store.add_bytes(b"portable blob").await.unwrap();
        assert_full_store(&store);
        assert!(store.has(&result.blob_ref.hash).await.unwrap());
        assert_eq!(
            store.get_bytes(&result.blob_ref.hash).await.unwrap().unwrap(),
            Bytes::from_static(b"portable blob")
        );
    }
}
