//! Tests for RaftNode direct API implementation.
//!
//! These tests validate that the RaftNode properly implements the
//! ClusterController and KeyValueStore traits after the actor-to-direct
//! API migration.
//!
//! Tiger Style: Fixed timeouts, bounded operations, explicit error handling.

use std::time::Duration;

use aspen::api::AddLearnerRequest;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::ControlPlaneError;
use aspen::api::DeleteRequest;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::KeyValueStoreError;
use aspen::api::ReadRequest;
use aspen::api::ScanRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;

/// Test timeout for all operations.
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Helper to create a single-node cluster.
///
/// Note: mDNS is disabled for sandbox/CI compatibility (mDNS requires multicast
/// networking which is not available in Nix sandbox).
async fn setup_single_node(node_id: u64, temp_dir: &TempDir) -> anyhow::Result<aspen::node::Node> {
    let data_dir = temp_dir.path().join(format!("node-{}", node_id));
    // Use a unique test cookie to satisfy cookie validation
    let test_cookie = format!("test-cookie-{}", node_id);
    let node = NodeBuilder::new(NodeId(node_id), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie(test_cookie)
        .with_mdns(false) // Disable mDNS for sandbox/CI compatibility
        .start()
        .await?;
    Ok(node)
}

/// Helper to initialize a single-node cluster.
async fn init_single_node(node: &aspen::node::Node) -> anyhow::Result<()> {
    let raft_node = node.raft_node();
    let endpoint_addr = node.endpoint_addr();

    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(node.node_id().0, endpoint_addr)],
    };

    raft_node.init(init_request).await?;
    Ok(())
}

// =============================================================================
// ClusterController Trait Tests
// =============================================================================

/// Test init fails with empty members.
#[tokio::test]
async fn test_init_empty_members_fails() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(100, &temp_dir).await.unwrap();

    let raft_node = node.raft_node();
    let init_request = InitRequest {
        initial_members: vec![],
    };

    let result = raft_node.init(init_request).await;
    assert!(matches!(result, Err(ControlPlaneError::InvalidRequest { .. })));

    node.shutdown().await.unwrap();
}

/// Test successful single-node initialization.
#[tokio::test]
async fn test_init_single_node_success() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(101, &temp_dir).await.unwrap();

    let raft_node = node.raft_node();
    let endpoint_addr = node.endpoint_addr();

    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(101, endpoint_addr)],
    };

    let result = timeout(TEST_TIMEOUT, raft_node.init(init_request)).await.unwrap();
    assert!(result.is_ok(), "init failed: {:?}", result);

    let state = result.unwrap();
    assert_eq!(state.members, vec![101]);
    assert_eq!(state.nodes.len(), 1);
    assert!(state.learners.is_empty());

    node.shutdown().await.unwrap();
}

/// Test current_state returns correct information after init.
#[tokio::test]
async fn test_current_state_after_init() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(102, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    let raft_node = node.raft_node();
    let state = timeout(TEST_TIMEOUT, raft_node.current_state()).await.unwrap().unwrap();

    assert_eq!(state.members, vec![102]);
    assert_eq!(state.nodes.len(), 1);
    assert_eq!(state.nodes[0].id, 102);

    node.shutdown().await.unwrap();
}

/// Test current_state fails before initialization.
#[tokio::test]
async fn test_current_state_before_init_fails() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(103, &temp_dir).await.unwrap();

    let raft_node = node.raft_node();
    let result = raft_node.current_state().await;

    assert!(matches!(result, Err(ControlPlaneError::NotInitialized)));

    node.shutdown().await.unwrap();
}

/// Test get_leader returns a leader after initialization.
#[tokio::test]
async fn test_get_leader_after_init() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(104, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for election to complete
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();
    let leader = timeout(TEST_TIMEOUT, raft_node.get_leader()).await.unwrap().unwrap();

    // In a single-node cluster, the node should be the leader
    assert_eq!(leader, Some(104));

    node.shutdown().await.unwrap();
}

/// Test get_metrics returns valid metrics.
#[tokio::test]
async fn test_get_metrics_returns_valid_data() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(105, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();
    let metrics = timeout(TEST_TIMEOUT, raft_node.get_metrics()).await.unwrap().unwrap();

    // Check basic metrics properties
    assert_eq!(metrics.id, 105);
    assert!(metrics.current_term > 0);
    // The node should be leader in single-node cluster
    assert!(metrics.state.is_leader(), "expected leader state, got {:?}", metrics.state);

    node.shutdown().await.unwrap();
}

