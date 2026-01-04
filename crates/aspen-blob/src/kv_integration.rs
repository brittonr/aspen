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
use crate::BlobRef;
use crate::BlobStore;
use crate::BlobStoreError;
use crate::IrohBlobStore;
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
        Ok(result.blob_ref.to_kv_value())
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
    async fn unprotect_blob(&self, key: &str, kv_value: &str) -> Result<(), BlobStoreError> {
        if !is_blob_ref(kv_value) {
            return Ok(());
        }

        let tag_name = IrohBlobStore::kv_tag(key);
        self.blobs.unprotect(&tag_name).await?;
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
                    let mut kv = result.kv.unwrap(); // Safe: we checked above
                    kv.value = resolved;
                    Ok(ReadResult { kv: Some(kv) })
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
            WriteCommand::Set { key, value } => {
                // Check if we should offload this value
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

                // First, read existing value to check if we need to unprotect old blob
                if let Some(old_value) = self.read_raw(&key).await
                    && let Err(e) = self.unprotect_blob(&key, &old_value).await
                {
                    warn!(error = %e, key, "failed to unprotect old blob");
                }

                WriteCommand::Set {
                    key,
                    value: final_value,
                }
            }
            WriteCommand::SetMulti { pairs } => {
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
                    if let Some(old_value) = self.read_raw(&key).await
                        && let Err(e) = self.unprotect_blob(&key, &old_value).await
                    {
                        warn!(error = %e, key, "failed to unprotect old blob");
                    }

                    processed_pairs.push((key, final_value));
                }
                WriteCommand::SetMulti { pairs: processed_pairs }
            }
            WriteCommand::Delete { key } => {
                // Unprotect blob before delete
                if let Some(old_value) = self.read_raw(&key).await
                    && let Err(e) = self.unprotect_blob(&key, &old_value).await
                {
                    warn!(error = %e, key, "failed to unprotect old blob");
                }
                WriteCommand::Delete { key }
            }
            WriteCommand::DeleteMulti { keys } => {
                // Unprotect blobs before delete
                for key in &keys {
                    if let Some(old_value) = self.read_raw(key).await
                        && let Err(e) = self.unprotect_blob(key, &old_value).await
                    {
                        warn!(error = %e, key, "failed to unprotect old blob");
                    }
                }
                WriteCommand::DeleteMulti { keys }
            }
            // CAS operations pass through directly without blob offloading.
            // CAS is typically used for small values (locks, counters) that don't need offloading.
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            },
            WriteCommand::CompareAndDelete { key, expected } => WriteCommand::CompareAndDelete { key, expected },
            // Batch operations pass through directly without blob offloading.
            // Batches are typically used for transactional operations that don't need offloading.
            WriteCommand::Batch { operations } => WriteCommand::Batch { operations },
            WriteCommand::ConditionalBatch { conditions, operations } => {
                WriteCommand::ConditionalBatch { conditions, operations }
            }
            // TTL operations pass through - no blob offloading for TTL values
            WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            } => WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            },
            WriteCommand::SetMultiWithTTL { pairs, ttl_seconds } => {
                WriteCommand::SetMultiWithTTL { pairs, ttl_seconds }
            }
            // Lease operations pass through - no blob offloading
            WriteCommand::SetWithLease { key, value, lease_id } => WriteCommand::SetWithLease { key, value, lease_id },
            WriteCommand::SetMultiWithLease { pairs, lease_id } => WriteCommand::SetMultiWithLease { pairs, lease_id },
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => WriteCommand::LeaseGrant { lease_id, ttl_seconds },
            WriteCommand::LeaseRevoke { lease_id } => WriteCommand::LeaseRevoke { lease_id },
            WriteCommand::LeaseKeepalive { lease_id } => WriteCommand::LeaseKeepalive { lease_id },
            // Transaction operations pass through directly (blob processing not supported in transactions)
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => WriteCommand::Transaction {
                compare,
                success,
                failure,
            },
            // OptimisticTransaction operations pass through directly
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                WriteCommand::OptimisticTransaction { read_set, write_set }
            }
            // Shard topology operations pass through directly (control plane operations)
            WriteCommand::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            } => WriteCommand::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            },
            WriteCommand::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            } => WriteCommand::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            },
            WriteCommand::TopologyUpdate { topology_data } => WriteCommand::TopologyUpdate { topology_data },
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
            && let Err(e) = self.unprotect_blob(&request.key, &old_value).await
        {
            warn!(error = %e, key = %request.key, "failed to unprotect old blob");
        }

        self.kv.delete(request).await
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        // Scan returns raw values - caller should use read() to resolve blobs
        // This is intentional to avoid loading all blob data during scans
        self.kv.scan(request).await
    }
}

#[cfg(test)]
mod tests {
    // Integration tests in tests/ directory
    // Unit tests would require mocking both KV and blob store
}
