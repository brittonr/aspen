/// Integration tests for shard split and merge operations.
///
/// Tests the ShardedKeyValueStore behavior during topology changes including:
/// - Basic split operations and data availability
/// - Merge operations and tombstone redirect
/// - ShardMoved error responses with proper redirect info
/// - Concurrent operations during topology transitions
use std::sync::Arc;

use aspen::api::DeterministicKeyValueStore;
use aspen::api::KeyValueStore;
use aspen::api::KeyValueStoreError;
use aspen::api::ReadRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
use aspen::sharding::ShardConfig;
use aspen::sharding::ShardTopology;
use aspen::sharding::ShardedKeyValueStore;
use tokio::sync::RwLock;

/// Create a sharded store with the given number of shards and topology.
async fn create_sharded_store(
    num_shards: u32,
) -> (ShardedKeyValueStore<DeterministicKeyValueStore>, Arc<RwLock<ShardTopology>>) {
    let config = ShardConfig::new(num_shards);
    let topology = Arc::new(RwLock::new(ShardTopology::new(num_shards, 1000)));
    let store: ShardedKeyValueStore<DeterministicKeyValueStore> =
        ShardedKeyValueStore::with_topology(config, topology.clone());

    // Add all shards
    for i in 0..num_shards {
        store.add_shard(i, DeterministicKeyValueStore::new()).await;
    }

    (store, topology)
}

/// Helper to find a key that routes to a specific shard.
fn find_key_for_shard(store: &ShardedKeyValueStore<DeterministicKeyValueStore>, target_shard: u32) -> String {
    for i in 0..1000 {
        let key = format!("key_{}", i);
        if store.router().get_shard_for_key(&key) == target_shard {
            return key;
        }
    }
    panic!("Could not find key for shard {}", target_shard);
}

#[tokio::test]
async fn test_basic_sharded_write_read() {
    let (store, _topology) = create_sharded_store(4).await;

    // Write keys that route to different shards
    for shard in 0..4 {
        let key = find_key_for_shard(&store, shard);
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: key.clone(),
                value: format!("value_for_shard_{}", shard),
            },
        };
        store.write(write_req).await.expect("write should succeed");

        // Verify read
        let read_req = ReadRequest::new(key.clone());
        let result = store.read(read_req).await.expect("read should succeed");
        assert!(result.kv.is_some());
        assert_eq!(result.kv.unwrap().value, format!("value_for_shard_{}", shard));
    }
}

#[tokio::test]
async fn test_split_updates_topology_version() {
    let (store, topology) = create_sharded_store(1).await;

    // Write initial data
    let key = "test_key".to_string();
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: key.clone(),
            value: "initial_value".to_string(),
        },
    };
    store.write(write_req).await.expect("write should succeed");

    // Verify initial topology version
    assert_eq!(topology.read().await.version, 1);

    // Perform split
    {
        let mut topo = topology.write().await;
        topo.apply_split(0, "m".to_string(), 1, 2000).expect("split should succeed");
    }

    // Verify topology version incremented
    assert_eq!(topology.read().await.version, 2);
}