/// Test add_learner without init fails.
#[tokio::test]
async fn test_add_learner_before_init_fails() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(106, &temp_dir).await.unwrap();

    let raft_node = node.raft_node();
    let learner = ClusterNode::with_iroh_addr(200, node.endpoint_addr());
    let request = AddLearnerRequest { learner };

    let result = raft_node.add_learner(request).await;
    assert!(matches!(result, Err(ControlPlaneError::NotInitialized)));

    node.shutdown().await.unwrap();
}

/// Test change_membership without init fails.
#[tokio::test]
async fn test_change_membership_before_init_fails() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(107, &temp_dir).await.unwrap();

    let raft_node = node.raft_node();
    let request = ChangeMembershipRequest {
        members: vec![107, 200],
    };

    let result = raft_node.change_membership(request).await;
    assert!(matches!(result, Err(ControlPlaneError::NotInitialized)));

    node.shutdown().await.unwrap();
}

/// Test change_membership with empty members fails.
#[tokio::test]
async fn test_change_membership_empty_members_fails() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(108, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();
    let request = ChangeMembershipRequest { members: vec![] };

    let result = raft_node.change_membership(request).await;
    assert!(matches!(result, Err(ControlPlaneError::InvalidRequest { .. })));

    node.shutdown().await.unwrap();
}

// =============================================================================
// KeyValueStore Trait Tests
// =============================================================================

/// Test write before initialization fails.
#[tokio::test]
async fn test_write_before_init_fails() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(200, &temp_dir).await.unwrap();

    let raft_node = node.raft_node();
    let request = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        },
    };

    let result = raft_node.write(request).await;
    assert!(matches!(result, Err(KeyValueStoreError::Failed { .. })));

    node.shutdown().await.unwrap();
}

/// Test successful write and read cycle.
#[tokio::test]
async fn test_write_read_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(201, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();

    // Write a key
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "mykey".to_string(),
            value: "myvalue".to_string(),
        },
    };

    let write_result = timeout(TEST_TIMEOUT, raft_node.write(write_request)).await.unwrap();
    assert!(write_result.is_ok(), "write failed: {:?}", write_result);

    // Read the key back
    let read_request = ReadRequest::new("mykey".to_string());

    let read_result = timeout(TEST_TIMEOUT, raft_node.read(read_request)).await.unwrap();
    assert!(read_result.is_ok(), "read failed: {:?}", read_result);

    let result = read_result.unwrap();
    let kv = result.kv.unwrap();
    assert_eq!(kv.key, "mykey");
    assert_eq!(kv.value, "myvalue");

    node.shutdown().await.unwrap();
}

/// Test reading non-existent key returns NotFound.
#[tokio::test]
async fn test_read_nonexistent_key_returns_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(202, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();
    let read_request = ReadRequest::new("nonexistent".to_string());

    let result = raft_node.read(read_request).await;
    assert!(
        matches!(&result, Err(KeyValueStoreError::NotFound { key }) if key == "nonexistent"),
        "expected NotFound, got {:?}",
        result
    );

    node.shutdown().await.unwrap();
}

/// Test delete operation.
#[tokio::test]
async fn test_delete_existing_key() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(203, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();

    // Write a key first
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "to_delete".to_string(),
            value: "temp".to_string(),
        },
    };
    raft_node.write(write_request).await.unwrap();

    // Delete the key
    let delete_request = DeleteRequest {
        key: "to_delete".to_string(),
    };
    let delete_result = timeout(TEST_TIMEOUT, raft_node.delete(delete_request)).await.unwrap();
    assert!(delete_result.is_ok(), "delete failed: {:?}", delete_result);

    let result = delete_result.unwrap();
    assert_eq!(result.key, "to_delete");
    assert!(result.is_deleted);

    // Verify key is gone
    let read_request = ReadRequest::new("to_delete".to_string());
    let read_result = raft_node.read(read_request).await;
    assert!(matches!(read_result, Err(KeyValueStoreError::NotFound { .. })));

    node.shutdown().await.unwrap();
}

/// Test SetMulti writes multiple keys atomically.
#[tokio::test]
async fn test_set_multi_writes_multiple_keys() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(204, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();

    // Write multiple keys
    let write_request = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: vec![
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
                ("key3".to_string(), "value3".to_string()),
            ],
        },
    };

    let write_result = timeout(TEST_TIMEOUT, raft_node.write(write_request)).await.unwrap();
    assert!(write_result.is_ok(), "write failed: {:?}", write_result);

    // Verify all keys were written
    for i in 1..=3 {
        let read_request = ReadRequest::new(format!("key{}", i));
        let result = raft_node.read(read_request).await.unwrap();
        assert_eq!(result.kv.unwrap().value, format!("value{}", i));
    }

    node.shutdown().await.unwrap();
}

