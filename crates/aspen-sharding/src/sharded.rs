//! Sharded KeyValueStore wrapper for horizontal scaling.
//!
//! Wraps multiple KeyValueStore implementations (one per shard) and routes
//! operations to the appropriate shard based on key hashing.
//!
//! # Architecture
//!
//! ```text
//! ShardedKeyValueStore
//!     |
//!     +-- ShardRouter (determines shard for each key)
//!     |
//!     +-- shards[0] -> RaftNode / KeyValueStore
//!     +-- shards[1] -> RaftNode / KeyValueStore
//!     +-- shards[2] -> RaftNode / KeyValueStore
//!     +-- ...
//! ```
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_SHARDS enforced at construction
//! - Explicit error handling: Shard lookup errors are propagated
//! - Thread-safe: Uses Arc for shared access to shards

use std::collections::HashMap;
use std::sync::Arc;

use aspen_core::BatchCondition;
use aspen_core::BatchOperation;
use aspen_core::DeleteRequest;
use aspen_core::DeleteResult;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::ReadResult;
use aspen_core::ScanRequest;
use aspen_core::ScanResult;
use aspen_core::TxnOp;
use aspen_core::WriteCommand;
use aspen_core::WriteOp;
use aspen_core::WriteRequest;
use aspen_core::WriteResult;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::router::ShardConfig;
use crate::router::ShardId;
use crate::router::ShardRouter;
use crate::topology::ShardState;
use crate::topology::ShardTopology;

/// A sharded KeyValueStore that distributes keys across multiple shards.
///
/// Each shard is an independent KeyValueStore (typically a RaftNode) that
/// handles a subset of the key space determined by consistent hashing.
///
/// When a shard is not available locally (due to split/merge), the store
/// returns `ShardMoved` errors with redirect information so clients can
/// update their topology cache and retry.
#[derive(Clone)]
pub struct ShardedKeyValueStore<KV: KeyValueStore> {
    /// Router for determining which shard owns each key.
    router: ShardRouter,
    /// Map from shard ID to the KeyValueStore for that shard.
    shards: Arc<RwLock<HashMap<ShardId, Arc<KV>>>>,
    /// Optional topology reference for ShardMoved error generation.
    /// When set, enables proper redirect responses with successor info.
    topology: Option<Arc<RwLock<ShardTopology>>>,
}

impl<KV: KeyValueStore> ShardedKeyValueStore<KV> {
    /// Create a new sharded store with the given configuration.
    ///
    /// Initially no shards are registered. Use `add_shard()` to add shard backends.
    ///
    /// # Arguments
    ///
    /// * `config` - Shard configuration specifying number of shards
    pub fn new(config: ShardConfig) -> Self {
        Self {
            router: ShardRouter::new(config),
            shards: Arc::new(RwLock::new(HashMap::new())),
            topology: None,
        }
    }

    /// Create a new sharded store with topology reference for ShardMoved errors.
    ///
    /// When the topology is set, the store can return proper ShardMoved errors
    /// with successor shard information when a shard is not available locally.
    ///
    /// # Arguments
    ///
    /// * `config` - Shard configuration specifying number of shards
    /// * `topology` - Shared topology reference for redirect info
    pub fn with_topology(config: ShardConfig, topology: Arc<RwLock<ShardTopology>>) -> Self {
        Self {
            router: ShardRouter::new(config),
            shards: Arc::new(RwLock::new(HashMap::new())),
            topology: Some(topology),
        }
    }

    /// Set the topology reference for ShardMoved error generation.
    ///
    /// This can be called after construction to enable proper redirect responses.
    pub fn set_topology(&mut self, topology: Arc<RwLock<ShardTopology>>) {
        self.topology = Some(topology);
    }

