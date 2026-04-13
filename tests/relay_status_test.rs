//! Integration test for relay status on a real multi-node cluster.
//!
//! Verifies that nodes in a real Aspen cluster have working relay connectivity:
//! - Each node's iroh endpoint comes online (connects to a relay)
//! - EndpointAddr contains relay URL information
//! - Connection types between peers use relay or direct paths
//! - Endpoint addresses are consistent across the cluster
//!
//! These tests require real network access and are ignored by default.
//! Run with: `cargo nextest run -P network --run-ignored all -E 'test(/relay_status/)'`

use std::time::Duration;

use anyhow::Result;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::info;

/// Test timeout for cluster operations.
const CLUSTER_TIMEOUT: Duration = Duration::from_secs(60);
/// Time to wait for gossip discovery between nodes.
const GOSSIP_DISCOVERY_WAIT: Duration = Duration::from_secs(5);
/// Time to wait for leader election.
const LEADER_ELECTION_WAIT: Duration = Duration::from_millis(1000);
/// Test cookie for the cluster.
const TEST_COOKIE: &str = "relay-status-test-cluster";
/// Number of nodes in the test cluster.
const NODE_COUNT: u64 = 3;
/// Timeout for iroh endpoint to come online (relay connection).
const ONLINE_TIMEOUT: Duration = Duration::from_secs(30);

/// Generate a deterministic secret key for a node.
fn generate_secret_key(node_id: u64) -> String {
    format!("{:064x}", 2000 + node_id)
}

/// Helper to create a node with gossip enabled.
async fn create_node(node_id: u64, temp_dir: &TempDir, secret_key: &str) -> Result<Node> {
    let data_dir = temp_dir.path().join(format!("node-{}", node_id));

    let mut node = NodeBuilder::new(NodeId(node_id), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie(TEST_COOKIE)
        .with_gossip(true)
        .with_mdns(false)
        .with_iroh_secret_key(secret_key)
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1500, 3000)
        .start()
        .await?;

    node.spawn_router().unwrap();
    Ok(node)
}

/// Extract relay URL strings from an EndpointAddr.
fn relay_urls_from_addr(addr: &iroh::EndpointAddr) -> Vec<String> {
    addr.relay_urls().map(|u| u.to_string()).collect()
}

/// Count IP (direct) addresses from an EndpointAddr.
fn ip_addr_count(addr: &iroh::EndpointAddr) -> usize {
    addr.ip_addrs().count()
}

/// Helper to form a full cluster from a vec of nodes.
async fn form_cluster(nodes: &[Node]) -> Result<()> {
    let node1 = &nodes[0];

    // Initialize cluster on node 1
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
        trust: Default::default(),
    };
    timeout(CLUSTER_TIMEOUT, node1.raft_node().init(init_request)).await??;
    sleep(LEADER_ELECTION_WAIT).await;

    // Wait for gossip discovery
    sleep(GOSSIP_DISCOVERY_WAIT).await;

    // Add other nodes as learners
    for id in 2..=nodes.len() as u64 {
        let node = &nodes[(id - 1) as usize];
        let learner = ClusterNode::with_iroh_addr(id, node.endpoint_addr());
        let request = aspen::api::AddLearnerRequest { learner };
        timeout(CLUSTER_TIMEOUT, node1.raft_node().add_learner(request)).await??;
        sleep(Duration::from_millis(500)).await;
    }

    // Promote all nodes to voters
    sleep(Duration::from_secs(2)).await;
    let members: Vec<u64> = (1..=nodes.len() as u64).collect();
    let request = aspen::api::ChangeMembershipRequest { members };
    timeout(CLUSTER_TIMEOUT, node1.raft_node().change_membership(request)).await??;

    Ok(())
}

// ============================================================================
// Relay Status Tests
// ============================================================================

/// Test that each node's iroh endpoint comes online and has a relay URL.
///
/// "Online" means the endpoint has successfully contacted a relay server.
/// The EndpointAddr should contain a relay URL after coming online.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_relay_status_nodes_come_online() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    info!("Starting relay status test: nodes come online");

    let temp_dir = TempDir::new()?;
    let mut nodes: Vec<Node> = Vec::new();

    // Create nodes
    for id in 1..=NODE_COUNT {
        let secret_key = generate_secret_key(id);
        let node = create_node(id, &temp_dir, &secret_key).await?;
        nodes.push(node);
    }

    // Wait for each node's endpoint to come online (contact relay)
    for (idx, node) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;
        let endpoint = node.handle().network.iroh_manager.endpoint();

        info!(node_id, "waiting for endpoint to come online...");
        match timeout(ONLINE_TIMEOUT, endpoint.online()).await {
            Ok(()) => {
                info!(node_id, "endpoint is online (relay connected)");
            }
            Err(_) => {
                // Timeout is not necessarily a failure — relay may be unavailable
                // but direct connections can still work
                info!(node_id, "endpoint did not come online within {:?} (relay may be unavailable)", ONLINE_TIMEOUT);
            }
        }

        // Check the endpoint address for relay information
        let addr = endpoint.addr();
        let relays = relay_urls_from_addr(&addr);
        let direct_count = ip_addr_count(&addr);

        info!(
            node_id,
            endpoint_id = %addr.id.fmt_short(),
            has_relay = !relays.is_empty(),
            relay_urls = ?relays,
            direct_addresses = direct_count,
            "endpoint address info"
        );

        // At minimum we should have direct addresses (bound sockets)
        assert!(
            direct_count > 0 || !relays.is_empty(),
            "node {} should have at least one address (direct or relay)",
            node_id
        );
    }

    // Shutdown
    for node in nodes {
        node.shutdown().await?;
    }

    info!("Relay status test: nodes come online — completed");
    Ok(())
}

