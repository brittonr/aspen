//! Trait adapters for the blob replication system.
//!
//! This module provides concrete implementations of the replication traits
//! (`ReplicaMetadataStore` and `ReplicaBlobTransfer`) that bridge the abstract
//! replication manager with Aspen's KV store and IrohBlobStore.
//!
//! ## Architecture
//!
//! ```text
//! BlobReplicationManager
//!     |
//!     +-> KvReplicaMetadataStore (implements ReplicaMetadataStore)
//!     |   - Reads/writes ReplicaSet to Raft KV under _system:blob:replica: prefix
//!     |   - Provides scan_by_status for repair operations
//!     |
//!     +-> IrohBlobTransfer (implements ReplicaBlobTransfer)
//!         - Transfers blobs via iroh-blobs P2P protocol
//!         - Sends BlobReplicatePull RPC to target nodes
//!         - Target nodes download from source using download_from_peer()
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use aspen_blob::replication::adapters::{KvReplicaMetadataStore, IrohBlobTransfer};
//!
//! let metadata_store = KvReplicaMetadataStore::new(kv_store.clone());
//! let blob_transfer = IrohBlobTransfer::new(blob_store.clone(), endpoint.clone());
//!
//! let (manager, task) = BlobReplicationManager::spawn(
//!     config,
//!     blob_events,
//!     Arc::new(metadata_store),
//!     Arc::new(blob_transfer),
//!     placement,
//!     cancel,
//! ).await?;
//! ```

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::endpoint::VarInt;
use iroh_blobs::Hash;
use tokio::time::timeout;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::replication::MAX_REPAIR_BATCH_SIZE;
use crate::replication::REPLICA_KEY_PREFIX;
use crate::replication::ReplicaSet;
use crate::replication::ReplicationStatus;
use crate::replication::manager::NodeInfo;
use crate::replication::manager::ReplicaBlobTransfer;
use crate::replication::manager::ReplicaMetadataStore;
use crate::store::BlobStore;
use crate::store::IrohBlobStore;

// ============================================================================
// KV-backed Replica Metadata Store
// ============================================================================

/// Adapter that implements `ReplicaMetadataStore` using Aspen's KeyValueStore.
///
/// Stores replica metadata in the Raft-backed KV store under the
/// `_system:blob:replica:` key prefix, ensuring cluster-wide linearizable
/// tracking of replica locations.
pub struct KvReplicaMetadataStore<KV> {
    kv_store: Arc<KV>,
}

impl<KV> KvReplicaMetadataStore<KV> {
    /// Create a new metadata store backed by the given KeyValueStore.
    pub fn new(kv_store: Arc<KV>) -> Self {
        Self { kv_store }
    }
}