/// Test scan operation returns matching keys.
#[tokio::test]
async fn test_scan_with_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(205, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();

    // Write some keys with different prefixes
    let write_request = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: vec![
                ("users/1".to_string(), "alice".to_string()),
                ("users/2".to_string(), "bob".to_string()),
                ("config/db".to_string(), "postgres".to_string()),
                ("users/3".to_string(), "charlie".to_string()),
            ],
        },
    };
    raft_node.write(write_request).await.unwrap();

    // Scan for users
    let scan_request = ScanRequest {
        prefix: "users/".to_string(),
        limit: None,
        continuation_token: None,
    };

    let scan_result = timeout(TEST_TIMEOUT, raft_node.scan(scan_request)).await.unwrap();
    assert!(scan_result.is_ok(), "scan failed: {:?}", scan_result);

    let result = scan_result.unwrap();
    assert_eq!(result.count, 3);
    assert!(!result.is_truncated);
    assert!(result.continuation_token.is_none());

    // Verify entries are sorted
    let keys: Vec<&str> = result.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["users/1", "users/2", "users/3"]);

    node.shutdown().await.unwrap();
}

/// Test scan with limit and pagination.
#[tokio::test]
async fn test_scan_pagination() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(206, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();

    // Write 5 keys
    let write_request = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: (1..=5).map(|i| (format!("item/{:02}", i), format!("data{}", i))).collect(),
        },
    };
    raft_node.write(write_request).await.unwrap();

    // Scan with limit of 2
    let scan_request = ScanRequest {
        prefix: "item/".to_string(),
        limit: Some(2),
        continuation_token: None,
    };

    let scan_result = raft_node.scan(scan_request).await.unwrap();
    assert_eq!(scan_result.count, 2);
    assert!(scan_result.is_truncated);
    assert!(scan_result.continuation_token.is_some());

    // Get next page
    let scan_request2 = ScanRequest {
        prefix: "item/".to_string(),
        limit: Some(2),
        continuation_token: scan_result.continuation_token,
    };

    let scan_result2 = raft_node.scan(scan_request2).await.unwrap();
    assert_eq!(scan_result2.count, 2);
    assert!(scan_result2.is_truncated);

    // Get final page
    let scan_request3 = ScanRequest {
        prefix: "item/".to_string(),
        limit: Some(2),
        continuation_token: scan_result2.continuation_token,
    };

    let scan_result3 = raft_node.scan(scan_request3).await.unwrap();
    assert_eq!(scan_result3.count, 1);
    assert!(!scan_result3.is_truncated);
    assert!(scan_result3.continuation_token.is_none());

    node.shutdown().await.unwrap();
}

/// Test overwriting an existing key.
#[tokio::test]
async fn test_overwrite_existing_key() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(207, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let raft_node = node.raft_node();

    // Write initial value
    let write1 = WriteRequest {
        command: WriteCommand::Set {
            key: "key".to_string(),
            value: "value1".to_string(),
        },
    };
    raft_node.write(write1).await.unwrap();

    // Verify initial value
    let read1 = raft_node.read(ReadRequest::new("key".to_string())).await.unwrap();
    assert_eq!(read1.kv.unwrap().value, "value1");

    // Overwrite with new value
    let write2 = WriteRequest {
        command: WriteCommand::Set {
            key: "key".to_string(),
            value: "value2".to_string(),
        },
    };
    raft_node.write(write2).await.unwrap();

    // Verify new value
    let read2 = raft_node.read(ReadRequest::new("key".to_string())).await.unwrap();
    assert_eq!(read2.kv.unwrap().value, "value2");

    node.shutdown().await.unwrap();
}

// =============================================================================
// Health Monitoring Tests
// =============================================================================

/// Test RaftNodeHealth basic functionality.
#[tokio::test]
async fn test_health_monitor_reports_healthy() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(300, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let handle = node.handle();
    let health_monitor = handle.shutdown.health_monitor.clone();

    let is_healthy = health_monitor.is_healthy().await;
    assert!(is_healthy, "node should be healthy after initialization");

    let status = health_monitor.status().await;
    assert!(status.is_healthy);
    assert!(status.has_membership);
    assert!(!status.is_shutdown);
    assert!(status.leader.is_some());

    node.shutdown().await.unwrap();
}

/// Test health status reflects node state.
#[tokio::test]
async fn test_health_status_reflects_state() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_single_node(301, &temp_dir).await.unwrap();
    init_single_node(&node).await.unwrap();

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;

    let handle = node.handle();
    let health_monitor = handle.shutdown.health_monitor.clone();

    let status = health_monitor.status().await;

    // Single-node cluster should be leader
    assert!(status.state.is_leader(), "expected leader state, got {:?}", status.state);
    assert_eq!(status.leader, Some(301));
    assert_eq!(status.consecutive_failures, 0);

    node.shutdown().await.unwrap();
}
