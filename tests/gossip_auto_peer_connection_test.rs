//! Integration test for automatic peer connection via gossip discovery.
//!
//! This test verifies that nodes automatically discover and connect to each other
//! via gossip announcements without requiring manual peer configuration.

use std::time::Duration;

use anyhow::Result;
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use tempfile::TempDir;
use tokio::time::sleep;

/// Helper to create a test configuration for a node.
fn create_node_config(node_id: u64, temp_dir: &TempDir, cookie: &str) -> ClusterBootstrapConfig {
    ClusterBootstrapConfig {
        node_id,
        data_dir: Some(temp_dir.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0, // OS-assigned port
        cookie: cookie.into(),
        http_addr: format!("127.0.0.1:{}", 8080 + node_id).parse().unwrap(),
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            relay_url: None,
            enable_gossip: true,
            gossip_ticket: None,
        },
        peers: vec![], // No manual peers - rely on gossip!
    }
}

/// Test that three nodes automatically discover each other via gossip.
///
/// This test verifies the complete auto-discovery flow:
/// 1. Nodes start with no manual peer configuration
/// 2. Nodes broadcast their presence via gossip every 10 seconds
/// 3. Nodes receive announcements from other peers
/// 4. Network factory is automatically updated with discovered peers
///
/// NOTE: This test is currently ignored because Iroh gossip requires nodes to be
/// connected via the underlying Iroh network layer before they can exchange gossip
/// messages. In a production deployment, this works because nodes either:
/// 1. Have at least one bootstrap peer to join the gossip swarm
/// 2. Use mDNS or relay servers for initial discovery
/// 3. Are provided with cluster tickets containing bootstrap peers
///
/// This limitation is documented in the gossip integration design.
#[tokio::test]
#[ignore = "requires iroh network connectivity between nodes"]
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
    let peers1 = handle1.network_factory.peer_addrs();
    let peers2 = handle2.network_factory.peer_addrs();
    let peers3 = handle3.network_factory.peer_addrs();

    tracing::info!("node 1 discovered {} peers: {:?}", peers1.len(), peers1.keys().collect::<Vec<_>>());
    tracing::info!("node 2 discovered {} peers: {:?}", peers2.len(), peers2.keys().collect::<Vec<_>>());
    tracing::info!("node 3 discovered {} peers: {:?}", peers3.len(), peers3.keys().collect::<Vec<_>>());

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
    assert!(peers1.contains_key(&2), "node 1 should know about node 2");
    assert!(peers1.contains_key(&3), "node 1 should know about node 3");
    assert!(peers2.contains_key(&1), "node 2 should know about node 1");
    assert!(peers2.contains_key(&3), "node 2 should know about node 3");
    assert!(peers3.contains_key(&1), "node 3 should know about node 1");
    assert!(peers3.contains_key(&2), "node 3 should know about node 2");

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
    let config1 = ClusterBootstrapConfig {
        node_id: 10,
        data_dir: Some(temp_dir1.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "manual-peers-test".into(),
        http_addr: "127.0.0.1:9010".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            relay_url: None,
            enable_gossip: false, // Gossip DISABLED
            gossip_ticket: None,
        },
        peers: vec![],
    };

    let handle1 = bootstrap_node(config1).await?;

    // Node 2 with gossip disabled but manual peer to node 1
    let addr1 = handle1.iroh_manager.node_addr();
    let peer_spec = format!("10@{}", addr1.id);

    let config2 = ClusterBootstrapConfig {
        node_id: 11,
        data_dir: Some(temp_dir2.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "manual-peers-test".into(),
        http_addr: "127.0.0.1:9011".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig {
            secret_key: None,
            relay_url: None,
            enable_gossip: false,
            gossip_ticket: None,
        },
        peers: vec![peer_spec], // Manual peer configuration
    };

    let handle2 = bootstrap_node(config2).await?;

    // Verify manual peer was added to node 2
    let peers2 = handle2.network_factory.peer_addrs();
    assert_eq!(peers2.len(), 1, "node 2 should have 1 manually configured peer");
    assert!(peers2.contains_key(&10), "node 2 should know about node 10");

    // Verify gossip is None (disabled)
    assert!(
        handle1.gossip_discovery.is_none(),
        "node 1 should not have gossip discovery"
    );
    assert!(
        handle2.gossip_discovery.is_none(),
        "node 2 should not have gossip discovery"
    );

    // Node 1 should have zero peers (no gossip, no manual config)
    let peers1 = handle1.network_factory.peer_addrs();
    assert_eq!(peers1.len(), 0, "node 1 should have 0 peers");

    handle1.shutdown().await?;
    handle2.shutdown().await?;

    tracing::info!("test completed - manual peer configuration works with gossip disabled");

    Ok(())
}
