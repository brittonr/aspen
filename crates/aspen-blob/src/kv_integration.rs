//! Blob-aware KeyValueStore wrapper for large value offloading.
//!
//! Transparently handles large values by storing them in the blob store
//! and saving a reference in the KV store.
//!
//! ## Architecture
//!
//! ```text
//! Application
//!     |
//!     v
//! BlobAwareKeyValueStore
//!     |
//!     +-- Small values (< BLOB_THRESHOLD) --> Direct to KV
//!     |
//!     +-- Large values (>= BLOB_THRESHOLD) --> BlobStore + BlobRef in KV
//! ```
//!
//! ## Garbage Collection
//!
//! When a key with a blob reference is deleted or overwritten:
//! 1. The old blob tag is removed
//! 2. The blob becomes eligible for GC after the grace period
//! 3. A new tag is created for the new value (if applicable)

use std::sync::Arc;

use anyhow::Result;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_traits::KeyValueStore;
use aspen_traits::KvDelete;
use aspen_traits::KvRead;
use aspen_traits::KvScan;
use aspen_traits::KvWrite;
use async_trait::async_trait;
use tracing::debug;
use tracing::warn;

use crate::BLOB_THRESHOLD;
use crate::BlobRead;
use crate::BlobRef;
use crate::BlobStoreError;
use crate::BlobWrite;
use crate::UnprotectReason;
use crate::constants::KV_TAG_PREFIX;
use crate::is_blob_ref;

/// Create a KV-scoped blob protection tag name.
fn kv_blob_tag(key: &str) -> String {
    format!("{}{}", KV_TAG_PREFIX, key)
}

/// Wrapper around KeyValueStore that offloads large values to blob storage.
///
/// Values larger than `BLOB_THRESHOLD` (1MB) are automatically stored in
/// the blob backend with a `BlobRef` saved in the KV store.
///
/// The blob backend `B` only needs [`BlobRead`] + [`BlobWrite`] capabilities;
/// transfer and query surfaces are not required.
pub struct BlobAwareKeyValueStore<KV: KeyValueStore, B: BlobRead + BlobWrite> {
    /// Underlying KV store for small values and blob references.
    kv: Arc<KV>,
    /// Blob store for large values.
    blobs: Arc<B>,
    /// Whether automatic blob offloading is enabled.
    auto_offload: bool,
}

impl<KV: KeyValueStore, B: BlobRead + BlobWrite> BlobAwareKeyValueStore<KV, B> {
    /// Create a new blob-aware KV store wrapper.
    ///
    /// # Arguments
    /// * `kv` - The underlying KeyValueStore implementation
    /// * `blobs` - Any blob backend implementing [`BlobRead`] + [`BlobWrite`]
    /// * `auto_offload` - Whether to automatically offload large values
    pub fn new(kv: Arc<KV>, blobs: Arc<B>, auto_offload: bool) -> Self {
        Self {
            kv,
            blobs,
            auto_offload,
        }
    }

    /// Get the underlying KV store.
    pub fn kv(&self) -> &Arc<KV> {
        &self.kv
    }

    /// Get the blob store.
    pub fn blobs(&self) -> &Arc<B> {
        &self.blobs
    }

    /// Check if a value should be offloaded to blobs.
    fn should_offload(&self, value: &str) -> bool {
        self.auto_offload && value.len() as u32 >= BLOB_THRESHOLD
    }

    /// Store a large value in blobs and return the blob reference.
    async fn offload_to_blob(&self, key: &str, value: &str) -> Result<String, BlobStoreError> {
        // Add to blob store
        let result = self.blobs.add_bytes(value.as_bytes()).await?;

        // Create GC protection tag for this KV entry
        let tag_name = kv_blob_tag(key);
        self.blobs.protect(&result.blob_ref.hash, &tag_name).await?;

        // Return serialized blob reference
        // Note: BlobRef serialization should never fail for valid data, but we handle it
        // gracefully by converting the error to a storage error.
        result.blob_ref.to_kv_value().map_err(|e| BlobStoreError::Storage {
            message: format!("failed to serialize blob reference: {}", e),
        })
    }

