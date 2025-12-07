///! Test: Cluster Total Shutdown and Recovery
///!
///! Validates that a cluster can gracefully shutdown and restart cleanly.
///!
///! # Test Strategy
///!
///! With in-memory storage (current implementation):
///! 1. Start 3-node cluster
///! 2. Write data and verify replication
///! 3. Gracefully shutdown all nodes
///! 4. Restart all nodes
///! 5. Verify cluster can be re-initialized (data lost due to in-memory storage)
///!
///! With persistent storage (future redb implementation):
///! - Data should be recovered after restart
///! - No re-initialization needed
///! - Log indices and membership preserved
///!
///! # Tiger Style Compliance
///!
///! - Fixed cluster size: 3 nodes
///! - Explicit shutdown order: reverse of startup
///! - Bounded timeouts: 500ms for initialization, 200ms for replication

use std::collections::BTreeSet;
use std::time::Duration;

use aspen::api::{KeyValueStore, ReadRequest, WriteRequest, WriteCommand};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend};
use aspen::kv::client::KvClient;

/// Test that cluster shuts down gracefully and can be restarted cleanly.
///
/// Note: With in-memory storage, data is lost on shutdown. This test validates
/// clean shutdown/restart paths. With redb persistence (future), data recovery
/// would also be tested.
#[tokio::test]
#[ignore] // TODO: Fix initialization and API usage to match working test patterns
async fn test_cluster_total_shutdown_and_restart() -> anyhow::Result<()> {
    // Phase 1: Start and populate cluster
    let temp_dir1 = tempfile::tempdir()?;
    let temp_dir2 = tempfile::tempdir()?;
    let temp_dir3 = tempfile::tempdir()?;

    let mut handles = Vec::new();

    // Start 3 nodes
    for (node_id, data_dir) in [(1, temp_dir1.path()), (2, temp_dir2.path()), (3, temp_dir3.path())] {
        let config = ClusterBootstrapConfig {
            node_id,
            control_backend: ControlBackend::Deterministic,
            host: "127.0.0.1".to_string(),

            ractor_port: 46000 + node_id as u16,
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "shutdown-recovery-test".to_string(),
            ..Default::default()
        };

        let handle = bootstrap_node(config).await?;
        handles.push(handle);
    }

    // Initialize cluster on node 1
    let members = BTreeSet::from([1, 2, 3]);
    handles[0]
        .raft_actor
        .cast(aspen::raft::RaftActorMessage::InitCluster(InitRequest { initial_members: members }, answer_port))?;

    tokio::time::sleep(Duration::from_millis(500)).await;

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
    assert_eq!(result.value, Some("data-before-shutdown".to_string()));

    // Phase 2: Graceful shutdown of all nodes
    for handle in handles {
        handle.shutdown().await?;
    }

    // Wait for full shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Phase 3: Restart all nodes
    let mut new_handles = Vec::new();

    for (node_id, data_dir) in [(1, temp_dir1.path()), (2, temp_dir2.path()), (3, temp_dir3.path())] {
        let config = ClusterBootstrapConfig {
            node_id,
            control_backend: ControlBackend::Deterministic,
            host: "127.0.0.1".to_string(),

            ractor_port: 46000 + node_id as u16,
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "shutdown-recovery-test".to_string(),
            ..Default::default()
        };

        let handle = bootstrap_node(config).await?;
        new_handles.push(handle);
    }

    // Phase 4: Verify cluster can be re-initialized
    // Note: With in-memory storage, we need to re-init. With redb, the cluster
    // would recover its previous state automatically.
    let members = BTreeSet::from([1, 2, 3]);
    new_handles[0]
        .raft_actor
        .cast(aspen::raft::RaftActorMessage::InitCluster(InitRequest { initial_members: members }, answer_port))?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Phase 5: Verify cluster is operational
    let kv_new = KvClient::new(new_handles[0].raft_actor.clone());

    // With in-memory storage, old data is lost
    let read_req = ReadRequest {
        key: "recovery-test".to_string(),
    };
    let result = kv_new.read(read_req).await?;
    assert_eq!(
        result.value, None,
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
    assert_eq!(result.value, Some("after-restart".to_string()));

    // Cleanup
    for handle in new_handles {
        handle.shutdown().await?;
    }

    Ok(())
}

/// Test that a single node can shutdown and restart independently.
#[tokio::test]
#[ignore] // TODO: Fix initialization and API usage to match working test patterns
async fn test_single_node_restart() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Start node
    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::Deterministic,
        host: "127.0.0.1".to_string(),

        ractor_port: 56000,
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "single-node-restart".to_string(),
        ..Default::default()
    };

    let handle = bootstrap_node(config.clone()).await?;

    // Initialize single-node cluster
    handle
        .raft_actor
        .cast(aspen::raft::RaftActorMessage::InitCluster(InitRequest {
            members: BTreeSet::from([1]),
        })?;

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

    // Re-initialize
    new_handle
        .raft_actor
        .cast(aspen::raft::RaftActorMessage::InitCluster(InitRequest {
            members: BTreeSet::from([1]),
        })?;

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
    assert_eq!(result.value, Some("new-value".to_string()));

    // Cleanup
    new_handle.shutdown().await?;

    Ok(())
}
