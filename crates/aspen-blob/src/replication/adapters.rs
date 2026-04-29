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
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
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
use crate::store::IrohBlobStore;
use crate::traits::BlobRead;

// ============================================================================
// KV-backed Replica Metadata Store
// ============================================================================

/// Scan extra rows so status filtering can still return a full repair batch.
const REPLICA_STATUS_SCAN_OVERSAMPLE: u32 = 10;

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
where KV: KeyValueStore + Send + Sync + 'static
{
    async fn get_replica_set(&self, hash: &Hash) -> Result<Option<ReplicaSet>> {
        let key = format!("{}{}", REPLICA_KEY_PREFIX, hash.to_hex());

        let result = self.kv_store.read(ReadRequest::new(&key)).await.context("failed to read replica set from KV")?;

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
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .context("failed to write replica set to KV")?;

        debug!(hash = %replicas.hash.to_hex(), nodes = replicas.nodes.len(), "replica set saved");
        Ok(())
    }

    async fn delete_replica_set(&self, hash: &Hash) -> Result<()> {
        let key = format!("{}{}", REPLICA_KEY_PREFIX, hash.to_hex());

        self.kv_store.delete(DeleteRequest { key }).await.context("failed to delete replica set from KV")?;

        debug!(hash = %hash.to_hex(), "replica set deleted");
        Ok(())
    }

    async fn scan_by_status(&self, status: ReplicationStatus) -> Result<Vec<Hash>> {
        // Scan all replica metadata with the system prefix
        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: REPLICA_KEY_PREFIX.to_string(),
                limit_results: Some(MAX_REPAIR_BATCH_SIZE * REPLICA_STATUS_SCAN_OVERSAMPLE),
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

        // Verify local availability and get size
        let size = self.transfer_verify_local_blob(hash).await?;

        // Get our public key as the provider
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
        let request = aspen_client_api::ClientRpcRequest::BlobReplicatePull {
            hash: hash.to_hex(),
            size_bytes: size,
            provider: our_key.to_string(),
            tag: Some(format!("_replica:{}", hash.to_hex())),
        };

        // Send RPC and get response
        let response = self.transfer_send_rpc(target, &request).await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Process the response
        self.transfer_handle_response(response, hash, target, duration_ms)
    }

    async fn has_locally(&self, hash: &Hash) -> Result<bool> {
        self.blob_store.has(hash).await.context("failed to check blob availability")
    }

    async fn get_size(&self, hash: &Hash) -> Result<Option<u64>> {
        let status = self.blob_store.status(hash).await.context("failed to get blob status")?;
        Ok(status.and_then(|s| s.size_bytes))
    }
}

impl IrohBlobTransfer {
    /// Verify blob exists locally and return its size.
    async fn transfer_verify_local_blob(&self, hash: &Hash) -> Result<u64> {
        if !self.blob_store.has(hash).await.context("failed to check local blob availability")? {
            return Err(anyhow::anyhow!("cannot transfer blob {}: not available locally", hash.to_hex()));
        }

        let size = self
            .blob_store
            .status(hash)
            .await
            .context("failed to get blob status for transfer")?
            .and_then(|s| s.size_bytes)
            .unwrap_or(0);

        Ok(size)
    }

    /// Connect to target node and send RPC request.
    async fn transfer_send_rpc(
        &self,
        target: &NodeInfo,
        request: &aspen_client_api::ClientRpcRequest,
    ) -> Result<aspen_client_api::ClientRpcResponse> {
        // Connect to the target node
        let target_addr = EndpointAddr::new(target.public_key);
        let connection = timeout(RPC_TIMEOUT, async {
            self.endpoint.connect(target_addr, CLIENT_ALPN).await.context("failed to connect to target node")
        })
        .await
        .context("connection timeout")??;

        // Serialize request before entering the timed exchange
        let request_bytes = postcard::to_stdvec(request).context("failed to serialize request")?;

        // Bound the full post-connect exchange with one overall deadline while
        // preserving stage-specific timeout errors.
        let deadline = std::time::Instant::now() + RPC_TIMEOUT;

        // Open bidirectional stream
        let (mut send, mut recv) = tokio::time::timeout(remaining_timeout_blob(deadline)?, connection.open_bi())
            .await
            .context("stream open timeout")?
            .context("failed to open stream")?;

        // Send request
        tokio::time::timeout(remaining_timeout_blob(deadline)?, send.write_all(&request_bytes))
            .await
            .context("request write timeout")?
            .context("failed to send request")?;
        tokio::time::timeout(remaining_timeout_blob(deadline)?, async {
            send.finish().context("failed to finish send stream")
        })
        .await
        .context("stream finish timeout")??;

        // Read response with deadline
        let response_bytes =
            tokio::time::timeout(remaining_timeout_blob(deadline)?, recv.read_to_end(MAX_RPC_MESSAGE_SIZE))
                .await
                .context("response timeout")?
                .context("failed to read response")?;

        // Deserialize response
        let response: aspen_client_api::ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Close connection gracefully
        connection.close(VarInt::from_u32(0), b"done");

        Ok(response)
    }

