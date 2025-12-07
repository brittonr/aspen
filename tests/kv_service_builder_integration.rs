//! Integration tests for KvServiceBuilder.
//!
//! These tests validate the production API for programmatic node startup,
//! covering lifecycle management, configuration options, and multi-node clusters.

use std::time::Duration;

use aspen::kv::{KvServiceBuilder, NodeId};
use aspen::raft::storage::StorageBackend;
use tempfile::TempDir;
use tokio::time::sleep;

/// Test basic service lifecycle: start and shutdown.
#[tokio::test]
async fn test_service_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-1");

    let service = KvServiceBuilder::new(NodeId(1), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service");

    // Verify node metadata
    assert_eq!(service.node_id(), NodeId(1));
    assert!(service.data_dir().to_str().unwrap().contains("node-1"));

    // Verify endpoint address is available
    let endpoint = service.endpoint_addr();
    assert!(!endpoint.id.to_string().is_empty());

    // Shutdown should succeed
    service.shutdown().await.expect("failed to shutdown");
}

/// Test service with custom storage backend.
#[tokio::test]
async fn test_custom_storage_backend() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-2");

    let service = KvServiceBuilder::new(NodeId(2), &data_dir)
        .with_storage(StorageBackend::Sqlite)
        .start()
        .await
        .expect("failed to start service");

    assert_eq!(service.node_id(), NodeId(2));
    service.shutdown().await.expect("failed to shutdown");
}

/// Test service with gossip discovery enabled.
#[tokio::test]
async fn test_gossip_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-3");

    let service = KvServiceBuilder::new(NodeId(3), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_gossip(true)
        .start()
        .await
        .expect("failed to start service");

    assert_eq!(service.node_id(), NodeId(3));
    service.shutdown().await.expect("failed to shutdown");
}

/// Test service with mDNS discovery enabled.
#[tokio::test]
async fn test_mdns_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-4");

    let service = KvServiceBuilder::new(NodeId(4), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_mdns(true)
        .start()
        .await
        .expect("failed to start service");

    assert_eq!(service.node_id(), NodeId(4));
    service.shutdown().await.expect("failed to shutdown");
}

/// Test service with custom Raft timeouts.
#[tokio::test]
async fn test_custom_raft_timeouts() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-5");

    let service = KvServiceBuilder::new(NodeId(5), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1500, 3000)
        .start()
        .await
        .expect("failed to start service");

    assert_eq!(service.node_id(), NodeId(5));
    service.shutdown().await.expect("failed to shutdown");
}

/// Test creating a KV client from the service.
#[tokio::test]
async fn test_client_creation() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-6");

    let service = KvServiceBuilder::new(NodeId(6), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service");

    // Client creation should succeed
    let _client = service.client();

    service.shutdown().await.expect("failed to shutdown");
}

/// Test accessing Raft core for direct operations.
#[tokio::test]
async fn test_raft_core_access() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-7");

    let service = KvServiceBuilder::new(NodeId(7), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service");

    // Access Raft core
    let raft_core = service.raft_core();
    let metrics = raft_core.metrics().borrow().clone();

    // Should have basic Raft state (follower, leader, or learner are all valid initial states)
    assert!(
        metrics.state.is_follower() || metrics.state.is_leader() || metrics.state.is_learner(),
        "unexpected Raft state: {:?}",
        metrics.state
    );

    service.shutdown().await.expect("failed to shutdown");
}

/// Test accessing bootstrap handle for advanced operations.
#[tokio::test]
async fn test_bootstrap_handle_access() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-8");

    let service = KvServiceBuilder::new(NodeId(8), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service");

    // Access bootstrap handle
    let handle = service.handle();
    assert_eq!(handle.config.node_id, 8);

    service.shutdown().await.expect("failed to shutdown");
}

/// Test builder pattern with all options.
#[tokio::test]
async fn test_builder_all_options() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-9");

    let service = KvServiceBuilder::new(NodeId(9), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_peers(vec![])
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(1000)
        .with_election_timeout_ms(3000, 6000)
        .start()
        .await
        .expect("failed to start service");

    assert_eq!(service.node_id(), NodeId(9));
    service.shutdown().await.expect("failed to shutdown");
}

/// Test multiple services can coexist.
#[tokio::test]
async fn test_multiple_services() {
    let temp_dir = TempDir::new().unwrap();

    let service1 = KvServiceBuilder::new(NodeId(10), temp_dir.path().join("node-10"))
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service1");

    let service2 = KvServiceBuilder::new(NodeId(11), temp_dir.path().join("node-11"))
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service2");

    assert_eq!(service1.node_id(), NodeId(10));
    assert_eq!(service2.node_id(), NodeId(11));
    assert_ne!(service1.endpoint_addr(), service2.endpoint_addr());

    service1
        .shutdown()
        .await
        .expect("failed to shutdown service1");
    service2
        .shutdown()
        .await
        .expect("failed to shutdown service2");
}

/// Test service restart (shutdown and recreate).
#[tokio::test]
async fn test_service_restart_inmemory() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-12");

    // Start first instance
    let service1 = KvServiceBuilder::new(NodeId(12), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service1");

    let endpoint1 = service1.endpoint_addr();
    service1
        .shutdown()
        .await
        .expect("failed to shutdown service1");

    // Brief delay before restart
    sleep(Duration::from_millis(100)).await;

    // Start second instance (InMemory means fresh state)
    let service2 = KvServiceBuilder::new(NodeId(12), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .start()
        .await
        .expect("failed to start service2");

    let endpoint2 = service2.endpoint_addr();

    // Endpoint will be different (new instance)
    assert_ne!(endpoint1, endpoint2);

    service2
        .shutdown()
        .await
        .expect("failed to shutdown service2");
}

/// Test rapid start/shutdown cycles.
#[tokio::test]
async fn test_rapid_lifecycle_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-13");

    for i in 0..3 {
        let service = KvServiceBuilder::new(NodeId(13), &data_dir)
            .with_storage(StorageBackend::InMemory)
            .start()
            .await
            .unwrap_or_else(|e| panic!("failed to start service on cycle {}: {}", i, e));

        assert_eq!(service.node_id(), NodeId(13));

        service
            .shutdown()
            .await
            .unwrap_or_else(|e| panic!("failed to shutdown service on cycle {}: {}", i, e));

        // Small delay between cycles
        sleep(Duration::from_millis(50)).await;
    }
}

/// Test service with empty peer list.
#[tokio::test]
async fn test_peer_configuration() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("node-14");

    // Start with empty peer list (peers can be added dynamically later)
    let peers = vec![];

    let service = KvServiceBuilder::new(NodeId(14), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_peers(peers)
        .start()
        .await
        .expect("failed to start service");

    assert_eq!(service.node_id(), NodeId(14));
    service.shutdown().await.expect("failed to shutdown");
}