    /// Register a KeyValueStore for a specific shard.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard ID to register
    /// * `store` - The KeyValueStore implementation for this shard
    ///
    /// # Panics
    ///
    /// Panics if shard_id >= num_shards
    pub async fn add_shard(&self, shard_id: ShardId, store: Arc<KV>) {
        assert!(
            shard_id < self.router.num_shards(),
            "shard_id {} >= num_shards {}",
            shard_id,
            self.router.num_shards()
        );
        self.shards.write().await.insert(shard_id, store);
        self.router.add_local_shard(shard_id);
    }

    /// Remove a shard from the store.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard ID to remove
    pub async fn remove_shard(&self, shard_id: ShardId) {
        self.shards.write().await.remove(&shard_id);
        self.router.remove_local_shard(shard_id);
    }

    /// Get the shard for a specific key.
    ///
    /// Returns `ShardMoved` error with redirect info if the shard is not local.
    /// When topology is available, checks for tombstoned shards and includes
    /// successor information in the error.
    async fn get_shard(&self, key: &str) -> Result<Arc<KV>, KeyValueStoreError> {
        let shard_id = self.router.get_shard_for_key(key);

        // Try to get the shard locally
        if let Some(shard) = self.shards.read().await.get(&shard_id).cloned() {
            return Ok(shard);
        }

        // Shard not local - generate ShardMoved error with redirect info
        self.make_shard_moved_error(key, shard_id).await
    }

    /// Get the shard by ID.
    ///
    /// Returns `ShardMoved` error with redirect info if the shard is not local.
    async fn get_shard_by_id(&self, shard_id: ShardId) -> Result<Arc<KV>, KeyValueStoreError> {
        if let Some(shard) = self.shards.read().await.get(&shard_id).cloned() {
            return Ok(shard);
        }

        // Shard not local - generate ShardMoved error
        self.make_shard_moved_error_by_id(shard_id).await
    }

    /// Generate a ShardMoved error with proper redirect information.
    ///
    /// Checks the topology for tombstoned shards and includes successor info.
    async fn make_shard_moved_error(&self, key: &str, shard_id: ShardId) -> Result<Arc<KV>, KeyValueStoreError> {
        // Check topology for redirect info if available
        if let Some(ref topology) = self.topology {
            let topo = topology.read().await;

            if let Some(info) = topo.get_shard(shard_id) {
                // If shard is tombstoned, return successor info
                if let ShardState::Tombstone {
                    successor_shard_id: Some(successor),
                    ..
                } = &info.state
                {
                    return Err(KeyValueStoreError::ShardMoved {
                        key: key.to_string(),
                        new_shard_id: *successor,
                        topology_version: topo.version,
                    });
                }

                // If shard is in transitional state, return ShardNotReady
                if info.state.is_transitioning() {
                    let state_name = match &info.state {
                        ShardState::Splitting { .. } => "splitting",
                        ShardState::Merging { .. } => "merging",
                        _ => "transitioning",
                    };
                    return Err(KeyValueStoreError::ShardNotReady {
                        shard_id,
                        state: state_name.to_string(),
                    });
                }
            }

            // Shard not in topology or is active but not local
            return Err(KeyValueStoreError::ShardMoved {
                key: key.to_string(),
                new_shard_id: shard_id,
                topology_version: topo.version,
            });
        }

        // No topology available - fallback to generic ShardMoved
        Err(KeyValueStoreError::ShardMoved {
            key: key.to_string(),
            new_shard_id: shard_id,
            topology_version: 0,
        })
    }