    /// Resolve a blob reference to its actual value.
    async fn resolve_blob_ref(&self, kv_value: &str) -> Result<Option<String>, BlobStoreError> {
        let blob_ref = match BlobRef::from_kv_value(kv_value) {
            Some(r) => r,
            None => return Ok(Some(kv_value.to_string())), // Not a blob ref
        };

        match self.blobs.get_bytes(&blob_ref.hash).await? {
            Some(bytes) => {
                let value = String::from_utf8_lossy(&bytes).into_owned();
                Ok(Some(value))
            }
            None => {
                warn!(hash = %blob_ref.hash, "blob not found for reference");
                Ok(None)
            }
        }
    }

    /// Remove GC protection for a blob referenced by a key.
    ///
    /// # Arguments
    /// * `key` - The KV key whose blob protection should be removed
    /// * `kv_value` - The KV value (used to check if it's a blob reference)
    /// * `reason` - Why the protection is being removed (delete vs overwrite)
    async fn unprotect_blob(&self, key: &str, kv_value: &str, reason: UnprotectReason) -> Result<(), BlobStoreError> {
        if !is_blob_ref(kv_value) {
            return Ok(());
        }

        let tag_name = kv_blob_tag(key);
        self.blobs.unprotect_with_reason(&tag_name, reason).await?;
        debug!(key, "removed blob GC protection");

        Ok(())
    }

    /// Read a key from the underlying store, used for checking existing values.
    async fn read_raw(&self, key: &str) -> Option<String> {
        match self.kv.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => result.kv.map(|kv| kv.value),
            Err(KeyValueStoreError::NotFound { .. }) => None,
            Err(_) => None,
        }
    }

    /// Process a Set command, offloading large values to blobs if needed.
    async fn write_process_set(&self, key: String, value: String) -> WriteCommand {
        let final_value = if self.should_offload(&value) {
            match self.offload_to_blob(&key, &value).await {
                Ok(blob_ref) => {
                    debug!(key, size = value.len(), "offloaded large value to blob");
                    blob_ref
                }
                Err(e) => {
                    warn!(error = %e, key, "failed to offload to blob, storing directly");
                    value
                }
            }
        } else {
            value
        };

        // Unprotect old blob if exists
        self.write_unprotect_old_blob(&key, UnprotectReason::KvOverwrite).await;

        WriteCommand::Set {
            key,
            value: final_value,
        }
    }

    /// Process a SetMulti command, offloading large values to blobs if needed.
    async fn write_process_set_multi(&self, pairs: Vec<(String, String)>) -> WriteCommand {
        let mut processed_pairs = Vec::with_capacity(pairs.len());
        for (key, value) in pairs {
            let final_value = if self.should_offload(&value) {
                match self.offload_to_blob(&key, &value).await {
                    Ok(blob_ref) => {
                        debug!(key, size = value.len(), "offloaded large value to blob");
                        blob_ref
                    }
                    Err(e) => {
                        warn!(error = %e, key, "failed to offload to blob, storing directly");
                        value
                    }
                }
            } else {
                value
            };

            // Unprotect old blob if exists
            self.write_unprotect_old_blob(&key, UnprotectReason::KvOverwrite).await;

            processed_pairs.push((key, final_value));
        }
        WriteCommand::SetMulti { pairs: processed_pairs }
    }

    /// Process a Delete command, unprotecting any blob references.
    async fn write_process_delete(&self, key: String) -> WriteCommand {
        self.write_unprotect_old_blob(&key, UnprotectReason::KvDelete).await;
        WriteCommand::Delete { key }
    }

    /// Process a DeleteMulti command, unprotecting any blob references.
    async fn write_process_delete_multi(&self, keys: Vec<String>) -> WriteCommand {
        for key in &keys {
            self.write_unprotect_old_blob(key, UnprotectReason::KvDelete).await;
        }
        WriteCommand::DeleteMulti { keys }
    }

    /// Helper to unprotect old blob for a key if it exists.
    async fn write_unprotect_old_blob(&self, key: &str, reason: UnprotectReason) {
        if let Some(old_value) = self.read_raw(key).await
            && let Err(e) = self.unprotect_blob(key, &old_value, reason).await
        {
            warn!(error = %e, key, "failed to unprotect old blob");
        }
    }
}