#[tokio::test]
async fn test_merge_creates_tombstone_with_successor() {
    let (store, topology) = create_sharded_store(2).await;

    // Write data to shard 1
    let key = find_key_for_shard(&store, 1);
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: key.clone(),
            value: "value_in_shard_1".to_string(),
        },
    };
    store.write(write_req).await.expect("write should succeed");

    // Perform merge: shard 1 -> shard 0
    {
        let mut topo = topology.write().await;
        topo.apply_merge(1, 0, 2000).expect("merge should succeed");
    }

    // Remove shard 1 from the store (simulating it being no longer local)
    store.remove_shard(1).await;

    // Now reading from a key that routes to shard 1 should return ShardMoved
    let read_req = ReadRequest::new(key.clone());
    let result = store.read(read_req).await;

    match result {
        Err(KeyValueStoreError::ShardMoved {
            new_shard_id,
            topology_version,
            ..
        }) => {
            assert_eq!(new_shard_id, 0, "should redirect to successor shard 0");
            assert_eq!(topology_version, 2, "topology version should be 2");
        }
        other => panic!("Expected ShardMoved error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_shard_moved_error_contains_key() {
    let (store, _topology) = create_sharded_store(4).await;

    // Remove shard 2
    store.remove_shard(2).await;

    // Find a key that routes to the removed shard
    let key = find_key_for_shard(&store, 2);

    // Try to read from removed shard
    let read_req = ReadRequest::new(key.clone());
    let result = store.read(read_req).await;

    match result {
        Err(KeyValueStoreError::ShardMoved {
            key: error_key,
            new_shard_id,
            ..
        }) => {
            assert_eq!(error_key, key, "error should contain the key");
            assert_eq!(new_shard_id, 2, "should point to the expected shard");
        }
        other => panic!("Expected ShardMoved error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_scan_across_multiple_shards() {
    let (store, _topology) = create_sharded_store(4).await;

    // Write keys with a common prefix that span multiple shards
    let prefix = "scan_test:";
    for i in 0..20 {
        let key = format!("{}{:02}", prefix, i);
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: key.clone(),
                value: format!("value_{}", i),
            },
        };
        store.write(write_req).await.expect("write should succeed");
    }

    // Scan should aggregate results from all shards
    let scan_req = aspen::api::ScanRequest {
        prefix: prefix.to_string(),
        limit: Some(100),
        continuation_token: None,
    };
    let result = store.scan(scan_req).await.expect("scan should succeed");

    assert_eq!(result.entries.len(), 20, "should find all 20 entries");

    // Verify results are sorted
    for i in 0..result.entries.len() - 1 {
        assert!(result.entries[i].key < result.entries[i + 1].key, "entries should be sorted");
    }
}

#[tokio::test]
async fn test_write_blocked_during_split() {
    let config = ShardConfig::new(2);

    // Create topology where shard 0 is splitting
    let mut topology = ShardTopology::new(2, 1000);
    if let Some(shard) = topology.get_shard_mut(0) {
        shard.state = aspen::sharding::topology::ShardState::Splitting {
            split_key: "m".to_string(),
            new_shard_id: 2,
        };
    }

    let topology = Arc::new(RwLock::new(topology));
    let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::with_topology(config, topology);

    // Add shard 0
    store.add_shard(0, DeterministicKeyValueStore::new()).await;

    // Find a key that routes to shard 0
    let key = find_key_for_shard(&store, 0);

    // Write should be blocked with ShardNotReady
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key,
            value: "value".to_string(),
        },
    };
    let result = store.write(write_req).await;

    match result {
        Err(KeyValueStoreError::ShardNotReady { shard_id, state }) => {
            assert_eq!(shard_id, 0);
            assert_eq!(state, "splitting");
        }
        other => panic!("Expected ShardNotReady error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_write_redirected_during_merge() {
    let config = ShardConfig::new(2);

    // Create topology where shard 1 is merging into shard 0
    let mut topology = ShardTopology::new(2, 1000);
    if let Some(shard) = topology.get_shard_mut(1) {
        shard.state = aspen::sharding::topology::ShardState::Merging { target_shard_id: 0 };
    }

    let topology = Arc::new(RwLock::new(topology));
    let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::with_topology(config, topology);

    // Add both shards
    store.add_shard(0, DeterministicKeyValueStore::new()).await;
    store.add_shard(1, DeterministicKeyValueStore::new()).await;

    // Find a key that routes to shard 1 (the merging shard)
    let key = find_key_for_shard(&store, 1);

    // Write should redirect to target shard
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key,
            value: "value".to_string(),
        },
    };
    let result = store.write(write_req).await;

    match result {
        Err(KeyValueStoreError::ShardMoved { new_shard_id, .. }) => {
            assert_eq!(new_shard_id, 0, "should redirect to merge target");
        }
        other => panic!("Expected ShardMoved error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_topology_version_in_shard_moved_error() {
    // Start with 2 shards to make split/merge easier
    let config = ShardConfig::new(2);
    let mut topology = ShardTopology::new(2, 1000);

    // Apply a merge to increment version (shard 1 -> shard 0)
    topology.apply_merge(1, 0, 2000).expect("merge should succeed");

    // Now version should be 2
    assert_eq!(topology.version, 2);

    let topology = Arc::new(RwLock::new(topology));
    let store: ShardedKeyValueStore<DeterministicKeyValueStore> = ShardedKeyValueStore::with_topology(config, topology);

    // Only add shard 0 (shard 1 is tombstoned)
    store.add_shard(0, DeterministicKeyValueStore::new()).await;

    // Find a key that routes to shard 1 (tombstoned)
    let key = find_key_for_shard(&store, 1);

    let read_req = ReadRequest::new(key);
    let result = store.read(read_req).await;

    match result {
        Err(KeyValueStoreError::ShardMoved {
            topology_version,
            new_shard_id,
            ..
        }) => {
            assert_eq!(topology_version, 2, "should include current topology version");
            assert_eq!(new_shard_id, 0, "should redirect to successor shard");
        }
        other => panic!("Expected ShardMoved error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multi_key_write_same_shard() {
    let (store, _topology) = create_sharded_store(4).await;

    // Find multiple keys that route to the same shard
    let target_shard = 1;
    let mut keys = Vec::new();
    for i in 0..1000 {
        let key = format!("batch_key_{}", i);
        if store.router().get_shard_for_key(&key) == target_shard {
            keys.push(key);
            if keys.len() >= 3 {
                break;
            }
        }
    }
    assert!(keys.len() >= 3, "need at least 3 keys in same shard");

    // SetMulti should succeed for keys in the same shard
    let write_req = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: keys.iter().enumerate().map(|(i, k)| (k.clone(), format!("value_{}", i))).collect(),
        },
    };
    store.write(write_req).await.expect("SetMulti should succeed");

    // Verify all keys were written
    for (i, key) in keys.iter().enumerate() {
        let read_req = ReadRequest::new(key.clone());
        let result = store.read(read_req).await.expect("read should succeed");
        assert!(result.kv.is_some());
        assert_eq!(result.kv.unwrap().value, format!("value_{}", i));
    }
}

#[tokio::test]
async fn test_cross_shard_write_rejected() {
    let (store, _topology) = create_sharded_store(4).await;

    // Find keys that route to different shards
    let key1 = find_key_for_shard(&store, 0);
    let key2 = find_key_for_shard(&store, 1);

    // SetMulti should fail for cross-shard operations
    let write_req = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: vec![
                (key1.clone(), "value1".to_string()),
                (key2.clone(), "value2".to_string()),
            ],
        },
    };
    let result = store.write(write_req).await;

    match result {
        Err(KeyValueStoreError::Failed { reason }) => {
            assert!(reason.contains("cross-shard"), "error should mention cross-shard: {}", reason);
        }
        other => panic!("Expected Failed error for cross-shard, got {:?}", other),
    }
}
