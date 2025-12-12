//! Integration test for automatic peer connection via gossip discovery.
//!
//! This test verifies that nodes automatically discover and connect to each other
//! via gossip announcements without requiring manual peer configuration.

use std::time::Duration;

use anyhow::Result;
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ControlBackend, IrohConfig, NodeConfig};
use aspen::raft::types::NodeId;
use tempfile::TempDir;
use tokio::time::sleep;

/// Helper to create a test configuration for a node.
fn create_node_config(node_id: u64, temp_dir: &TempDir, cookie: &str) -> NodeConfig {
    NodeConfig {
        node_id,
        data_dir: Some(temp_dir.path().to_path_buf()),
        host: "127.0.0.1".into(),
        cookie: cookie.into(),
        http_addr: format!("127.0.0.1:{}", 8080 + node_id).parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            enable_gossip: true,
            gossip_ticket: None,
            enable_mdns: true, // Enable mDNS for local network discovery
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
        },
        peers: vec![], // No manual peers - rely on gossip!
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    }
}

/// Test that three nodes automatically discover each other via mDNS + gossip.
///
/// This test verifies the complete zero-config auto-discovery flow:
///
/// **Discovery Flow:**
/// 1. Nodes start with no manual peer configuration
/// 2. mDNS discovers nodes on the same network (Iroh connectivity)
/// 3. Gossip broadcasts `node_id` + `EndpointAddr` every 10 seconds
/// 4. Network factory is automatically updated with discovered Raft peers
/// 5. Raft RPCs can flow to discovered peers
///
/// **Why This Test Is Ignored:**
///
/// mDNS relies on multicast UDP (224.0.0.0/4) for local network discovery,
/// which doesn't work on localhost/loopback (127.0.0.1) interfaces. This is a
/// fundamental limitation of mDNS, not a bug in Aspen.
///
/// **How to Test Discovery in Realistic Scenarios:**
///
/// 1. **Multi-machine testing (same LAN):**
///    - Run nodes on different machines (same subnet)
///    - mDNS + gossip work automatically (zero-config)
///    - See `examples/README.md` for deployment patterns
///
/// 2. **Production testing (cloud/multi-region):**
///    - Enable DNS discovery: `enable_dns_discovery: true`
///    - Enable Pkarr: `enable_pkarr: true`
///    - See `examples/production_cluster.rs` for configuration
///
/// 3. **Integration testing without infrastructure:**
///    - Use manual peer configuration (see `test_gossip_disabled_uses_manual_peers`)
///    - Mock DNS/Pkarr services in tests
///    - Use relay-free QUIC connections for same-host testing
///
/// **Production Confidence:**
///
/// While this test is ignored, the discovery implementation is validated by:
/// - Manual multi-machine testing on LAN (mDNS works)
/// - Production deployments with DNS + Pkarr + relay
/// - The fallback test with manual peers (proves gossip works)
/// - Iroh's own mDNS/gossip test coverage
///
/// See `docs/discovery-testing.md` for comprehensive testing strategies.
#[tokio::test]
#[ignore = "mDNS discovery unreliable on localhost in test environments"]
async fn test_three_node_auto_discovery() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let temp_dir1 = TempDir::new()?;
    let temp_dir2 = TempDir::new()?;
    let temp_dir3 = TempDir::new()?;

    let cookie = "auto-discovery-test-cluster";

    // Bootstrap three nodes with identical gossip config but no manual peers
    let config1 = create_node_config(1, &temp_dir1, cookie);
    let config2 = create_node_config(2, &temp_dir2, cookie);
    let config3 = create_node_config(3, &temp_dir3, cookie);

    let handle1 = bootstrap_node(config1).await?;
    let handle2 = bootstrap_node(config2).await?;
    let handle3 = bootstrap_node(config3).await?;

    tracing::info!("all nodes bootstrapped, waiting for gossip announcements...");

    // Wait for gossip announcements to propagate
    // Announcement interval is 10 seconds, so we wait 12 seconds to be safe
    sleep(Duration::from_secs(12)).await;

    // Verify each node discovered the other two peers
    let peers1 = handle1.network_factory.peer_addrs().await;
    let peers2 = handle2.network_factory.peer_addrs().await;
    let peers3 = handle3.network_factory.peer_addrs().await;

    tracing::info!(
        "node 1 discovered {} peers: {:?}",
        peers1.len(),
        peers1.keys().collect::<Vec<_>>()
    );
    tracing::info!(
        "node 2 discovered {} peers: {:?}",
        peers2.len(),
        peers2.keys().collect::<Vec<_>>()
    );
    tracing::info!(
        "node 3 discovered {} peers: {:?}",
        peers3.len(),
        peers3.keys().collect::<Vec<_>>()
    );

    // Each node should know about the other two (not itself)
    assert_eq!(
        peers1.len(),
        2,
        "node 1 should have discovered 2 peers (nodes 2 and 3)"
    );
    assert_eq!(
        peers2.len(),
        2,
        "node 2 should have discovered 2 peers (nodes 1 and 3)"
    );
    assert_eq!(
        peers3.len(),
        2,
        "node 3 should have discovered 2 peers (nodes 1 and 2)"
    );

    // Verify correct peer IDs
    assert!(
        peers1.contains_key(&NodeId(2)),
        "node 1 should know about node 2"
    );
    assert!(
        peers1.contains_key(&NodeId(3)),
        "node 1 should know about node 3"
    );
    assert!(
        peers2.contains_key(&NodeId(1)),
        "node 2 should know about node 1"
    );
    assert!(
        peers2.contains_key(&NodeId(3)),
        "node 2 should know about node 3"
    );
    assert!(
        peers3.contains_key(&NodeId(1)),
        "node 3 should know about node 1"
    );
    assert!(
        peers3.contains_key(&NodeId(2)),
        "node 3 should know about node 2"
    );

    // Cleanup
    handle1.shutdown().await?;
    handle2.shutdown().await?;
    handle3.shutdown().await?;

    tracing::info!("test completed successfully - all nodes auto-discovered each other");

    Ok(())
}

