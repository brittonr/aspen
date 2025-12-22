//! Tests for in-memory API implementations.
//!
//! Tests DeterministicClusterController and DeterministicKeyValueStore
//! for correct behavior including validation, scan pagination, and error paths.
//!
//! Target: Increase api/inmemory.rs coverage from 5-7% to 50%+

mod support;

use bolero::check;
use support::bolero_generators::{ScanKeyCount, ScanPrefixWithColon, ValidApiKey, ValidApiValue};

use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ControlPlaneError,
    DEFAULT_SCAN_LIMIT, DeleteRequest, DeterministicClusterController, DeterministicKeyValueStore,
    InitRequest, KeyValueStore, KeyValueStoreError, MAX_SCAN_RESULTS, ReadRequest, ScanRequest,
    WriteCommand, WriteRequest,
};

// DeterministicClusterController tests

#[tokio::test]
async fn test_cluster_controller_new_creates_empty_state() {
    let controller = DeterministicClusterController::new();
    let state = controller.current_state().await.unwrap();
    assert!(state.nodes.is_empty());
    assert!(state.members.is_empty());
    assert!(state.learners.is_empty());
}

#[tokio::test]
async fn test_cluster_controller_init_with_empty_members_fails() {
    let controller = DeterministicClusterController::new();
    let result = controller
        .init(InitRequest {
            initial_members: vec![],
        })
        .await;

    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("empty"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

#[tokio::test]
async fn test_cluster_controller_init_with_single_member() {
    let controller = DeterministicClusterController::new();
    let node = ClusterNode::new(1, "node1", None);
    let result = controller
        .init(InitRequest {
            initial_members: vec![node.clone()],
        })
        .await
        .unwrap();

    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.members, vec![1]);
    assert!(result.learners.is_empty());
}

#[tokio::test]
async fn test_cluster_controller_init_with_multiple_members() {
    let controller = DeterministicClusterController::new();
    let nodes = vec![
        ClusterNode::new(1, "node1", None),
        ClusterNode::new(2, "node2", None),
        ClusterNode::new(3, "node3", None),
    ];
    let result = controller
        .init(InitRequest {
            initial_members: nodes,
        })
        .await
        .unwrap();

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.members, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_cluster_controller_add_learner() {
    let controller = DeterministicClusterController::new();

    // Initialize first
    let node = ClusterNode::new(1, "node1", None);
    controller
        .init(InitRequest {
            initial_members: vec![node],
        })
        .await
        .unwrap();

    // Add learner
    let learner = ClusterNode::new(10, "learner1", None);
    let result = controller
        .add_learner(AddLearnerRequest {
            learner: learner.clone(),
        })
        .await
        .unwrap();

    assert_eq!(result.learners.len(), 1);
    assert_eq!(result.learners[0].id, 10);
}

#[tokio::test]
async fn test_cluster_controller_add_multiple_learners() {
    let controller = DeterministicClusterController::new();

    // Add learners without init (valid for deterministic controller)
    for i in 0..5 {
        let learner = ClusterNode::new(100 + i, format!("learner{}", i), None);
        controller
            .add_learner(AddLearnerRequest { learner })
            .await
            .unwrap();
    }

    let state = controller.current_state().await.unwrap();
    assert_eq!(state.learners.len(), 5);
}

#[tokio::test]
async fn test_cluster_controller_change_membership_with_empty_fails() {
    let controller = DeterministicClusterController::new();
    let result = controller
        .change_membership(ChangeMembershipRequest { members: vec![] })
        .await;

    match result {
        Err(ControlPlaneError::InvalidRequest { reason }) => {
            assert!(reason.contains("at least one"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

#[tokio::test]
async fn test_cluster_controller_change_membership_updates_members() {
    let controller = DeterministicClusterController::new();

    // Initialize with 3 members
    let nodes: Vec<ClusterNode> = (1..=3)
        .map(|i| ClusterNode::new(i, format!("node{}", i), None))
        .collect();
    controller
        .init(InitRequest {
            initial_members: nodes,
        })
        .await
        .unwrap();

    // Change to new membership
    let result = controller
        .change_membership(ChangeMembershipRequest {
            members: vec![1, 2, 4, 5],
        })
        .await
        .unwrap();

    assert_eq!(result.members, vec![1, 2, 4, 5]);
}

#[tokio::test]
async fn test_cluster_controller_get_metrics_unsupported() {
    let controller = DeterministicClusterController::new();
    let result = controller.get_metrics().await;

    match result {
        Err(ControlPlaneError::Unsupported { backend, operation }) => {
            assert_eq!(backend, "deterministic");
            assert_eq!(operation, "get_metrics");
        }
        _ => panic!("Expected Unsupported error"),
    }
}

#[tokio::test]
async fn test_cluster_controller_trigger_snapshot_unsupported() {
    let controller = DeterministicClusterController::new();
    let result = controller.trigger_snapshot().await;

    match result {
        Err(ControlPlaneError::Unsupported { backend, operation }) => {
            assert_eq!(backend, "deterministic");
            assert_eq!(operation, "trigger_snapshot");
        }
        _ => panic!("Expected Unsupported error"),
    }
}

#[tokio::test]
async fn test_cluster_controller_current_state_returns_latest() {
    let controller = DeterministicClusterController::new();

    // Modify state
    let node = ClusterNode::new(42, "node42", None);
    controller
        .init(InitRequest {
            initial_members: vec![node],
        })
        .await
        .unwrap();

    // Verify current_state reflects changes
    let state = controller.current_state().await.unwrap();
    assert_eq!(state.nodes.len(), 1);
    assert_eq!(state.nodes[0].id, 42);
}

// DeterministicKeyValueStore tests

#[tokio::test]
async fn test_kv_store_new_is_empty() {
    let store = DeterministicKeyValueStore::new();
    let result = store
        .read(ReadRequest {
            key: "nonexistent".to_string(),
        })
        .await;

    match result {
        Err(KeyValueStoreError::NotFound { key }) => {
            assert_eq!(key, "nonexistent");
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_kv_store_set_and_read() {
    let store = DeterministicKeyValueStore::new();

    // Write
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
        })
        .await
        .unwrap();

    // Read
    let result = store
        .read(ReadRequest {
            key: "key1".to_string(),
        })
        .await
        .unwrap();

    let kv = result.kv.unwrap();
    assert_eq!(kv.key, "key1");
    assert_eq!(kv.value, "value1");
}

#[tokio::test]
async fn test_kv_store_set_overwrites() {
    let store = DeterministicKeyValueStore::new();

    // Write initial
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key1".to_string(),
                value: "initial".to_string(),
            },
        })
        .await
        .unwrap();

    // Overwrite
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key1".to_string(),
                value: "updated".to_string(),
            },
        })
        .await
        .unwrap();

    let result = store
        .read(ReadRequest {
            key: "key1".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(result.kv.unwrap().value, "updated");
}

#[tokio::test]
async fn test_kv_store_set_multi() {
    let store = DeterministicKeyValueStore::new();

    let pairs = vec![
        ("key1".to_string(), "value1".to_string()),
        ("key2".to_string(), "value2".to_string()),
        ("key3".to_string(), "value3".to_string()),
    ];

    store
        .write(WriteRequest {
            command: WriteCommand::SetMulti { pairs },
        })
        .await
        .unwrap();

    // Verify all keys exist
    for i in 1..=3 {
        let result = store
            .read(ReadRequest {
                key: format!("key{}", i),
            })
            .await
            .unwrap();
        assert_eq!(result.kv.unwrap().value, format!("value{}", i));
    }
}

#[tokio::test]
async fn test_kv_store_delete_existing_key() {
    let store = DeterministicKeyValueStore::new();

    // Write then delete
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
        })
        .await
        .unwrap();

    let result = store
        .delete(DeleteRequest {
            key: "key1".to_string(),
        })
        .await
        .unwrap();

    assert!(result.deleted);
    assert_eq!(result.key, "key1");

    // Verify it's gone
    let read_result = store
        .read(ReadRequest {
            key: "key1".to_string(),
        })
        .await;
    assert!(read_result.is_err());
}

#[tokio::test]
async fn test_kv_store_delete_nonexistent_key() {
    let store = DeterministicKeyValueStore::new();

    let result = store
        .delete(DeleteRequest {
            key: "nonexistent".to_string(),
        })
        .await
        .unwrap();

    assert!(!result.deleted);
    assert_eq!(result.key, "nonexistent");
}

#[tokio::test]
async fn test_kv_store_delete_via_write_command() {
    let store = DeterministicKeyValueStore::new();

    // Write
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
        })
        .await
        .unwrap();

    // Delete via WriteCommand
    store
        .write(WriteRequest {
            command: WriteCommand::Delete {
                key: "key1".to_string(),
            },
        })
        .await
        .unwrap();

    // Verify deleted
    let result = store
        .read(ReadRequest {
            key: "key1".to_string(),
        })
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_kv_store_delete_multi() {
    let store = DeterministicKeyValueStore::new();

    // Write multiple keys
    for i in 1..=5 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key{}", i),
                    value: format!("value{}", i),
                },
            })
            .await
            .unwrap();
    }

    // Delete some via DeleteMulti
    store
        .write(WriteRequest {
            command: WriteCommand::DeleteMulti {
                keys: vec!["key1".to_string(), "key3".to_string(), "key5".to_string()],
            },
        })
        .await
        .unwrap();

    // Verify deleted keys are gone
    for key in ["key1", "key3", "key5"] {
        let result = store
            .read(ReadRequest {
                key: key.to_string(),
            })
            .await;
        assert!(result.is_err(), "Key {} should be deleted", key);
    }

    // Verify remaining keys exist
    for key in ["key2", "key4"] {
        let result = store
            .read(ReadRequest {
                key: key.to_string(),
            })
            .await;
        assert!(result.is_ok(), "Key {} should exist", key);
    }
}

