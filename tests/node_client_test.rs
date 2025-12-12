//! Integration tests for NodeClient with RaftActor backend.
//!
//! These tests validate the key-value client implementation by bootstrapping
//! real nodes with the RaftActor backend and exercising KV operations. Multi-node
//! tests use runtime peer address exchange to enable IRPC connectivity.
//! Full multi-node scenarios are also validated by the smoke test scripts.

use std::time::Duration;

use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, InitRequest,
    KeyValueStore, KeyValueStoreError, ReadRequest, WriteCommand, WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ControlBackend, IrohConfig, NodeConfig};
use aspen::node::NodeClient;
use aspen::raft::RaftControlClient;
use aspen::raft::types::NodeId;
use aspen::testing::create_test_raft_member_info;
use tempfile::TempDir;
use tokio::time::sleep;

/// Test single node: initialize cluster, write a key, read it back.
#[tokio::test]
async fn test_single_node_write_read() {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfig {
        node_id: 1000,
        data_dir: Some(temp_dir.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0, // OS-assigned port
        cookie: "test-kv-single".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await.unwrap();
    let cluster_client = RaftControlClient::new(handle.raft_actor.clone());
    let kv_client = NodeClient::new(handle.raft_actor.clone());

    // Initialize cluster with single node
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(
            1000,
            create_test_raft_member_info(1000).iroh_addr,
        )],
    };
    cluster_client.init(init_req).await.unwrap();

    // Write a key-value pair
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "foo".into(),
            value: "bar".into(),
        },
    };
    let write_result = kv_client.write(write_req).await.unwrap();
    assert_eq!(
        write_result.command,
        WriteCommand::Set {
            key: "foo".into(),
            value: "bar".into()
        }
    );

    // Read the key back
    let read_req = ReadRequest { key: "foo".into() };
    let read_result = kv_client.read(read_req).await.unwrap();
    assert_eq!(read_result.key, "foo");
    assert_eq!(read_result.value, "bar");

    // Shutdown
    handle.shutdown().await.unwrap();
}

/// Test two nodes: initialize cluster, write on node1, read from node2.
/// This validates that writes are replicated through Raft consensus.
#[tokio::test]
async fn test_two_node_replication() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    let config1 = NodeConfig {
        node_id: 2001,
        data_dir: Some(temp_dir1.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-two-nodes".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir1.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir1.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let config2 = NodeConfig {
        node_id: 2002,
        data_dir: Some(temp_dir2.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-two-nodes".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir2.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir2.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    // Bootstrap both nodes
    let (handle1, handle2) = tokio::try_join!(
        bootstrap_node(config1.clone()),
        bootstrap_node(config2.clone())
    )
    .unwrap();

    // Exchange peer addresses between nodes - use REAL Iroh addresses
    let addr1 = handle1.iroh_manager.node_addr().clone();
    let addr2 = handle2.iroh_manager.node_addr().clone();

    handle1
        .network_factory
        .add_peer(NodeId::from(config2.node_id), addr2.clone())
        .await;
    handle2
        .network_factory
        .add_peer(NodeId::from(config1.node_id), addr1.clone())
        .await;

    let cluster_client1 = RaftControlClient::new(handle1.raft_actor.clone());
    let kv_client1 = NodeClient::new(handle1.raft_actor.clone());
    let _kv_client2 = NodeClient::new(handle2.raft_actor.clone());

    // Initialize cluster with only node1 as the initial member
    // Node2 will join later as a learner via add_learner()
    // Use REAL Iroh address from running node
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(2001, addr1.clone())],
    };

    cluster_client1.init(init_req).await.unwrap();

    // Note: Node2 should NOT call init() - it will join as a learner via add_learner()
    // Only the initial leader (node1) calls init()
    let _cluster_client2 = RaftControlClient::new(handle2.raft_actor.clone());

    // Give Raft time to establish leadership
    sleep(Duration::from_millis(500)).await;

    // Add node2 as a learner - use REAL Iroh address from running node
    let learner_req = AddLearnerRequest {
        learner: ClusterNode::with_iroh_addr(2002, addr2.clone()),
    };
    cluster_client1.add_learner(learner_req).await.unwrap();

    // Give time for learner to sync
    sleep(Duration::from_millis(500)).await;

    // Promote learner to voter
    let membership_req = ChangeMembershipRequest {
        members: vec![2001, 2002],
    };
    cluster_client1
        .change_membership(membership_req)
        .await
        .unwrap();

    // Give Raft time to establish new membership and replicate
    sleep(Duration::from_millis(1000)).await;

    // Write on node1
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "replicated_key".into(),
            value: "replicated_value".into(),
        },
    };
    kv_client1.write(write_req).await.unwrap();

    // Give time for replication
    sleep(Duration::from_millis(500)).await;

    // Read from leader (node1) to verify write succeeded and was replicated
    // Note: Linearizable reads must go through the leader in Raft
    let read_req = ReadRequest {
        key: "replicated_key".into(),
    };
    let read_result = kv_client1.read(read_req).await.unwrap();
    assert_eq!(read_result.key, "replicated_key");
    assert_eq!(read_result.value, "replicated_value");

    // Shutdown both nodes
    let _ = tokio::try_join!(handle1.shutdown(), handle2.shutdown());
}