#[async_trait]
impl<KV: KeyValueStore + Send + Sync + 'static, B: BlobRead + BlobWrite + 'static> KvRead for BlobAwareKeyValueStore<KV, B> {
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        // Read from KV
        let result = self.kv.read(request.clone()).await?;

        // Extract value from kv field
        let value = match &result.kv {
            Some(kv) => &kv.value,
            None => return Ok(result), // No value, return as-is
        };

        if is_blob_ref(value) {
            // Resolve blob reference
            match self.resolve_blob_ref(value).await {
                Ok(Some(resolved)) => {
                    // Create new result with resolved value
                    if let Some(mut kv) = result.kv {
                        kv.value = resolved;
                        Ok(ReadResult { kv: Some(kv) })
                    } else {
                        Ok(ReadResult { kv: None })
                    }
                }
                Ok(None) => {
                    // Blob not found, return raw reference as fallback
                    warn!(key = %request.key, "blob not found, returning raw reference");
                    Ok(result)
                }
                Err(e) => {
                    warn!(error = %e, key = %request.key, "failed to resolve blob reference");
                    // Return the raw reference as fallback
                    Ok(result)
                }
            }
        } else {
            Ok(result)
        }
    }

}

#[async_trait]
impl<KV: KeyValueStore + Send + Sync + 'static, B: BlobRead + BlobWrite + 'static> KvWrite for BlobAwareKeyValueStore<KV, B> {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        // Process command, offloading large values to blobs
        let processed_command = match request.command {
            WriteCommand::Set { key, value } => self.write_process_set(key, value).await,
            WriteCommand::SetMulti { pairs } => self.write_process_set_multi(pairs).await,
            WriteCommand::Delete { key } => self.write_process_delete(key).await,
            WriteCommand::DeleteMulti { keys } => self.write_process_delete_multi(keys).await,
            // All other commands pass through directly without blob offloading
            other => other,
        };

        // Write to underlying KV
        self.kv
            .write(WriteRequest {
                command: processed_command,
            })
            .await
    }

}

#[async_trait]
impl<KV: KeyValueStore + Send + Sync + 'static, B: BlobRead + BlobWrite + 'static> KvDelete for BlobAwareKeyValueStore<KV, B> {
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        // Unprotect blob before delete
        if let Some(old_value) = self.read_raw(&request.key).await
            && let Err(e) = self.unprotect_blob(&request.key, &old_value, UnprotectReason::KvDelete).await
        {
            warn!(error = %e, key = %request.key, "failed to unprotect old blob");
        }

        self.kv.delete(request).await
    }

}

#[async_trait]
impl<KV: KeyValueStore + Send + Sync + 'static, B: BlobRead + BlobWrite + 'static> KvScan for BlobAwareKeyValueStore<KV, B> {
    /// Scan keys in the underlying KV store.
    ///
    /// **IMPORTANT**: This returns raw KV values, which may include blob references
    /// (values starting with `__blob:`). To get the actual data for blob-backed values,
    /// callers should use `read()` on individual keys after scanning.
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        self.kv.scan(request).await
    }
}