#[tokio::test]
async fn test_kv_store_scan_empty() {
    let store = DeterministicKeyValueStore::new();

    let result = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    assert!(result.entries.is_empty());
    assert_eq!(result.count, 0);
    assert!(!result.is_truncated);
    assert!(result.continuation_token.is_none());
}

#[tokio::test]
async fn test_kv_store_scan_all_keys() {
    let store = DeterministicKeyValueStore::new();

    // Write keys
    for i in 1..=10 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key{:02}", i),
                    value: format!("value{}", i),
                },
            })
            .await
            .unwrap();
    }

    let result = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    assert_eq!(result.count, 10);
    assert!(!result.is_truncated);
    // Keys should be sorted
    assert_eq!(result.entries[0].key, "key01");
    assert_eq!(result.entries[9].key, "key10");
}

#[tokio::test]
async fn test_kv_store_scan_with_prefix() {
    let store = DeterministicKeyValueStore::new();

    // Write keys with different prefixes
    for prefix in ["user:", "config:", "session:"] {
        for i in 1..=5 {
            store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("{}{}", prefix, i),
                        value: format!("value_{}{}", prefix, i),
                    },
                })
                .await
                .unwrap();
        }
    }

    // Scan only user: prefix
    let result = store
        .scan(ScanRequest {
            prefix: "user:".to_string(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    assert_eq!(result.count, 5);
    for entry in &result.entries {
        assert!(entry.key.starts_with("user:"));
    }
}

#[tokio::test]
async fn test_kv_store_scan_with_limit() {
    let store = DeterministicKeyValueStore::new();

    // Write 100 keys
    for i in 1..=100 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key{:03}", i),
                    value: format!("value{}", i),
                },
            })
            .await
            .unwrap();
    }

    // Scan with limit of 10
    let result = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: Some(10),
            continuation_token: None,
        })
        .await
        .unwrap();

    assert_eq!(result.count, 10);
    assert!(result.is_truncated);
    assert!(result.continuation_token.is_some());
    assert_eq!(result.entries[0].key, "key001");
    assert_eq!(result.entries[9].key, "key010");
}

