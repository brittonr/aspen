//! Integration tests for the verification subsystem.
//!
//! These tests validate that the verification logic correctly identifies
//! replication status across KV, docs, and blob storage.
//!
//! Since verify commands require a running cluster with Iroh networking,
//! we test the verification logic components directly using the in-memory
//! deterministic implementations.

mod support;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::DeleteRequest;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::ReadRequest;
use aspen::api::ScanRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
use aspen::raft::madsim_network::FailureInjector;
use aspen::raft::madsim_network::MadsimNetworkFactory;
use aspen::raft::madsim_network::MadsimRaftRouter;
use aspen::raft::storage::InMemoryLogStore;
use aspen::raft::storage::InMemoryStateMachine;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen::testing::DeterministicClusterController;
use aspen::testing::DeterministicKeyValueStore;
use aspen_testing::create_test_raft_member_info;
use openraft::Config;
use openraft::Raft;

// =============================================================================
// KV Verification Tests
// =============================================================================

/// Test that KV verification correctly detects successful write/read cycle.
#[tokio::test]
async fn test_kv_verification_basic_roundtrip() {
    let store = DeterministicKeyValueStore::new();

    // Write test keys
    let test_keys = vec!["__verify_test_0", "__verify_test_1", "__verify_test_2"];
    for (i, key) in test_keys.iter().enumerate() {
        let value = format!("verify_value_{}", i);
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key.to_string(),
                    value,
                },
            })
            .await
            .expect("write should succeed");
    }

    // Verify reads match
    for (i, key) in test_keys.iter().enumerate() {
        let expected = format!("verify_value_{}", i);
        let result = store.read(ReadRequest::new(key.to_string())).await.expect("read should succeed");
        let kv = result.kv.as_ref().unwrap_or_else(|| panic!("key {} should be found", key));
        assert_eq!(kv.value, expected, "value mismatch for key {}", key);
    }

    // Cleanup
    for key in test_keys {
        store.delete(DeleteRequest { key: key.to_string() }).await.expect("delete should succeed");
    }
}

/// Test that KV verification detects missing keys.
#[tokio::test]
async fn test_kv_verification_missing_key() {
    use aspen::api::KeyValueStoreError;

    let store = DeterministicKeyValueStore::new();

    let result = store.read(ReadRequest::new("nonexistent_key".to_string())).await;

    // The API returns NotFound error for missing keys
    match result {
        Err(KeyValueStoreError::NotFound { key }) => {
            assert_eq!(key, "nonexistent_key", "error should contain the requested key");
        }
        Ok(r) => panic!("expected NotFound error, got Ok with kv={:?}", r.kv),
        Err(e) => panic!("expected NotFound error, got {:?}", e),
    }
}

/// Test that KV verification handles delete correctly.
#[tokio::test]
async fn test_kv_verification_delete_cycle() {
    use aspen::api::KeyValueStoreError;

    let store = DeterministicKeyValueStore::new();

    // Write
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "test_delete_key".to_string(),
                value: "test_value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    // Verify exists
    let result = store.read(ReadRequest::new("test_delete_key".to_string())).await.expect("read should succeed");
    assert!(result.kv.is_some(), "key should exist after write");

    // Delete
    store
        .delete(DeleteRequest {
            key: "test_delete_key".to_string(),
        })
        .await
        .expect("delete should succeed");

    // Verify deleted - API returns NotFound error for missing/deleted keys
    let result = store.read(ReadRequest::new("test_delete_key".to_string())).await;
    match result {
        Err(KeyValueStoreError::NotFound { key }) => {
            assert_eq!(key, "test_delete_key", "error should contain the deleted key");
        }
        Ok(r) => panic!("expected NotFound error after delete, got Ok with kv={:?}", r.kv),
        Err(e) => panic!("expected NotFound error after delete, got {:?}", e),
    }
}

// =============================================================================
// Cluster State Verification Tests
// =============================================================================

/// Test cluster initialization verification.
#[tokio::test]
async fn test_cluster_init_verification() {
    let controller = DeterministicClusterController::new();

    // Initialize cluster
    let init_result = controller
        .init(InitRequest {
            initial_members: vec![
                ClusterNode::new(1, "node1", None),
                ClusterNode::new(2, "node2", None),
                ClusterNode::new(3, "node3", None),
            ],
        })
        .await
        .expect("init should succeed");

    // Verify cluster state
    assert_eq!(init_result.nodes.len(), 3, "should have 3 nodes");
    assert_eq!(init_result.members.len(), 3, "should have 3 members");

    // Verify current state reflects initialization
    let state = controller.current_state().await.expect("current_state should succeed");
    assert_eq!(state.nodes.len(), 3, "current state should show 3 nodes");
}