    /// Handle the RPC response and return success/failure.
    fn transfer_handle_response(
        &self,
        response: aspen_client_api::ClientRpcResponse,
        hash: &Hash,
        target: &NodeInfo,
        duration_ms: u64,
    ) -> Result<bool> {
        match response {
            aspen_client_api::ClientRpcResponse::BlobReplicatePullResult(result) => {
                if result.is_success {
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
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use aspen_kv_types::DeleteResult;
    use aspen_kv_types::KeyValueStoreError;
    use aspen_kv_types::KeyValueWithRevision;
    use aspen_kv_types::ReadResult;
    use aspen_kv_types::ScanResult;
    use aspen_kv_types::WriteResult;
    use aspen_traits::KvDelete;
    use aspen_traits::KvRead;
    use aspen_traits::KvScan;
    use aspen_traits::KvWrite;

    use super::*;
    use crate::replication::ReplicationPolicy;

    /// Mock KeyValueStore for testing the metadata adapter.
    #[derive(Default)]
    struct MockKvStore {
        data: std::sync::RwLock<std::collections::HashMap<String, String>>,
        revision: AtomicU64,
    }

    const DEFAULT_SCAN_LIMIT: usize = 100;
    const INITIAL_REVISION: u64 = 1;
    const IROH_HASH_SIZE_BYTES: usize = 32;
    const MISSING_HASH_BYTE: u8 = 0x99;
    const MALFORMED_HASH_BYTE: u8 = 0x55;

    #[async_trait]
    impl KvWrite for MockKvStore {
        async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
            match request.command {
                WriteCommand::Set { key, value } => {
                    self.data.write().unwrap().insert(key, value);
                    self.revision.fetch_add(INITIAL_REVISION, Ordering::SeqCst);
                    Ok(WriteResult::default())
                }
                _ => Ok(WriteResult::default()),
            }
        }
    }

    #[async_trait]
    impl KvRead for MockKvStore {
        async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
            let data = self.data.read().unwrap();
            let kv = data.get(&request.key).map(|v| KeyValueWithRevision {
                key: request.key.clone(),
                value: v.clone(),
                version: INITIAL_REVISION,
                create_revision: INITIAL_REVISION,
                mod_revision: self.revision.load(Ordering::SeqCst),
            });
            Ok(ReadResult { kv })
        }
    }

    #[async_trait]
    impl KvDelete for MockKvStore {
        async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
            let is_deleted = self.data.write().unwrap().remove(&request.key).is_some();
            Ok(DeleteResult {
                key: request.key,
                is_deleted,
            })
        }
    }

    #[async_trait]
    impl KvScan for MockKvStore {
        async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            let limit_results = request.limit_results.map_or(DEFAULT_SCAN_LIMIT, |limit| limit as usize);
            let data = self.data.read().unwrap();
            let entries: Vec<_> = data
                .iter()
                .filter(|(k, _)| k.starts_with(&request.prefix))
                .take(limit_results)
                .map(|(k, v)| KeyValueWithRevision {
                    key: k.clone(),
                    value: v.clone(),
                    version: INITIAL_REVISION,
                    create_revision: INITIAL_REVISION,
                    mod_revision: INITIAL_REVISION,
                })
                .collect();
            let result_count = entries.len() as u32;
            Ok(ScanResult {
                entries,
                result_count,
                is_truncated: false,
                continuation_token: None,
            })
        }
    }