    /// Generate a ShardMoved error by shard ID (when key is unknown).
    async fn make_shard_moved_error_by_id(&self, shard_id: ShardId) -> Result<Arc<KV>, KeyValueStoreError> {
        // Check topology for redirect info if available
        if let Some(ref topology) = self.topology {
            let topo = topology.read().await;

            if let Some(info) = topo.get_shard(shard_id) {
                // If shard is tombstoned, return successor info
                if let ShardState::Tombstone {
                    successor_shard_id: Some(successor),
                    ..
                } = &info.state
                {
                    return Err(KeyValueStoreError::ShardMoved {
                        key: String::new(),
                        new_shard_id: *successor,
                        topology_version: topo.version,
                    });
                }

                // If shard is in transitional state, return ShardNotReady
                if info.state.is_transitioning() {
                    let state_name = match &info.state {
                        ShardState::Splitting { .. } => "splitting",
                        ShardState::Merging { .. } => "merging",
                        _ => "transitioning",
                    };
                    return Err(KeyValueStoreError::ShardNotReady {
                        shard_id,
                        state: state_name.to_string(),
                    });
                }
            }

            // Shard not in topology or is active but not local
            return Err(KeyValueStoreError::ShardMoved {
                key: String::new(),
                new_shard_id: shard_id,
                topology_version: topo.version,
            });
        }

        // No topology available - fallback to generic ShardMoved
        Err(KeyValueStoreError::ShardMoved {
            key: String::new(),
            new_shard_id: shard_id,
            topology_version: 0,
        })
    }

    /// Check if a shard can accept writes based on its state.
    ///
    /// Returns an error if the shard is transitioning or tombstoned.
    async fn check_shard_writable(&self, shard_id: ShardId) -> Result<(), KeyValueStoreError> {
        if let Some(ref topology) = self.topology {
            let topo = topology.read().await;

            if let Some(info) = topo.get_shard(shard_id)
                && !info.state.can_write()
            {
                match &info.state {
                    ShardState::Splitting { .. } => {
                        return Err(KeyValueStoreError::ShardNotReady {
                            shard_id,
                            state: "splitting".to_string(),
                        });
                    }
                    ShardState::Merging { target_shard_id } => {
                        return Err(KeyValueStoreError::ShardMoved {
                            key: String::new(),
                            new_shard_id: *target_shard_id,
                            topology_version: topo.version,
                        });
                    }
                    ShardState::Tombstone { successor_shard_id, .. } => {
                        return Err(KeyValueStoreError::ShardMoved {
                            key: String::new(),
                            new_shard_id: successor_shard_id.unwrap_or(shard_id),
                            topology_version: topo.version,
                        });
                    }
                    ShardState::Active => {} // Can write
                }
            }
        }
        Ok(())
    }

    /// Get the router.
    pub fn router(&self) -> &ShardRouter {
        &self.router
    }

    /// Get the number of registered shards.
    pub async fn registered_shard_count(&self) -> usize {
        self.shards.read().await.len()
    }

    /// Extract key from a batch operation.
    fn batch_op_key(op: &BatchOperation) -> &str {
        match op {
            BatchOperation::Set { key, .. } => key,
            BatchOperation::Delete { key } => key,
        }
    }

    /// Extract key from a transaction operation.
    fn txn_op_key(op: &TxnOp) -> &str {
        match op {
            TxnOp::Put { key, .. } => key,
            TxnOp::Delete { key } => key,
            TxnOp::Get { key } => key,
            TxnOp::Range { prefix, .. } => prefix,
        }
    }

    /// Extract key from a batch condition.
    fn condition_key(cond: &BatchCondition) -> &str {
        match cond {
            BatchCondition::ValueEquals { key, .. } => key,
            BatchCondition::KeyExists { key } => key,
            BatchCondition::KeyNotExists { key } => key,
        }
    }

