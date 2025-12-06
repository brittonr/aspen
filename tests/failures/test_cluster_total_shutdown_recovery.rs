//! Test: Cluster Total Shutdown and Recovery
//!
//! Validates that a cluster can gracefully shutdown and restart cleanly.
//!
//! # Test Strategy
//!
//! With in-memory storage (current implementation):
//! 1. Start 3-node cluster
//! 2. Write data and verify replication
//! 3. Gracefully shutdown all nodes
//! 4. Restart all nodes
//! 5. Verify cluster can be re-initialized (data lost due to in-memory storage)
//!
//! With persistent storage (future redb implementation):
//! - Data should be recovered after restart
//! - No re-initialization needed
//! - Log indices and membership preserved
//!
//! # Tiger Style Compliance
//!
//! - Fixed cluster size: 3 nodes
//! - Explicit shutdown order: reverse of startup
//! - Bounded timeouts: 500ms for initialization, 200ms for replication

use std::time::Duration;

use aspen::api::{
    ClusterController, ClusterNode, InitRequest, KeyValueStore, ReadRequest, WriteCommand,
    WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::RaftControlClient;

/// Test that cluster shuts down gracefully and can be restarted cleanly.
///
/// Note: With in-memory storage, data is lost on shutdown. This test validates
/// clean shutdown/restart paths. With redb persistence (future), data recovery
/// would also be tested.
#[tokio::test]
#[ignore] // Requires peer discovery setup for multi-node initialization
async fn test_cluster_total_shutdown_and_restart() -> anyhow::Result<()> {
    // Phase 1: Start and populate cluster
    let temp_dir1 = tempfile::tempdir()?;
    let temp_dir2 = tempfile::tempdir()?;
    let temp_dir3 = tempfile::tempdir()?;

    let mut handles = Vec::new();

    // Start 3 nodes
    for (node_id, data_dir) in [
        (1, temp_dir1.path()),
        (2, temp_dir2.path()),
        (3, temp_dir3.path()),
    ] {
        let config = ClusterBootstrapConfig {
            node_id,
            control_backend: ControlBackend::RaftActor,
            host: "127.0.0.1".to_string(),
            http_addr: format!("127.0.0.1:{}", 46000 + node_id as u16).parse()?,
            ractor_port: 0,
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "shutdown-recovery-test".to_string(),
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

        let handle = bootstrap_node(config).await?;
        handles.push(handle);
    }

    // Initialize cluster on node 1
    let cluster = RaftControlClient::new(handles[0].raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![
            ClusterNode::new(1, "127.0.0.1:26000", Some("iroh://placeholder1".into())),
            ClusterNode::new(2, "127.0.0.1:26001", Some("iroh://placeholder2".into())),
            ClusterNode::new(3, "127.0.0.1:26002", Some("iroh://placeholder3".into())),
        ],
    };
    cluster.init(init_req).await?;

    // Wait for leader election
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Write test data
    let kv = KvClient::new(handles[0].raft_actor.clone());
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "recovery-test".to_string(),
            value: "data-before-shutdown".to_string(),
        },
    };
    kv.write(write_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify data is readable
    let read_req = ReadRequest {
        key: "recovery-test".to_string(),
    };
    let result = kv.read(read_req).await?;
    assert_eq!(result.value, "data-before-shutdown".to_string());

    // Phase 2: Graceful shutdown of all nodes
    for handle in handles {
        handle.shutdown().await?;
    }

    // Wait for full shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Phase 3: Restart all nodes
    let mut new_handles = Vec::new();

    for (node_id, data_dir) in [
        (1, temp_dir1.path()),
        (2, temp_dir2.path()),
        (3, temp_dir3.path()),
    ] {
        let config = ClusterBootstrapConfig {
            node_id,
            control_backend: ControlBackend::RaftActor,
            host: "127.0.0.1".to_string(),
            http_addr: format!("127.0.0.1:{}", 46000 + node_id as u16).parse()?,
            ractor_port: 0,
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "shutdown-recovery-test".to_string(),
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

        let handle = bootstrap_node(config).await?;
        new_handles.push(handle);
    }

    // Phase 4: Re-initialize cluster (in-memory storage loses state)
    let cluster_new = RaftControlClient::new(new_handles[0].raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![
            ClusterNode::new(1, "127.0.0.1:26000", Some("iroh://placeholder1".into())),
            ClusterNode::new(2, "127.0.0.1:26001", Some("iroh://placeholder2".into())),
            ClusterNode::new(3, "127.0.0.1:26002", Some("iroh://placeholder3".into())),
        ],
    };
    cluster_new.init(init_req).await?;

    // Wait for leader election
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Phase 5: Verify cluster is operational
    let kv_new = KvClient::new(new_handles[0].raft_actor.clone());

    // With in-memory storage, old data is lost
    let read_req = ReadRequest {
        key: "recovery-test".to_string(),
    };
    // Expect read to fail (key not found) since in-memory storage lost data
    assert!(
        kv_new.read(read_req).await.is_err(),
        "With in-memory storage, data is lost after restart"
    );

    // But we can write new data
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "new-data".to_string(),
            value: "after-restart".to_string(),
        },
    };
    kv_new.write(write_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let read_req = ReadRequest {
        key: "new-data".to_string(),
    };
    let result = kv_new.read(read_req).await?;
    assert_eq!(result.value, "after-restart".to_string());

    // Cleanup
    for handle in new_handles {
        handle.shutdown().await?;
    }

    Ok(())
}

/// Test that a single node can shutdown and restart independently.
#[tokio::test]
async fn test_single_node_restart() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Start node
    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:0".parse()?,
        ractor_port: 0,
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "single-node-restart".to_string(),
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

    let handle = bootstrap_node(config.clone()).await?;

    // Initialize single-node cluster
    let cluster = RaftControlClient::new(handle.raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };
    cluster.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Write data
    let kv = KvClient::new(handle.raft_actor.clone());
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        },
    };
    kv.write(write_req).await?;

    // Shutdown
    handle.shutdown().await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Restart
    let new_handle = bootstrap_node(config).await?;

    // Re-initialize (in-memory storage lost state)
    let cluster_new = RaftControlClient::new(new_handle.raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };
    cluster_new.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify node is operational
    let kv_new = KvClient::new(new_handle.raft_actor.clone());
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "new-test".to_string(),
            value: "new-value".to_string(),
        },
    };
    kv_new.write(write_req).await?;

    let read_req = ReadRequest {
        key: "new-test".to_string(),
    };
    let result = kv_new.read(read_req).await?;
    assert_eq!(result.value, "new-value".to_string());

    // Cleanup
    new_handle.shutdown().await?;

    Ok(())
}

/// SQLite variant: Test that a single node can shutdown and restart with SQLite backend.
#[tokio::test]
async fn test_single_node_restart_sqlite() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Start node
    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:0".parse()?,
        ractor_port: 0,
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "single-node-restart-sqlite".to_string(),
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

    let handle = bootstrap_node(config.clone()).await?;

    // Initialize single-node cluster
    let cluster = RaftControlClient::new(handle.raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };
    cluster.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Write data
    let kv = KvClient::new(handle.raft_actor.clone());
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        },
    };
    kv.write(write_req).await?;

    // Shutdown
    handle.shutdown().await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Restart
    let new_handle = bootstrap_node(config).await?;

    // Re-initialize (SQLite storage persists state)
    let cluster_new = RaftControlClient::new(new_handle.raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };
    cluster_new.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify node is operational
    let kv_new = KvClient::new(new_handle.raft_actor.clone());
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "new-test".to_string(),
            value: "new-value".to_string(),
        },
    };
    kv_new.write(write_req).await?;

    let read_req = ReadRequest {
        key: "new-test".to_string(),
    };
    let result = kv_new.read(read_req).await?;
    assert_eq!(result.value, "new-value".to_string());

    // Cleanup
    new_handle.shutdown().await?;

    Ok(())
}