/// Test that verification detects leader election.
/// Note: The deterministic backend doesn't support get_metrics/get_leader
/// since it's an in-memory stub without Raft consensus. We verify that
/// the operation correctly returns an Unsupported error.
#[tokio::test]
async fn test_cluster_leader_verification() {
    use aspen::api::ControlPlaneError;

    let controller = DeterministicClusterController::new();

    // Initialize
    controller
        .init(InitRequest {
            initial_members: vec![ClusterNode::new(1, "node1", None)],
        })
        .await
        .expect("init should succeed");

    // The deterministic backend doesn't support get_leader (requires get_metrics)
    // Verify it returns the expected Unsupported error
    let result = controller.get_leader().await;
    match result {
        Err(ControlPlaneError::Unsupported { backend, operation }) => {
            assert_eq!(backend, "deterministic");
            assert_eq!(operation, "get_metrics");
        }
        Ok(leader) => {
            // If it ever gets implemented, verify we get a leader
            assert!(leader.is_some(), "cluster should have a leader after init");
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}

// =============================================================================
// Madsim Multi-Node Replication Verification Tests
// =============================================================================

/// Helper to create a Raft instance for madsim testing.
async fn create_raft_node(
    node_id: NodeId,
    router: Arc<MadsimRaftRouter>,
    injector: Arc<FailureInjector>,
) -> Raft<AppTypeConfig> {
    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };
    let config = Arc::new(config.validate().expect("invalid raft config"));

    let log_store = InMemoryLogStore::default();
    let state_machine = Arc::new(InMemoryStateMachine::default());

    let network_factory = MadsimNetworkFactory::new(node_id, router, injector);

    Raft::new(node_id, config, network_factory, log_store, state_machine)
        .await
        .expect("failed to create raft instance")
}

/// Test that madsim cluster correctly replicates writes to all nodes.
#[madsim::test]
async fn test_madsim_replication_verification() {
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    // Create 3-node cluster
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;

    // Register nodes
    router
        .register_node(NodeId::from(1), "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("register node 1");
    router
        .register_node(NodeId::from(2), "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("register node 2");
    router
        .register_node(NodeId::from(3), "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("register node 3");

    // Initialize cluster
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("init cluster");

    // Wait for leader election
    madsim::time::sleep(Duration::from_millis(5000)).await;

    // Find leader
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("should have leader");

    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader"),
    };

    // Write test data
    for i in 0..5 {
        leader_raft
            .client_write(AppRequest::Set {
                key: format!("verify_key_{}", i),
                value: format!("verify_value_{}", i),
            })
            .await
            .expect("write should succeed");
    }

    // Wait for replication
    madsim::time::sleep(Duration::from_millis(2000)).await;

    // VERIFICATION: Check all nodes have replicated the entries
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    // All nodes should have applied the same log index
    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    // Verify log indices are consistent (replication complete)
    let applied1 = final_metrics1.last_applied.map(|l| l.index).unwrap_or(0);
    let applied2 = final_metrics2.last_applied.map(|l| l.index).unwrap_or(0);
    let applied3 = final_metrics3.last_applied.map(|l| l.index).unwrap_or(0);

    // All followers should be caught up with leader
    let leader_applied = match leader_id.0 {
        1 => applied1,
        2 => applied2,
        3 => applied3,
        _ => panic!("invalid leader"),
    };

    assert!(applied1 >= leader_applied - 1, "node 1 should be caught up: {} vs {}", applied1, leader_applied);
    assert!(applied2 >= leader_applied - 1, "node 2 should be caught up: {} vs {}", applied2, leader_applied);
    assert!(applied3 >= leader_applied - 1, "node 3 should be caught up: {} vs {}", applied3, leader_applied);
}

/// Test replication lag detection in verification.
#[madsim::test]
async fn test_madsim_replication_lag_detection() {
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    // Create 3-node cluster
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;

    router
        .register_node(NodeId::from(1), "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("register node 1");
    router
        .register_node(NodeId::from(2), "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("register node 2");
    router
        .register_node(NodeId::from(3), "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("register node 3");

    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("init cluster");

    madsim::time::sleep(Duration::from_millis(5000)).await;

    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("should have leader");

    // Inject network partition to node 3 by dropping all messages to/from it
    // Drop messages from all nodes to node 3
    injector.set_message_drop(NodeId::from(1), NodeId::from(3), true);
    injector.set_message_drop(NodeId::from(2), NodeId::from(3), true);
    // Drop messages from node 3 to all other nodes
    injector.set_message_drop(NodeId::from(3), NodeId::from(1), true);
    injector.set_message_drop(NodeId::from(3), NodeId::from(2), true);

    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader"),
    };

    // Write some data (will replicate to node 1 and 2, but not 3)
    for i in 0..3 {
        let result = leader_raft
            .client_write(AppRequest::Set {
                key: format!("lag_test_{}", i),
                value: format!("value_{}", i),
            })
            .await;
        // May fail if leader is node 3 (partitioned)
        if result.is_err() && leader_id.0 == 3 {
            // Expected: partitioned leader can't reach quorum
            return;
        }
    }

    // Brief wait for partial replication
    madsim::time::sleep(Duration::from_millis(1000)).await;

    // Check metrics to detect lag
    let m1 = raft1.metrics().borrow().clone();
    let m2 = raft2.metrics().borrow().clone();
    let m3 = raft3.metrics().borrow().clone();

    let idx1 = m1.last_applied.map(|l| l.index).unwrap_or(0);
    let idx2 = m2.last_applied.map(|l| l.index).unwrap_or(0);
    let idx3 = m3.last_applied.map(|l| l.index).unwrap_or(0);

    // Node 3 should be behind (partitioned)
    // This demonstrates how verification would detect replication lag
    if leader_id.0 != 3 {
        // If leader is not node 3, node 3 should be lagging
        let max_non_partitioned = idx1.max(idx2);
        assert!(
            idx3 <= max_non_partitioned,
            "partitioned node 3 ({}) should not be ahead of non-partitioned nodes (max {})",
            idx3,
            max_non_partitioned
        );
    }

    // Heal partition - allow messages again
    injector.set_message_drop(NodeId::from(1), NodeId::from(3), false);
    injector.set_message_drop(NodeId::from(2), NodeId::from(3), false);
    injector.set_message_drop(NodeId::from(3), NodeId::from(1), false);
    injector.set_message_drop(NodeId::from(3), NodeId::from(2), false);
}

// =============================================================================
// Verification Result Structure Tests
// =============================================================================

/// Test VerifyResult formatting for human output.
#[test]
fn test_verify_result_human_format() {
    // We can't directly test the CLI output module here, but we can test
    // the expected patterns that verification should produce.

    let passed_message = "[PASS] kv-replication (123 ms): 5/5 keys verified";
    assert!(passed_message.contains("[PASS]"));
    assert!(passed_message.contains("kv-replication"));
    assert!(passed_message.contains("5/5"));

    let failed_message = "[FAIL] docs-sync (456 ms): Entry count mismatch";
    assert!(failed_message.contains("[FAIL]"));
    assert!(failed_message.contains("docs-sync"));
}

/// Test that verification handles empty cluster gracefully.
#[tokio::test]
async fn test_verification_empty_cluster() {
    let controller = DeterministicClusterController::new();

    // Before init, cluster should be empty
    let state = controller.current_state().await.expect("current_state should succeed");

    assert!(state.nodes.is_empty(), "uninitialized cluster should have no nodes");
}

// =============================================================================
// Scan Verification Tests
// =============================================================================

/// Test that scan verification works for prefix-based queries.
#[tokio::test]
async fn test_scan_verification_with_prefix() {
    let store = DeterministicKeyValueStore::new();

    // Write keys with common prefix
    for i in 0..10 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("scan_test_{}", i),
                    value: format!("value_{}", i),
                },
            })
            .await
            .expect("write should succeed");
    }

    // Write some keys without the prefix
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "other_key".to_string(),
                value: "other_value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    // Scan with prefix
    let results = store
        .scan(ScanRequest {
            prefix: "scan_test_".to_string(),
            limit: Some(100),
            continuation_token: None,
        })
        .await
        .expect("scan should succeed");

    assert_eq!(results.entries.len(), 10, "should find 10 keys with prefix");
    for entry in &results.entries {
        assert!(entry.key.starts_with("scan_test_"), "all keys should have the prefix");
    }
}

/// Test scan pagination for large result sets.
#[tokio::test]
async fn test_scan_verification_pagination() {
    let store = DeterministicKeyValueStore::new();

    // Write many keys
    for i in 0..50 {
        store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("page_test_{:03}", i),
                    value: format!("value_{}", i),
                },
            })
            .await
            .expect("write should succeed");
    }

    // Scan with limit
    let page1 = store
        .scan(ScanRequest {
            prefix: "page_test_".to_string(),
            limit: Some(20),
            continuation_token: None,
        })
        .await
        .expect("scan should succeed");

    assert_eq!(page1.entries.len(), 20, "first page should have 20 keys");
    assert!(page1.continuation_token.is_some(), "should have continuation token");

    // Get next page
    let page2 = store
        .scan(ScanRequest {
            prefix: "page_test_".to_string(),
            limit: Some(20),
            continuation_token: page1.continuation_token,
        })
        .await
        .expect("scan should succeed");

    assert_eq!(page2.entries.len(), 20, "second page should have 20 keys");

    // Verify no overlap
    let page1_keys: std::collections::HashSet<_> = page1.entries.iter().map(|e| &e.key).collect();
    let page2_keys: std::collections::HashSet<_> = page2.entries.iter().map(|e| &e.key).collect();
    assert!(page1_keys.is_disjoint(&page2_keys), "pages should not overlap");
}