#[tokio::test]
async fn test_kv_store_scan_pagination() {
    let store = DeterministicKeyValueStore::new();

    // Write 25 keys
    for i in 1..=25 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key{:02}", i),
                    value: format!("value{}", i),
                },
            })
            .await
            .unwrap();
    }

    // First page
    let page1 = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: Some(10),
            continuation_token: None,
        })
        .await
        .unwrap();

    assert_eq!(page1.count, 10);
    assert!(page1.is_truncated);
    assert_eq!(page1.entries[0].key, "key01");
    assert_eq!(page1.entries[9].key, "key10");

    // Second page
    let page2 = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: Some(10),
            continuation_token: page1.continuation_token,
        })
        .await
        .unwrap();

    assert_eq!(page2.count, 10);
    assert!(page2.is_truncated);
    assert_eq!(page2.entries[0].key, "key11");
    assert_eq!(page2.entries[9].key, "key20");

    // Third page (partial)
    let page3 = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: Some(10),
            continuation_token: page2.continuation_token,
        })
        .await
        .unwrap();

    assert_eq!(page3.count, 5);
    assert!(!page3.is_truncated);
    assert!(page3.continuation_token.is_none());
    assert_eq!(page3.entries[0].key, "key21");
    assert_eq!(page3.entries[4].key, "key25");
}