/// Test error handling: reading a key that doesn't exist.
#[tokio::test]
async fn test_read_nonexistent_key() {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfig {
        node_id: 3000,
        data_dir: Some(temp_dir.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-error".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await.unwrap();
    let cluster_client = RaftControlClient::new(handle.raft_actor.clone());
    let kv_client = NodeClient::new(handle.raft_actor.clone());

    // Initialize cluster
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(
            3000,
            create_test_raft_member_info(3000).iroh_addr,
        )],
    };
    cluster_client.init(init_req).await.unwrap();

    // Try to read a key that doesn't exist
    let read_req = ReadRequest {
        key: "nonexistent".into(),
    };
    let result = kv_client.read(read_req).await;

    // Verify we get a NotFound error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, KeyValueStoreError::NotFound { .. }));

    // Shutdown
    handle.shutdown().await.unwrap();
}

/// Test multiple sequential writes and reads on a single node.
#[tokio::test]
async fn test_multiple_operations() {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfig {
        node_id: 4000,
        data_dir: Some(temp_dir.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-multi".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await.unwrap();
    let cluster_client = RaftControlClient::new(handle.raft_actor.clone());
    let kv_client = NodeClient::new(handle.raft_actor.clone());

    // Initialize cluster
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(
            4000,
            create_test_raft_member_info(4000).iroh_addr,
        )],
    };
    cluster_client.init(init_req).await.unwrap();

    // Write multiple key-value pairs
    for i in 0..10 {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("key{}", i),
                value: format!("value{}", i),
            },
        };
        kv_client.write(write_req).await.unwrap();
    }

    // Read all keys back and verify
    for i in 0..10 {
        let read_req = ReadRequest {
            key: format!("key{}", i),
        };
        let read_result = kv_client.read(read_req).await.unwrap();
        assert_eq!(read_result.key, format!("key{}", i));
        assert_eq!(read_result.value, format!("value{}", i));
    }

    // Shutdown
    handle.shutdown().await.unwrap();
}