    /// Check if all keys in a write command belong to the same shard.
    ///
    /// Returns the shard ID if all keys hash to the same shard.
    fn validate_same_shard(&self, command: &WriteCommand) -> Result<Option<ShardId>, KeyValueStoreError> {
        let keys: Vec<&str> = match command {
            WriteCommand::Set { key, .. } => vec![key],
            WriteCommand::SetMulti { pairs, .. } => pairs.iter().map(|(k, _)| k.as_str()).collect(),
            WriteCommand::Delete { key, .. } => vec![key],
            WriteCommand::DeleteMulti { keys, .. } => keys.iter().map(|k| k.as_str()).collect(),
            WriteCommand::CompareAndSwap { key, .. } => vec![key],
            WriteCommand::CompareAndDelete { key, .. } => vec![key],
            WriteCommand::Batch { operations } => operations.iter().map(Self::batch_op_key).collect(),
            WriteCommand::ConditionalBatch { operations, conditions } => {
                let mut keys: Vec<&str> = operations.iter().map(Self::batch_op_key).collect();
                keys.extend(conditions.iter().map(Self::condition_key));
                keys
            }
            WriteCommand::SetWithTTL { key, .. } => vec![key],
            WriteCommand::SetMultiWithTTL { pairs, .. } => pairs.iter().map(|(k, _)| k.as_str()).collect(),
            WriteCommand::SetWithLease { key, .. } => vec![key],
            WriteCommand::SetMultiWithLease { pairs, .. } => pairs.iter().map(|(k, _)| k.as_str()).collect(),
            WriteCommand::LeaseGrant { .. } => return Ok(None), // Global operations
            WriteCommand::LeaseRevoke { .. } => return Ok(None),
            WriteCommand::LeaseKeepalive { .. } => return Ok(None),
            WriteCommand::Transaction {
                compare: _,
                success,
                failure,
            } => {
                let mut keys: Vec<&str> = success.iter().map(Self::txn_op_key).collect();
                keys.extend(failure.iter().map(Self::txn_op_key));
                keys
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                let mut keys: Vec<&str> = read_set.iter().map(|(k, _)| k.as_str()).collect();
                keys.extend(write_set.iter().map(|op| match op {
                    WriteOp::Set { key, .. } => key.as_str(),
                    WriteOp::Delete { key } => key.as_str(),
                }));
                keys
            }
            // Shard topology operations are control plane only (shard 0)
            // and don't involve user keys
            WriteCommand::ShardSplit { .. } | WriteCommand::ShardMerge { .. } | WriteCommand::TopologyUpdate { .. } => {
                return Ok(None); // Control plane operations
            }
        };

        if keys.is_empty() {
            return Ok(None);
        }

        let first_shard = self.router.get_shard_for_key(keys[0]);
        for key in &keys[1..] {
            let shard = self.router.get_shard_for_key(key);
            if shard != first_shard {
                return Err(KeyValueStoreError::Failed {
                    reason: format!(
                        "cross-shard operation not supported: key '{}' (shard {}) differs from '{}' (shard {})",
                        key, shard, keys[0], first_shard
                    ),
                });
            }
        }

        Ok(Some(first_shard))
    }
}