#[tokio::test]
async fn test_kv_store_scan_default_limit() {
    let store = DeterministicKeyValueStore::new();

    // Write more than DEFAULT_SCAN_LIMIT keys
    for i in 1..=1500 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key{:05}", i),
                    value: format!("v{}", i),
                },
            })
            .await
            .unwrap();
    }

    // Scan without explicit limit
    let result = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    assert_eq!(result.count, DEFAULT_SCAN_LIMIT);
    assert!(result.is_truncated);
}

#[tokio::test]
async fn test_kv_store_scan_respects_max_limit() {
    let store = DeterministicKeyValueStore::new();

    // Write many keys
    for i in 1..=15000 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key{:06}", i),
                    value: format!("v{}", i),
                },
            })
            .await
            .unwrap();
    }

    // Request more than MAX_SCAN_RESULTS
    let result = store
        .scan(ScanRequest {
            prefix: "".to_string(),
            limit: Some(20000),
            continuation_token: None,
        })
        .await
        .unwrap();

    // Should be capped at MAX_SCAN_RESULTS
    assert_eq!(result.count, MAX_SCAN_RESULTS);
    assert!(result.is_truncated);
}

#[tokio::test]
async fn test_kv_store_scan_prefix_no_matches() {
    let store = DeterministicKeyValueStore::new();

    // Write keys
    for i in 1..=10 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("user:{}", i),
                    value: format!("v{}", i),
                },
            })
            .await
            .unwrap();
    }

    // Scan with non-matching prefix
    let result = store
        .scan(ScanRequest {
            prefix: "config:".to_string(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    assert!(result.entries.is_empty());
    assert_eq!(result.count, 0);
    assert!(!result.is_truncated);
}

#[tokio::test]
async fn test_kv_store_write_returns_command() {
    let store = DeterministicKeyValueStore::new();

    let result = store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
        })
        .await
        .unwrap();

    match result.command {
        Some(WriteCommand::Set { key, value }) => {
            assert_eq!(key, "key1");
            assert_eq!(value, "value1");
        }
        _ => panic!("Expected Set command"),
    }
}

// Validation tests (delegated to validate_write_command)

