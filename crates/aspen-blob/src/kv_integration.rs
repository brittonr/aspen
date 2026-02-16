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
use aspen_core::DeleteRequest;
use aspen_core::DeleteResult;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::ReadResult;
use aspen_core::ScanRequest;
use aspen_core::ScanResult;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::WriteResult;
use async_trait::async_trait;
use tracing::debug;
use tracing::warn;

use crate::BLOB_THRESHOLD;
use crate::BlobRead;
use crate::BlobRef;
use crate::BlobStoreError;
use crate::BlobWrite;
use crate::IrohBlobStore;
use crate::UnprotectReason;
use crate::is_blob_ref;

/// Wrapper around KeyValueStore that offloads large values to blob storage.
///
/// Values larger than `BLOB_THRESHOLD` (1MB) are automatically stored in
/// iroh-blobs with a `BlobRef` saved in the KV store.
pub struct BlobAwareKeyValueStore<KV: KeyValueStore> {
    /// Underlying KV store for small values and blob references.
    kv: Arc<KV>,
    /// Blob store for large values.
    blobs: Arc<IrohBlobStore>,
    /// Whether automatic blob offloading is enabled.
    auto_offload: bool,
}

impl<KV: KeyValueStore> BlobAwareKeyValueStore<KV> {
    /// Create a new blob-aware KV store wrapper.
    ///
    /// # Arguments
    /// * `kv` - The underlying KeyValueStore implementation
    /// * `blobs` - The IrohBlobStore for large value storage
    /// * `auto_offload` - Whether to automatically offload large values
    pub fn new(kv: Arc<KV>, blobs: Arc<IrohBlobStore>, auto_offload: bool) -> Self {
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
    pub fn blobs(&self) -> &Arc<IrohBlobStore> {
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
        let tag_name = IrohBlobStore::kv_tag(key);
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

        let tag_name = IrohBlobStore::kv_tag(key);
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
impl<KV: KeyValueStore + Send + Sync + 'static> KeyValueStore for BlobAwareKeyValueStore<KV> {
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

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        // Unprotect blob before delete
        if let Some(old_value) = self.read_raw(&request.key).await
            && let Err(e) = self.unprotect_blob(&request.key, &old_value, UnprotectReason::KvDelete).await
        {
            warn!(error = %e, key = %request.key, "failed to unprotect old blob");
        }

        self.kv.delete(request).await
    }

    /// Scan keys in the underlying KV store.
    ///
    /// **IMPORTANT**: This returns raw KV values, which may include blob references
    /// (values starting with `__blob:`). To get the actual data for blob-backed values,
    /// callers should use `read()` on individual keys after scanning.
    ///
    /// This behavior is intentional to avoid loading potentially large blob data
    /// during range scans. A scan over thousands of keys should remain fast even
    /// if some values are multi-megabyte blobs.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Scan returns raw values (may include __blob: references)
    /// let scan_result = store.scan(request).await?;
    ///
    /// // To get actual blob data, read individual keys
    /// for entry in scan_result.entries {
    ///     if is_blob_ref(&entry.value) {
    ///         let full_value = store.read(ReadRequest::new(entry.key)).await?;
    ///         // full_value.kv.value contains the resolved blob data
    ///     }
    /// }
    /// ```
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        self.kv.scan(request).await
    }
}

#[cfg(test)]
mod tests {
    // Integration tests in tests/ directory
    // Unit tests would require mocking both KV and blob store
}