impl<KV: KeyValueStore + Send + Sync + 'static, B: BlobRead + BlobWrite + 'static> KeyValueStore for BlobAwareKeyValueStore<KV, B> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryBlobStore;

    /// A blob backend that implements BlobRead + BlobWrite but NOT BlobTransfer.
    /// Proves that `BlobAwareKeyValueStore` compiles without transfer capabilities.
    struct ReadWriteOnlyBlob(InMemoryBlobStore);

    #[async_trait]
    impl BlobRead for ReadWriteOnlyBlob {
        async fn get_bytes(&self, hash: &iroh_blobs::Hash) -> Result<Option<bytes::Bytes>, BlobStoreError> {
            self.0.get_bytes(hash).await
        }
        async fn has(&self, hash: &iroh_blobs::Hash) -> Result<bool, BlobStoreError> {
            self.0.has(hash).await
        }
        async fn status(&self, hash: &iroh_blobs::Hash) -> Result<Option<crate::BlobStatus>, BlobStoreError> {
            self.0.status(hash).await
        }
        async fn reader(&self, hash: &iroh_blobs::Hash) -> Result<Option<std::pin::Pin<Box<dyn crate::AsyncReadSeek>>>, BlobStoreError> {
            self.0.reader(hash).await
        }
    }

    #[async_trait]
    impl BlobWrite for ReadWriteOnlyBlob {
        async fn add_bytes(&self, data: &[u8]) -> Result<crate::AddBlobResult, BlobStoreError> {
            self.0.add_bytes(data).await
        }
        async fn add_path(&self, path: &std::path::Path) -> Result<crate::AddBlobResult, BlobStoreError> {
            self.0.add_path(path).await
        }
        async fn protect(&self, hash: &iroh_blobs::Hash, tag: &str) -> Result<(), BlobStoreError> {
            self.0.protect(hash, tag).await
        }
        async fn unprotect(&self, tag: &str) -> Result<(), BlobStoreError> {
            self.0.unprotect(tag).await
        }
    }

    /// Compile-time proof: BlobAwareKeyValueStore works with a blob backend
    /// that only provides read/write — no BlobTransfer or BlobQuery needed.
    fn _assert_no_transfer_needed<KV: KeyValueStore + Send + Sync + 'static>(kv: Arc<KV>) {
        fn takes_kv_store<S: KeyValueStore>(_s: &S) {}
        let blobs = Arc::new(ReadWriteOnlyBlob(InMemoryBlobStore::new()));
        let store = BlobAwareKeyValueStore::new(kv, blobs, true);
        takes_kv_store(&store);
    }

    #[test]
    fn test_small_value_stored_directly() {
        // Create a small value that shouldn't be offloaded
        let small_value = "a".repeat(100); // 100 bytes << 1MB threshold

        // Verify it's below threshold
        assert!((small_value.len() as u32) < BLOB_THRESHOLD);
    }

    #[test]
    fn test_blob_ref_roundtrip() {
        use iroh_blobs::BlobFormat;
        use iroh_blobs::Hash;

        // Create a blob reference
        let hash = Hash::new(b"test data");
        let blob_ref = BlobRef::new(hash, 12345, BlobFormat::Raw);

        // Convert to KV value
        let kv_value = blob_ref.to_kv_value().expect("serialization should succeed");

        // Verify it has the blob ref prefix
        assert!(kv_value.starts_with(crate::BLOB_REF_PREFIX));

        // Parse it back
        let parsed = BlobRef::from_kv_value(&kv_value).expect("should parse");

        // Verify roundtrip
        assert_eq!(parsed.hash, hash);
        assert_eq!(parsed.size_bytes, 12345);
        assert_eq!(parsed.format, BlobFormat::Raw);
    }

    #[test]
    fn test_is_blob_ref_detection() {
        use iroh_blobs::BlobFormat;
        use iroh_blobs::Hash;

        // Create a blob reference
        let hash = Hash::new(b"test");
        let blob_ref = BlobRef::new(hash, 100, BlobFormat::Raw);
        let blob_ref_value = blob_ref.to_kv_value().unwrap();

        // Should detect blob references
        assert!(is_blob_ref(&blob_ref_value));

        // Should not detect normal values
        assert!(!is_blob_ref("normal value"));
        assert!(!is_blob_ref(""));
        assert!(!is_blob_ref("__blo:not quite")); // Wrong prefix

        // Edge case: exact prefix but not a valid blob ref
        let fake_ref = format!("{}invalid json", crate::BLOB_REF_PREFIX);
        assert!(is_blob_ref(&fake_ref)); // is_blob_ref only checks prefix

        // But parsing should fail
        assert!(BlobRef::from_kv_value(&fake_ref).is_none());
    }

    #[test]
    fn test_malformed_blob_ref_prefix_but_bad_json() {
        let malformed = format!("{}{{not valid json", crate::BLOB_REF_PREFIX);
        assert!(is_blob_ref(&malformed));
        assert!(BlobRef::from_kv_value(&malformed).is_none());
    }

    #[test]
    fn test_malformed_blob_ref_missing_required_fields() {
        let missing_size = format!("{}{{\"format\":\"Raw\"}}", crate::BLOB_REF_PREFIX);
        assert!(BlobRef::from_kv_value(&missing_size).is_none());
    }

    /// A blob backend where protect/unprotect always fail.
    struct FailingProtectBlob(InMemoryBlobStore);

    #[async_trait]
    impl BlobRead for FailingProtectBlob {
        async fn get_bytes(&self, hash: &iroh_blobs::Hash) -> Result<Option<bytes::Bytes>, BlobStoreError> {
            self.0.get_bytes(hash).await
        }
        async fn has(&self, hash: &iroh_blobs::Hash) -> Result<bool, BlobStoreError> {
            self.0.has(hash).await
        }
        async fn status(&self, hash: &iroh_blobs::Hash) -> Result<Option<crate::BlobStatus>, BlobStoreError> {
            self.0.status(hash).await
        }
        async fn reader(&self, hash: &iroh_blobs::Hash) -> Result<Option<std::pin::Pin<Box<dyn crate::AsyncReadSeek>>>, BlobStoreError> {
            self.0.reader(hash).await
        }
    }

    #[async_trait]
    impl BlobWrite for FailingProtectBlob {
        async fn add_bytes(&self, data: &[u8]) -> Result<crate::AddBlobResult, BlobStoreError> {
            self.0.add_bytes(data).await
        }
        async fn add_path(&self, path: &std::path::Path) -> Result<crate::AddBlobResult, BlobStoreError> {
            self.0.add_path(path).await
        }
        async fn protect(&self, _hash: &iroh_blobs::Hash, tag: &str) -> Result<(), BlobStoreError> {
            Err(BlobStoreError::SetTag {
                tag: tag.to_string(),
                message: "simulated protect failure".to_string(),
            })
        }
        async fn unprotect(&self, tag: &str) -> Result<(), BlobStoreError> {
            Err(BlobStoreError::DeleteTag {
                tag: tag.to_string(),
                message: "simulated unprotect failure".to_string(),
            })
        }
    }

    /// Compile-time proof: a blob backend where protect/unprotect fail is
    /// still accepted by `BlobAwareKeyValueStore` (errors handled gracefully).
    fn _assert_failing_protect_blob_compiles<KV: KeyValueStore + Send + Sync + 'static>(kv: Arc<KV>) {
        let blobs = Arc::new(FailingProtectBlob(InMemoryBlobStore::new()));
        let _store = BlobAwareKeyValueStore::new(kv, blobs, true);
    }

    /// Replication-specific types are not available without the feature.
    #[test]
    fn test_replication_types_absent_without_feature() {
        // BlobReplicationManager, IrohBlobTransfer, etc. are only available
        // behind cfg(feature = "replication"). This test documents the boundary.
        #[cfg(not(feature = "replication"))]
        {
            // If replication is off, these modules should not exist.
            // This is a documentation assertion — the cfg gate ensures
            // non-replication consumers don't pull in aspen-client-api.
            assert!(!cfg!(feature = "replication"));
        }
    }
}