#[tokio::test]
async fn test_kv_store_rejects_oversized_key() {
    let store = DeterministicKeyValueStore::new();

    let oversized_key = "k".repeat(1025);
    let result = store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: oversized_key,
                value: "value".to_string(),
            },
        })
        .await;

    match result {
        Err(KeyValueStoreError::KeyTooLarge { .. }) => {}
        _ => panic!("Expected KeyTooLarge error"),
    }
}

#[tokio::test]
async fn test_kv_store_rejects_oversized_value() {
    let store = DeterministicKeyValueStore::new();

    let oversized_value = "v".repeat(1024 * 1024 + 1);
    let result = store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "key".to_string(),
                value: oversized_value,
            },
        })
        .await;

    match result {
        Err(KeyValueStoreError::ValueTooLarge { .. }) => {}
        _ => panic!("Expected ValueTooLarge error"),
    }
}

#[tokio::test]
async fn test_kv_store_rejects_oversized_batch() {
    let store = DeterministicKeyValueStore::new();

    let pairs: Vec<(String, String)> = (0..101)
        .map(|i| (format!("key{}", i), format!("value{}", i)))
        .collect();

    let result = store
        .write(WriteRequest {
            command: WriteCommand::SetMulti { pairs },
        })
        .await;

    match result {
        Err(KeyValueStoreError::BatchTooLarge { .. }) => {}
        _ => panic!("Expected BatchTooLarge error"),
    }
}

// Property-based tests using Bolero

/// Random write/read operations should be consistent.
#[test]
fn test_kv_store_write_read_consistency() {
    check!()
        .with_type::<(ValidApiKey, ValidApiValue)>()
        .for_each(|(key, value)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = DeterministicKeyValueStore::new();

                store
                    .write(WriteRequest {
                        command: WriteCommand::Set {
                            key: key.0.clone(),
                            value: value.0.clone(),
                        },
                    })
                    .await
                    .unwrap();

                let result = store
                    .read(ReadRequest { key: key.0.clone() })
                    .await
                    .unwrap();
                assert_eq!(result.kv.unwrap().value, value.0);
            });
        });
}

/// Delete should be idempotent.
#[test]
fn test_kv_store_delete_idempotent() {
    check!().with_type::<ValidApiKey>().for_each(|key| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = DeterministicKeyValueStore::new();

            // Delete non-existent key (first time)
            let result1 = store
                .delete(DeleteRequest { key: key.0.clone() })
                .await
                .unwrap();
            assert!(!result1.deleted);

            // Delete again (second time)
            let result2 = store
                .delete(DeleteRequest { key: key.0.clone() })
                .await
                .unwrap();
            assert!(!result2.deleted);
        });
    });
}

/// Scan with various prefixes should only return matching keys.
#[test]
fn test_kv_store_scan_prefix_filtering() {
    check!()
        .with_type::<(ScanPrefixWithColon, ScanKeyCount)>()
        .for_each(|(prefix, num_keys)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = DeterministicKeyValueStore::new();

                // Write keys with the prefix
                for i in 0..num_keys.0 {
                    store
                        .write(WriteRequest {
                            command: WriteCommand::Set {
                                key: format!("{}{}", prefix.0, i),
                                value: format!("v{}", i),
                            },
                        })
                        .await
                        .unwrap();
                }

                // Write keys with different prefix
                for i in 0..5 {
                    store
                        .write(WriteRequest {
                            command: WriteCommand::Set {
                                key: format!("other:{}", i),
                                value: format!("other{}", i),
                            },
                        })
                        .await
                        .unwrap();
                }

                let result = store
                    .scan(ScanRequest {
                        prefix: prefix.0.clone(),
                        limit: None,
                        continuation_token: None,
                    })
                    .await
                    .unwrap();

                assert_eq!(result.count as usize, num_keys.0);
                for entry in &result.entries {
                    assert!(entry.key.starts_with(&prefix.0));
                }
            });
        });
}