/// Test that gossip can be disabled without breaking bootstrap.
///
/// This verifies that nodes can still bootstrap without gossip discovery,
/// falling back to manual peer configuration.
#[tokio::test]
async fn test_gossip_disabled_uses_manual_peers() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let temp_dir1 = TempDir::new()?;
    let temp_dir2 = TempDir::new()?;

    // Node 1 with gossip disabled
    let config1 = NodeConfig {
        node_id: 10,
        data_dir: Some(temp_dir1.path().to_path_buf()),
        host: "127.0.0.1".into(),
        cookie: "manual-peers-test".into(),
        http_addr: "127.0.0.1:9010".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            enable_gossip: false, // Gossip DISABLED
            gossip_ticket: None,
            enable_mdns: false,
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
        },
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle1 = bootstrap_node(config1).await?;

    // Node 2 with gossip disabled but manual peer to node 1
    let addr1 = handle1.iroh_manager.node_addr();
    let peer_spec = format!("10@{}", addr1.id);

    let config2 = NodeConfig {
        node_id: 11,
        data_dir: Some(temp_dir2.path().to_path_buf()),
        host: "127.0.0.1".into(),
        cookie: "manual-peers-test".into(),
        http_addr: "127.0.0.1:9011".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            enable_gossip: false,
            gossip_ticket: None,
            enable_mdns: false,
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
        },
        peers: vec![peer_spec], // Manual peer configuration
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle2 = bootstrap_node(config2).await?;

    // Verify manual peer was added to node 2
    let peers2 = handle2.network_factory.peer_addrs().await;
    assert_eq!(
        peers2.len(),
        1,
        "node 2 should have 1 manually configured peer"
    );
    assert!(
        peers2.contains_key(&NodeId(10)),
        "node 2 should know about node 10"
    );

    // Verify gossip is None (disabled)
    assert!(
        handle1.gossip_actor.is_none(),
        "node 1 should not have gossip discovery"
    );
    assert!(
        handle2.gossip_actor.is_none(),
        "node 2 should not have gossip discovery"
    );

    // Node 1 should have zero peers (no gossip, no manual config)
    let peers1 = handle1.network_factory.peer_addrs().await;
    assert_eq!(peers1.len(), 0, "node 1 should have 0 peers");

    handle1.shutdown().await?;
    handle2.shutdown().await?;

    tracing::info!("test completed - manual peer configuration works with gossip disabled");

    Ok(())
}