#[async_trait]
impl<KV: KeyValueStore + Send + Sync + 'static> KeyValueStore for ShardedKeyValueStore<KV> {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        // Validate all keys belong to the same shard
        let shard_id: u32 = self.validate_same_shard(&request.command)?.unwrap_or_default();

        // Check shard is in writable state (not splitting/merging/tombstone)
        self.check_shard_writable(shard_id).await?;

        let shard = self.get_shard_by_id(shard_id).await?;
        shard.write(request).await
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let shard = self.get_shard(&request.key).await?;
        shard.read(request).await
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let shard = self.get_shard(&request.key).await?;
        shard.delete(request).await
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        // Scan operations must query all shards since prefix queries can span multiple shards
        let shard_ids = self.router.get_shards_for_prefix(&request.prefix);
        let shards = self.shards.read().await;

        let mut all_entries = Vec::new();
        let mut max_entries_per_shard = request.limit_results.unwrap_or(1000) / shard_ids.len().max(1) as u32;
        if max_entries_per_shard == 0 {
            max_entries_per_shard = 1;
        }

        for shard_id in shard_ids {
            if let Some(shard) = shards.get(&shard_id) {
                let shard_request = ScanRequest {
                    prefix: request.prefix.clone(),
                    limit_results: Some(max_entries_per_shard),
                    continuation_token: None, // Can't use tokens across shards
                };

                match shard.scan(shard_request).await {
                    Ok(result) => {
                        all_entries.extend(result.entries);
                    }
                    Err(KeyValueStoreError::NotFound { .. }) => {
                        // Shard has no matching entries, continue
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Sort all entries by key for consistent ordering
        all_entries.sort_by(|a, b| a.key.cmp(&b.key));

        // Apply overall limit
        let limit = request.limit_results.unwrap_or(1000) as usize;
        if all_entries.len() > limit {
            all_entries.truncate(limit);
        }

        // Generate continuation token if there might be more
        let is_truncated = all_entries.len() >= limit;
        let continuation_token = if is_truncated {
            all_entries.last().map(|e| e.key.clone())
        } else {
            None
        };
        let result_count = all_entries.len() as u32;

        Ok(ScanResult {
            entries: all_entries,
            result_count,
            is_truncated,
            continuation_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_sharded_store_routing() {
        let config = ShardConfig::new(4);
        let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);

        // Add shards
        for i in 0..4 {
            store.add_shard(i, DeterministicKeyValueStore::new()).await;
        }

        assert_eq!(store.registered_shard_count().await, 4);
    }

    #[tokio::test]
    async fn test_sharded_store_write_read() {
        let config = ShardConfig::new(2);
        let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);

        // Add shards
        for i in 0..2 {
            store.add_shard(i, DeterministicKeyValueStore::new()).await;
        }

        // Write a key
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: "test_key".to_string(),
                value: "test_value".to_string(),
            },
        };
        store.write(write_req).await.unwrap();

        // Read it back
        let read_req = ReadRequest::new("test_key".to_string());
        let result = store.read(read_req).await.unwrap();
        assert!(result.kv.is_some());
        assert_eq!(result.kv.unwrap().value, "test_value");
    }

    #[tokio::test]
    async fn test_sharded_store_missing_shard_returns_shard_moved() {
        let config = ShardConfig::new(4);
        let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);

        // Only add shard 0
        store.add_shard(0, DeterministicKeyValueStore::new()).await;

        // Find a key that routes to a missing shard
        let mut test_key = String::new();
        let mut expected_shard = 0;
        for i in 0..100 {
            let key = format!("key_{}", i);
            let shard = store.router.get_shard_for_key(&key);
            if shard != 0 {
                test_key = key;
                expected_shard = shard;
                break;
            }
        }

        // Attempt to write should fail with ShardMoved error
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: test_key.clone(),
                value: "value".to_string(),
            },
        };
        let result = store.write(write_req).await;
        assert!(matches!(
            result,
            Err(KeyValueStoreError::ShardMoved {
                new_shard_id,
                ..
            }) if new_shard_id == expected_shard
        ));
    }

    #[tokio::test]
    async fn test_shard_moved_with_topology_tombstone() {
        let config = ShardConfig::new(2);

        // Create topology where shard 1 is tombstoned with successor shard 0
        let mut topology = ShardTopology::new(2, 1000);
        topology.apply_merge(1, 0, 2000).expect("merge should succeed");

        let topology = Arc::new(RwLock::new(topology));
        let store: ShardedKeyValueStore<DeterministicKeyValueStore> =
            ShardedKeyValueStore::with_topology(config, topology.clone());

        // Only add shard 0
        store.add_shard(0, DeterministicKeyValueStore::new()).await;

        // Find a key that routes to shard 1 (the tombstoned shard)
        let mut test_key = String::new();
        for i in 0..100 {
            let key = format!("key_{}", i);
            if store.router.get_shard_for_key(&key) == 1 {
                test_key = key;
                break;
            }
        }

        // Should return ShardMoved pointing to successor (shard 0)
        let read_req = ReadRequest::new(test_key.clone());
        let result = store.read(read_req).await;
        assert!(matches!(
            result,
            Err(KeyValueStoreError::ShardMoved {
                new_shard_id: 0,
                topology_version: 2,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_shard_not_ready_when_splitting() {
        let config = ShardConfig::new(2);

        // Create topology where shard 0 is splitting
        let mut topology = ShardTopology::new(2, 1000);
        if let Some(shard) = topology.get_shard_mut(0) {
            shard.state = ShardState::Splitting {
                split_key: "m".to_string(),
                new_shard_id: 2,
            };
        }

        let topology = Arc::new(RwLock::new(topology));
        let mut store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);
        store.set_topology(topology);

        // Add shard 0
        store.add_shard(0, DeterministicKeyValueStore::new()).await;

        // Find a key that routes to shard 0
        let mut test_key = String::new();
        for i in 0..100 {
            let key = format!("key_{}", i);
            if store.router.get_shard_for_key(&key) == 0 {
                test_key = key;
                break;
            }
        }

        // Write should fail with ShardNotReady
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: test_key.clone(),
                value: "value".to_string(),
            },
        };
        let result = store.write(write_req).await;
        assert!(matches!(
            result,
            Err(KeyValueStoreError::ShardNotReady {
                shard_id: 0,
                state,
            }) if state == "splitting"
        ));
    }

    #[tokio::test]
    async fn test_shard_moved_when_merging() {
        let config = ShardConfig::new(2);

        // Create topology where shard 1 is merging into shard 0
        let mut topology = ShardTopology::new(2, 1000);
        if let Some(shard) = topology.get_shard_mut(1) {
            shard.state = ShardState::Merging { target_shard_id: 0 };
        }

        let topology = Arc::new(RwLock::new(topology));
        let mut store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);
        store.set_topology(topology);

        // Add both shards
        store.add_shard(0, DeterministicKeyValueStore::new()).await;
        store.add_shard(1, DeterministicKeyValueStore::new()).await;

        // Find a key that routes to shard 1
        let mut test_key = String::new();
        for i in 0..100 {
            let key = format!("key_{}", i);
            if store.router.get_shard_for_key(&key) == 1 {
                test_key = key;
                break;
            }
        }

        // Write to merging shard should fail with ShardMoved pointing to target
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: test_key.clone(),
                value: "value".to_string(),
            },
        };
        let result = store.write(write_req).await;
        assert!(matches!(result, Err(KeyValueStoreError::ShardMoved { new_shard_id: 0, .. })));
    }

    #[tokio::test]
    async fn test_cross_shard_operation_rejected() {
        let config = ShardConfig::new(4);
        let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);

        // Add all shards
        for i in 0..4 {
            store.add_shard(i, DeterministicKeyValueStore::new()).await;
        }

        // Find two keys that route to different shards
        let mut key1 = String::new();
        let mut key2 = String::new();
        for i in 0..100 {
            let k = format!("key_{}", i);
            let shard = store.router.get_shard_for_key(&k);
            if shard == 0 && key1.is_empty() {
                key1 = k;
            } else if shard == 1 && key2.is_empty() {
                key2 = k;
            }
            if !key1.is_empty() && !key2.is_empty() {
                break;
            }
        }

        // Cross-shard SetMulti should fail
        let write_req = WriteRequest {
            command: WriteCommand::SetMulti {
                pairs: vec![(key1.clone(), "v1".to_string()), (key2.clone(), "v2".to_string())],
            },
        };
        let result = store.write(write_req).await;
        assert!(result.is_err());
        if let Err(KeyValueStoreError::Failed { reason }) = result {
            assert!(reason.contains("cross-shard"));
        }
    }

    #[tokio::test]
    async fn test_sharded_scan() {
        let config = ShardConfig::new(2);
        let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::new(config);

        // Add shards
        for i in 0..2 {
            store.add_shard(i, DeterministicKeyValueStore::new()).await;
        }

        // Write keys to different shards with same prefix
        for i in 0..10 {
            let key = format!("prefix:{}", i);
            let write_req = WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value: format!("value_{}", i),
                },
            };
            store.write(write_req).await.unwrap();
        }

        // Scan should find all keys
        let scan_req = ScanRequest {
            prefix: "prefix:".to_string(),
            limit: Some(100),
            continuation_token: None,
        };
        let result = store.scan(scan_req).await.unwrap();
        assert_eq!(result.entries.len(), 10);

        // Results should be sorted
        for i in 0..result.entries.len() - 1 {
            assert!(result.entries[i].key < result.entries[i + 1].key);
        }
    }
}