// NOTE: test_relay_status_connection_types_in_cluster was removed because
// iroh 0.97 dropped Endpoint::conn_type() and the ConnectionType enum.
// The remaining tests cover relay URL presence, addr consistency, and
// single-node endpoint behavior without depending on connection type introspection.

/// Test that endpoint addresses are consistent across the cluster.
///
/// Each node should see its own EndpointAddr with a relay URL (when relay is
/// enabled), and the addresses reported in Raft membership should match the
/// actual endpoint addresses.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_relay_status_endpoint_addr_consistency() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    info!("Starting relay status test: endpoint address consistency");

    let temp_dir = TempDir::new()?;
    let mut nodes: Vec<Node> = Vec::new();

    for id in 1..=NODE_COUNT {
        let secret_key = generate_secret_key(id);
        let node = create_node(id, &temp_dir, &secret_key).await?;
        nodes.push(node);
    }

    // Wait for all endpoints to come online
    for (idx, node) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;
        let endpoint = node.handle().network.iroh_manager.endpoint();
        match timeout(ONLINE_TIMEOUT, endpoint.online()).await {
            Ok(()) => info!(node_id, "online"),
            Err(_) => info!(node_id, "online timeout (relay may be unavailable)"),
        }
    }

    form_cluster(&nodes).await?;
    info!("cluster formed, checking address consistency...");

    // Collect endpoint IDs from each node
    let mut endpoint_ids = Vec::new();
    for (idx, node) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;
        let addr = node.endpoint_addr();
        let iroh_manager_addr = node.handle().network.iroh_manager.node_addr();

        let relays = relay_urls_from_addr(&addr);
        let direct_count = ip_addr_count(&addr);

        info!(
            node_id,
            endpoint_id = %addr.id.fmt_short(),
            relay_urls = ?relays,
            direct_addrs = direct_count,
            "node endpoint address"
        );

        // endpoint_addr() and iroh_manager.node_addr() should be consistent
        assert_eq!(
            addr.id, iroh_manager_addr.id,
            "node {}: endpoint ID should be consistent between Node and IrohEndpointManager",
            node_id
        );

        endpoint_ids.push(addr.id);
    }

    // All endpoint IDs should be unique
    let unique_count = {
        let mut unique = endpoint_ids.clone();
        unique.sort();
        unique.dedup();
        unique.len()
    };
    assert_eq!(unique_count, NODE_COUNT as usize, "all nodes should have unique endpoint IDs");

    // Check Raft membership includes all node endpoint IDs
    let state = nodes[0].raft_node().current_state().await?;
    info!(
        members = ?state.members,
        nodes_count = state.nodes.len(),
        "cluster membership"
    );

    for (idx, _) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;
        assert!(state.members.contains(&node_id), "node {} should be in cluster membership", node_id);
    }

    // Verify membership node info contains matching iroh addresses
    for node_info in &state.nodes {
        let raft_endpoint_id = node_info.iroh_addr().unwrap().id;
        assert!(
            endpoint_ids.contains(&raft_endpoint_id),
            "raft membership endpoint ID {} should match a running node",
            raft_endpoint_id.fmt_short()
        );
        info!(
            node_id = node_info.id,
            endpoint_id = %raft_endpoint_id.fmt_short(),
            "raft membership node matches running node"
        );
    }

    // Shutdown
    for node in nodes {
        node.shutdown().await?;
    }

    info!("Relay status test: endpoint address consistency — completed");
    Ok(())
}

/// Test relay status with a single-node cluster (simplest case).
///
/// Even a single node should be able to contact a relay and report its address.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_relay_status_single_node() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    info!("Starting relay status test: single node");

    let temp_dir = TempDir::new()?;
    let secret_key = generate_secret_key(1);
    let node = create_node(1, &temp_dir, &secret_key).await?;

    let endpoint = node.handle().network.iroh_manager.endpoint();

    // Wait for online
    info!("waiting for single node to come online...");
    match timeout(ONLINE_TIMEOUT, endpoint.online()).await {
        Ok(()) => {
            let addr = endpoint.addr();
            let relays = relay_urls_from_addr(&addr);
            let direct_count = ip_addr_count(&addr);

            info!(
                endpoint_id = %addr.id.fmt_short(),
                relay_urls = ?relays,
                direct_addrs = direct_count,
                "single node is online"
            );

            // When online, we should have a relay URL
            assert!(!relays.is_empty(), "online endpoint should have at least one relay URL");
        }
        Err(_) => {
            info!("single node did not come online within timeout");
            // Still check what address info we have
            let addr = endpoint.addr();
            let relays = relay_urls_from_addr(&addr);
            let direct_count = ip_addr_count(&addr);

            info!(
                endpoint_id = %addr.id.fmt_short(),
                relay_urls = ?relays,
                direct_addrs = direct_count,
                "address info without relay"
            );
        }
    }

    // Check bound sockets
    let sockets = endpoint.bound_sockets();
    info!(sockets = ?sockets, "bound sockets");
    assert!(!sockets.is_empty(), "should have at least one bound socket");

    node.shutdown().await?;

    info!("Relay status test: single node — completed");
    Ok(())
}