    impl KeyValueStore for MockKvStore {}

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
        assert_eq!(loaded.size_bytes, 1024);
        assert!(loaded.nodes.contains(&1));

        // Delete
        store.delete_replica_set(&hash).await.unwrap();
        let deleted = store.get_replica_set(&hash).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_kv_metadata_store_missing_returns_none() {
        let kv = Arc::new(MockKvStore::default());
        let store = KvReplicaMetadataStore::new(kv);

        let hash = Hash::from_bytes([MISSING_HASH_BYTE; IROH_HASH_SIZE_BYTES]);
        let loaded = store.get_replica_set(&hash).await.unwrap();

        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_kv_metadata_store_malformed_metadata_returns_error() {
        let kv = Arc::new(MockKvStore::default());
        let store = KvReplicaMetadataStore::new(kv.clone());

        let hash = Hash::from_bytes([MALFORMED_HASH_BYTE; IROH_HASH_SIZE_BYTES]);
        let key = format!("{}{}", REPLICA_KEY_PREFIX, hash.to_hex());
        kv.data.write().unwrap().insert(key, "not-json".to_string());

        let err = store.get_replica_set(&hash).await.expect_err("malformed metadata must fail");

        assert!(err.to_string().contains("failed to deserialize replica set"));
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

/// Compute remaining time until a deadline, returning an error if already past.
fn remaining_timeout_blob(deadline: std::time::Instant) -> anyhow::Result<std::time::Duration> {
    match deadline.checked_duration_since(std::time::Instant::now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(anyhow::anyhow!("deadline exceeded")),
    }
}

#[cfg(test)]
mod remaining_timeout_tests {
    use super::*;

    #[test]
    fn remaining_timeout_blob_returns_duration_when_deadline_is_ahead() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let remaining = remaining_timeout_blob(deadline).expect("should return Ok");
        assert!(remaining.as_secs() > 0);
    }

    #[test]
    fn remaining_timeout_blob_returns_error_when_deadline_is_past() {
        let deadline = std::time::Instant::now() - std::time::Duration::from_secs(1);
        let err = remaining_timeout_blob(deadline).expect_err("should return Err");
        assert!(err.to_string().contains("deadline exceeded"));
    }

    /// Verify each post-connect stage is bounded by the deadline
    /// using the same pattern as `transfer_send_rpc`.
    #[tokio::test]
    async fn stages_bounded_by_deadline() {
        for stage in ["open_bi", "write", "finish", "read"] {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(10);
            let result: Result<Result<(), std::io::Error>, _> =
                tokio::time::timeout(remaining_timeout_blob(deadline).unwrap(), std::future::pending()).await;
            assert!(result.is_err(), "{stage} stage must be bounded by deadline");
        }
    }

    /// Expired deadline fails immediately for every stage.
    #[test]
    fn expired_deadline_blocks_all_stages() {
        let deadline = std::time::Instant::now() - std::time::Duration::from_secs(1);
        for stage in ["open_bi", "write", "finish", "read"] {
            let err = remaining_timeout_blob(deadline).expect_err("expired deadline must fail");
            assert!(
                err.to_string().contains("deadline exceeded"),
                "{stage} stage must not proceed past an expired deadline"
            );
        }
    }

    /// End-to-end: connect to an unreachable peer through a real iroh
    /// endpoint and verify the connect stage times out.
    #[tokio::test]
    async fn connect_timeout_on_unreachable_peer() {
        use aspen_client_api::CLIENT_ALPN;

        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(secret_key)
            .bind_addr(std::net::SocketAddr::from(([127, 0, 0, 1], 0u16)))
            .expect("invalid bind address")
            .bind()
            .await
            .expect("failed to bind endpoint");

        let unreachable_key = iroh::SecretKey::generate(&mut rand::rng());
        let unreachable_addr = iroh::EndpointAddr::from(unreachable_key.public());

        let connect_result = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            endpoint.connect(unreachable_addr, CLIENT_ALPN),
        )
        .await;

        assert!(connect_result.is_err(), "connecting to an unreachable peer must time out");

        endpoint.close().await;
    }
}