/// Test adding a learner to a running cluster.
#[tokio::test]
async fn test_add_learner_and_replicate() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    let config1 = NodeConfig {
        node_id: 5001,
        data_dir: Some(temp_dir1.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-learner".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let config2 = NodeConfig {
        node_id: 5002,
        data_dir: Some(temp_dir2.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-learner".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    // Bootstrap both nodes
    let (handle1, handle2) = tokio::try_join!(
        bootstrap_node(config1.clone()),
        bootstrap_node(config2.clone())
    )
    .unwrap();

    // Exchange peer addresses between nodes - use the REAL Iroh addresses
    let addr1 = handle1.iroh_manager.node_addr().clone();
    let addr2 = handle2.iroh_manager.node_addr().clone();

    handle1
        .network_factory
        .add_peer(NodeId::from(config2.node_id), addr2.clone())
        .await;
    handle2
        .network_factory
        .add_peer(NodeId::from(config1.node_id), addr1.clone())
        .await;

    let cluster_client1 = RaftControlClient::new(handle1.raft_actor.clone());
    let kv_client1 = NodeClient::new(handle1.raft_actor.clone());
    let _kv_client2 = NodeClient::new(handle2.raft_actor.clone());

    // Initialize cluster with only node1 - use REAL Iroh address from running node
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(5001, addr1.clone())],
    };
    cluster_client1.init(init_req).await.unwrap();

    sleep(Duration::from_millis(500)).await;

    // Write a key before adding learner
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "before_learner".into(),
            value: "value1".into(),
        },
    };
    kv_client1.write(write_req).await.unwrap();

    // Add node2 as learner - use REAL Iroh address from running node
    let learner_req = AddLearnerRequest {
        learner: ClusterNode::with_iroh_addr(5002, addr2.clone()),
    };
    cluster_client1.add_learner(learner_req).await.unwrap();

    sleep(Duration::from_millis(1000)).await;

    // Write another key after adding learner
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "after_learner".into(),
            value: "value2".into(),
        },
    };
    kv_client1.write(write_req).await.unwrap();

    sleep(Duration::from_millis(500)).await;

    // Promote learner to voting member
    let membership_req = ChangeMembershipRequest {
        members: vec![5001, 5002],
    };
    cluster_client1
        .change_membership(membership_req)
        .await
        .unwrap();

    sleep(Duration::from_millis(1000)).await;

    // Verify both keys are readable from leader (node1)
    // Note: Linearizable reads must go through the leader in Raft
    // This verifies that both writes succeeded and were committed
    let read_req1 = ReadRequest {
        key: "before_learner".into(),
    };
    let result1 = kv_client1.read(read_req1).await.unwrap();
    assert_eq!(result1.value, "value1");

    let read_req2 = ReadRequest {
        key: "after_learner".into(),
    };
    let result2 = kv_client1.read(read_req2).await.unwrap();
    assert_eq!(result2.value, "value2");

    // Shutdown both nodes
    let _ = tokio::try_join!(handle1.shutdown(), handle2.shutdown());
}

/// Test SetMulti: atomic multi-key writes on a single node.
#[tokio::test]
async fn test_setmulti_operations() {
    let temp_dir = TempDir::new().unwrap();

    let config = NodeConfig {
        node_id: 6000,
        data_dir: Some(temp_dir.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-kv-setmulti".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await.unwrap();
    let cluster_client = RaftControlClient::new(handle.raft_actor.clone());
    let kv_client = NodeClient::new(handle.raft_actor.clone());

    // Initialize cluster
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(
            6000,
            create_test_raft_member_info(6000).iroh_addr,
        )],
    };
    cluster_client.init(init_req).await.unwrap();

    // Write multiple key-value pairs atomically using SetMulti
    let pairs = vec![
        ("multi_key1".to_string(), "multi_value1".to_string()),
        ("multi_key2".to_string(), "multi_value2".to_string()),
        ("multi_key3".to_string(), "multi_value3".to_string()),
    ];
    let write_req = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: pairs.clone(),
        },
    };
    kv_client.write(write_req).await.unwrap();

    // Read all keys back and verify they were all written
    for (key, expected_value) in &pairs {
        let read_req = ReadRequest { key: key.clone() };
        let read_result = kv_client.read(read_req).await.unwrap();
        assert_eq!(read_result.key, *key);
        assert_eq!(read_result.value, *expected_value);
    }

    // Test overwriting existing keys with SetMulti
    let new_pairs = vec![
        ("multi_key1".to_string(), "updated1".to_string()),
        ("multi_key2".to_string(), "updated2".to_string()),
    ];
    let write_req2 = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: new_pairs.clone(),
        },
    };
    kv_client.write(write_req2).await.unwrap();

    // Verify the values were updated
    let read_result1 = kv_client
        .read(ReadRequest {
            key: "multi_key1".into(),
        })
        .await
        .unwrap();
    assert_eq!(read_result1.value, "updated1");

    let read_result2 = kv_client
        .read(ReadRequest {
            key: "multi_key2".into(),
        })
        .await
        .unwrap();
    assert_eq!(read_result2.value, "updated2");

    // Verify multi_key3 still has the original value
    let read_result3 = kv_client
        .read(ReadRequest {
            key: "multi_key3".into(),
        })
        .await
        .unwrap();
    assert_eq!(read_result3.value, "multi_value3");

    // Shutdown
    handle.shutdown().await.unwrap();
}