#[async_trait]
impl<KV> ReplicaMetadataStore for KvReplicaMetadataStore<KV>
where
    KV: aspen_core::traits::KeyValueStore + Send + Sync + 'static,
{
    async fn get_replica_set(&self, hash: &Hash) -> Result<Option<ReplicaSet>> {
        let key = format!("{}{}", REPLICA_KEY_PREFIX, hash.to_hex());

        let result = self
            .kv_store
            .read(aspen_core::kv::ReadRequest::new(&key))
            .await
            .context("failed to read replica set from KV")?;

        match result.kv {
            Some(kv) => {
                let replicas = ReplicaSet::from_json(&kv.value).context("failed to deserialize replica set")?;
                Ok(Some(replicas))
            }
            None => Ok(None),
        }
    }

    async fn save_replica_set(&self, replicas: &ReplicaSet) -> Result<()> {
        let key = replicas.kv_key();
        let value = replicas.to_json().context("failed to serialize replica set")?;

        self.kv_store
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
            .context("failed to write replica set to KV")?;

        debug!(hash = %replicas.hash.to_hex(), nodes = replicas.nodes.len(), "replica set saved");
        Ok(())
    }

    async fn delete_replica_set(&self, hash: &Hash) -> Result<()> {
        let key = format!("{}{}", REPLICA_KEY_PREFIX, hash.to_hex());

        self.kv_store
            .delete(aspen_core::kv::DeleteRequest { key })
            .await
            .context("failed to delete replica set from KV")?;

        debug!(hash = %hash.to_hex(), "replica set deleted");
        Ok(())
    }

    async fn scan_by_status(&self, status: ReplicationStatus) -> Result<Vec<Hash>> {
        // Scan all replica metadata with the system prefix
        let scan_result = self
            .kv_store
            .scan(aspen_core::kv::ScanRequest {
                prefix: REPLICA_KEY_PREFIX.to_string(),
                limit: Some(MAX_REPAIR_BATCH_SIZE * 10), // Scan extra to find enough matching status
                continuation_token: None,
            })
            .await
            .context("failed to scan replica metadata")?;

        let mut matching_hashes = Vec::new();

        for entry in scan_result.entries {
            // Parse the replica set from the value
            match ReplicaSet::from_json(&entry.value) {
                Ok(replicas) => {
                    if replicas.status() == status {
                        matching_hashes.push(replicas.hash);

                        // Limit results to prevent unbounded work
                        if matching_hashes.len() >= MAX_REPAIR_BATCH_SIZE as usize {
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!(key = entry.key, error = %e, "failed to parse replica set, skipping");
                }
            }
        }

        debug!(status = ?status, count = matching_hashes.len(), "scanned replicas by status");
        Ok(matching_hashes)
    }
}

// ============================================================================
// IrohBlobStore-backed Blob Transfer
// ============================================================================

/// ALPN protocol identifier for Aspen client RPC.
const CLIENT_ALPN: &[u8] = b"aspen/client/1";

/// Maximum message size for RPC responses.
const MAX_RPC_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16 MB

/// Timeout for RPC operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Adapter that implements `ReplicaBlobTransfer` using IrohBlobStore.
///
/// Uses iroh-blobs P2P protocol to transfer blobs between cluster nodes.
/// The source node sends a BlobReplicatePull RPC to the target node,
/// which then downloads the blob using iroh-blobs download_from_peer().
pub struct IrohBlobTransfer {
    blob_store: Arc<IrohBlobStore>,
    endpoint: Endpoint,
}

impl IrohBlobTransfer {
    /// Create a new blob transfer adapter.
    ///
    /// # Arguments
    /// * `blob_store` - The IrohBlobStore for local blob operations
    /// * `endpoint` - The Iroh endpoint for RPC communication
    pub fn new(blob_store: Arc<IrohBlobStore>, endpoint: Endpoint) -> Self {
        Self { blob_store, endpoint }
    }
}

#[async_trait]
impl ReplicaBlobTransfer for IrohBlobTransfer {
    async fn transfer_to_node(&self, hash: &Hash, target: &NodeInfo) -> Result<bool> {
        let start = Instant::now();

        // First verify we have the blob locally
        if !self.blob_store.has(hash).await.map_err(|e| anyhow::anyhow!("{}", e))? {
            return Err(anyhow::anyhow!("cannot transfer blob {}: not available locally", hash.to_hex()));
        }

        // Get blob size for the request
        let size = self
            .blob_store
            .status(hash)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .and_then(|s| s.size)
            .unwrap_or(0);

        // Get our public key as the provider (use secret_key().public() for iroh 0.95)
        let our_key = self.endpoint.secret_key().public();

        debug!(
            hash = %hash.to_hex(),
            target_node = target.node_id,
            target_key = %target.public_key.fmt_short(),
            provider = %our_key.fmt_short(),
            size,
            "sending BlobReplicatePull request to target"
        );

        // Build the RPC request
        // We need to use the client API types - import them via the messages module
        let request = aspen_client_api::ClientRpcRequest::BlobReplicatePull {
            hash: hash.to_hex(),
            size,
            provider: our_key.to_string(),
            tag: Some(format!("_replica:{}", hash.to_hex())),
        };

        // Connect to the target node
        let target_addr = EndpointAddr::new(target.public_key);
        let connection = timeout(RPC_TIMEOUT, async {
            self.endpoint.connect(target_addr, CLIENT_ALPN).await.context("failed to connect to target node")
        })
        .await
        .context("connection timeout")??;

        // Open bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Serialize request
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes).await.context("failed to send request")?;
        send.finish().context("failed to finish send stream")?;

        // Read response with timeout
        let response_bytes = timeout(RPC_TIMEOUT, async {
            recv.read_to_end(MAX_RPC_MESSAGE_SIZE).await.context("failed to read response")
        })
        .await
        .context("response timeout")??;

        // Deserialize response
        let response: aspen_client_api::ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Close connection gracefully
        connection.close(VarInt::from_u32(0), b"done");

        let duration_ms = start.elapsed().as_millis() as u64;

        // Check the response
        match response {
            aspen_client_api::ClientRpcResponse::BlobReplicatePullResult(result) => {
                if result.success {
                    info!(
                        hash = %hash.to_hex(),
                        target_node = target.node_id,
                        duration_ms,
                        "blob replication succeeded"
                    );
                    Ok(true)
                } else {
                    let error = result.error.unwrap_or_else(|| "unknown error".to_string());
                    warn!(
                        hash = %hash.to_hex(),
                        target_node = target.node_id,
                        duration_ms,
                        error = %error,
                        "blob replication failed"
                    );
                    Err(anyhow::anyhow!("replication failed: {}", error))
                }
            }
            other => {
                warn!(
                    hash = %hash.to_hex(),
                    target_node = target.node_id,
                    response = ?other,
                    "unexpected response type"
                );
                Err(anyhow::anyhow!("unexpected response type: {:?}", other))
            }
        }
    }

    async fn has_locally(&self, hash: &Hash) -> Result<bool> {
        self.blob_store
            .has(hash)
            .await
            .map_err(|e| anyhow::anyhow!("failed to check blob availability: {}", e))
    }

    async fn get_size(&self, hash: &Hash) -> Result<Option<u64>> {
        match self.blob_store.status(hash).await {
            Ok(Some(status)) => Ok(status.size),
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("failed to get blob status: {}", e)),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::replication::ReplicationPolicy;

    /// Mock KeyValueStore for testing the metadata adapter.
    #[derive(Default)]
    struct MockKvStore {
        data: std::sync::RwLock<std::collections::HashMap<String, String>>,
        revision: AtomicU64,
    }

    #[async_trait]
    impl aspen_core::traits::KeyValueStore for MockKvStore {
        async fn write(
            &self,
            request: aspen_core::kv::WriteRequest,
        ) -> Result<aspen_core::kv::WriteResult, aspen_core::error::KeyValueStoreError> {
            match request.command {
                aspen_core::kv::WriteCommand::Set { key, value } => {
                    self.data.write().unwrap().insert(key, value);
                    self.revision.fetch_add(1, Ordering::SeqCst);
                    Ok(aspen_core::kv::WriteResult::default())
                }
                _ => Ok(aspen_core::kv::WriteResult::default()),
            }
        }

        async fn read(
            &self,
            request: aspen_core::kv::ReadRequest,
        ) -> Result<aspen_core::kv::ReadResult, aspen_core::error::KeyValueStoreError> {
            let data = self.data.read().unwrap();
            let kv = data.get(&request.key).map(|v| aspen_core::kv::KeyValueWithRevision {
                key: request.key.clone(),
                value: v.clone(),
                version: 1,
                create_revision: 1,
                mod_revision: self.revision.load(Ordering::SeqCst),
            });
            Ok(aspen_core::kv::ReadResult { kv })
        }

        async fn delete(
            &self,
            request: aspen_core::kv::DeleteRequest,
        ) -> Result<aspen_core::kv::DeleteResult, aspen_core::error::KeyValueStoreError> {
            let deleted = self.data.write().unwrap().remove(&request.key).is_some();
            Ok(aspen_core::kv::DeleteResult {
                key: request.key,
                deleted,
            })
        }

        async fn scan(
            &self,
            request: aspen_core::kv::ScanRequest,
        ) -> Result<aspen_core::kv::ScanResult, aspen_core::error::KeyValueStoreError> {
            let data = self.data.read().unwrap();
            let entries: Vec<_> = data
                .iter()
                .filter(|(k, _)| k.starts_with(&request.prefix))
                .take(request.limit.unwrap_or(100) as usize)
                .map(|(k, v)| aspen_core::kv::KeyValueWithRevision {
                    key: k.clone(),
                    value: v.clone(),
                    version: 1,
                    create_revision: 1,
                    mod_revision: 1,
                })
                .collect();
            let count = entries.len() as u32;
            Ok(aspen_core::kv::ScanResult {
                entries,
                count,
                is_truncated: false,
                continuation_token: None,
            })
        }
    }

    #[tokio::test]
    async fn test_kv_metadata_store_roundtrip() {
        let kv = Arc::new(MockKvStore::default());
        let store = KvReplicaMetadataStore::new(kv);

        let hash = Hash::from_bytes([0x42; 32]);
        let policy = ReplicationPolicy::with_factor(3);
        let replicas = ReplicaSet::new(hash, 1024, policy, 1);

        // Save
        store.save_replica_set(&replicas).await.unwrap();

        // Read back
        let loaded = store.get_replica_set(&hash).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.hash, hash);
        assert_eq!(loaded.size, 1024);
        assert!(loaded.nodes.contains(&1));

        // Delete
        store.delete_replica_set(&hash).await.unwrap();
        let deleted = store.get_replica_set(&hash).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_kv_metadata_store_scan_by_status() {
        let kv = Arc::new(MockKvStore::default());
        let store = KvReplicaMetadataStore::new(kv);

        // Create replicas with different statuses
        let hash1 = Hash::from_bytes([0x01; 32]);
        let hash2 = Hash::from_bytes([0x02; 32]);
        let hash3 = Hash::from_bytes([0x03; 32]);

        // Under-replicated (1 node, needs 2)
        let policy1 = ReplicationPolicy {
            replication_factor: 3,
            min_replicas: 2,
            ..Default::default()
        };
        let replicas1 = ReplicaSet::new(hash1, 100, policy1, 1);
        store.save_replica_set(&replicas1).await.unwrap();

        // Healthy (3 nodes, needs 3)
        let policy2 = ReplicationPolicy::with_factor(3);
        let mut replicas2 = ReplicaSet::new(hash2, 200, policy2, 1);
        replicas2.add_node(2);
        replicas2.add_node(3);
        store.save_replica_set(&replicas2).await.unwrap();

        // Degraded (2 nodes, min 2, target 3)
        let policy3 = ReplicationPolicy {
            replication_factor: 3,
            min_replicas: 2,
            ..Default::default()
        };
        let mut replicas3 = ReplicaSet::new(hash3, 300, policy3, 1);
        replicas3.add_node(2);
        store.save_replica_set(&replicas3).await.unwrap();

        // Scan for under-replicated
        let under = store.scan_by_status(ReplicationStatus::UnderReplicated).await.unwrap();
        assert_eq!(under.len(), 1);
        assert!(under.contains(&hash1));

        // Scan for healthy
        let healthy = store.scan_by_status(ReplicationStatus::Healthy).await.unwrap();
        assert_eq!(healthy.len(), 1);
        assert!(healthy.contains(&hash2));

        // Scan for degraded
        let degraded = store.scan_by_status(ReplicationStatus::Degraded).await.unwrap();
        assert_eq!(degraded.len(), 1);
        assert!(degraded.contains(&hash3));
    }

    // Note: IrohBlobTransfer tests require a full Iroh endpoint and IrohBlobStore.
    // Integration tests should be written in the integration test suite with actual
    // Iroh endpoints for proper P2P blob transfer testing.
}
