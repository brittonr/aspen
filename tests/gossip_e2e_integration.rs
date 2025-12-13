//! End-to-end integration test for gossip-based peer discovery.
//!
//! This test verifies basic gossip configuration and ticket handling.
//! Full gossip discovery is not yet implemented in the simplified bootstrap,
//! so this test focuses on configuration and ticket serialization.

use std::time::Duration;

use anyhow::Result;
use aspen::api::{ClusterNode, InitRequest, ReadRequest, WriteCommand, WriteRequest};
use aspen::cluster::ticket::AspenClusterTicket;
use aspen::node::{NodeBuilder, NodeId};
use aspen::raft::storage::StorageBackend;
use iroh_gossip::proto::TopicId;
use tempfile::TempDir;
use tokio::time::sleep;
use tracing::info;

/// Create a test cluster cookie that all nodes will share.
const TEST_CLUSTER_COOKIE: &str = "gossip-e2e-test-cluster";

/// Start a node with gossip enabled.
async fn start_node_with_gossip(node_id: NodeId, temp_dir: &TempDir) -> Result<aspen::node::Node> {
    let data_dir = temp_dir.path().join(format!("node-{}", node_id.0));

    let node = NodeBuilder::new(node_id, &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_gossip(true)
        // More reasonable timeouts for testing
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1500, 3000)
        .start()
        .await?;

    Ok(node)
}

/// Test basic cluster formation with gossip configuration.
#[tokio::test]
async fn test_cluster_formation_with_gossip_config() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info")
        .try_init();

    let temp_dir = TempDir::new()?;

    info!("Starting 3 nodes with gossip enabled");

    // Start nodes with gossip enabled (even though discovery isn't fully implemented)
    let node1 = start_node_with_gossip(NodeId(1), &temp_dir).await?;
    let endpoint1 = node1.endpoint_addr();
    info!("Node 1 started with endpoint: {:?}", endpoint1);

    let node2 = start_node_with_gossip(NodeId(2), &temp_dir).await?;
    info!("Node 2 started");

    let node3 = start_node_with_gossip(NodeId(3), &temp_dir).await?;
    info!("Node 3 started");

    // Give nodes a moment to establish connections
    info!("Waiting for nodes to establish connections...");
    sleep(Duration::from_secs(2)).await;

    // Since gossip discovery isn't fully implemented in simplified bootstrap,
    // manually form the cluster for now
    info!("Forming Raft cluster manually");

    // Initialize Raft cluster using node 1 as bootstrap leader
    let controller1 = node1.cluster_controller();

    // Create initial membership with all nodes
    let mut initial_members = Vec::new();
    initial_members.push(ClusterNode {
        id: node1.node_id().0,
        addr: format!("node-{}", node1.node_id().0),
        raft_addr: None,
        iroh_addr: Some(node1.endpoint_addr()),
    });
    initial_members.push(ClusterNode {
        id: node2.node_id().0,
        addr: format!("node-{}", node2.node_id().0),
        raft_addr: None,
        iroh_addr: Some(node2.endpoint_addr()),
    });
    initial_members.push(ClusterNode {
        id: node3.node_id().0,
        addr: format!("node-{}", node3.node_id().0),
        raft_addr: None,
        iroh_addr: Some(node3.endpoint_addr()),
    });

    controller1.init(InitRequest { initial_members }).await?;
    info!("Raft cluster initialized");

    // Wait for leader election - give more time for election to complete
    info!("Waiting for leader election...");
    sleep(Duration::from_secs(10)).await;

    // Test that the cluster is operational
    info!("Testing cluster operations");
    let kv_store1 = node1.kv_store();

    // Write some test data
    kv_store1
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "test-key".to_string(),
                value: "test-value".to_string(),
            },
        })
        .await?;
    info!("Wrote test data to cluster");

    // Give time for replication
    sleep(Duration::from_millis(500)).await;

    // Verify data is replicated to all nodes
    let kv_store2 = node2.kv_store();
    let kv_store3 = node3.kv_store();

    let value1 = kv_store1
        .read(ReadRequest {
            key: "test-key".to_string(),
        })
        .await?;
    let value2 = kv_store2
        .read(ReadRequest {
            key: "test-key".to_string(),
        })
        .await?;
    let value3 = kv_store3
        .read(ReadRequest {
            key: "test-key".to_string(),
        })
        .await?;

    assert_eq!(value1.value, "test-value");
    assert_eq!(value2.value, "test-value");
    assert_eq!(value3.value, "test-value");

    info!("Data successfully replicated to all nodes");

    // Clean shutdown
    info!("Shutting down nodes");
    node3.shutdown().await?;
    node2.shutdown().await?;
    node1.shutdown().await?;

    Ok(())
}

/// Test gossip ticket serialization and deserialization.
#[tokio::test]
async fn test_gossip_ticket_handling() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info")
        .try_init();

    // Derive topic ID from cluster cookie
    let topic_hash = blake3::hash(TEST_CLUSTER_COOKIE.as_bytes());
    let topic_id = TopicId::from_bytes(*topic_hash.as_bytes());

    // Create a test endpoint ID
    let secret_key = iroh::SecretKey::from([1u8; 32]);
    let endpoint_id = secret_key.public();

    // Create and serialize a ticket
    let ticket =
        AspenClusterTicket::with_bootstrap(topic_id, TEST_CLUSTER_COOKIE.into(), endpoint_id);
    let ticket_str = ticket.serialize();

    info!("Created gossip ticket: {}", ticket_str);
    assert!(ticket_str.starts_with("aspen"));

    // Deserialize the ticket
    let deserialized = AspenClusterTicket::deserialize(&ticket_str)?;
    assert_eq!(deserialized.topic_id, topic_id);
    assert_eq!(deserialized.cluster_id, TEST_CLUSTER_COOKIE);
    assert_eq!(deserialized.bootstrap.len(), 1);
    assert!(deserialized.bootstrap.contains(&endpoint_id));

    info!("Ticket serialization/deserialization successful");

    Ok(())
}

/// Test multi-node cluster without gossip discovery.
#[tokio::test]
async fn test_multi_node_cluster_manual_config() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info")
        .try_init();

    let temp_dir = TempDir::new()?;
    let num_nodes = 5;

    info!("Starting {} nodes", num_nodes);

    // Start nodes
    let mut nodes = Vec::new();
    for i in 1..=num_nodes {
        let node = start_node_with_gossip(NodeId(i as u64), &temp_dir).await?;
        info!("Node {} started", i);
        nodes.push(node);
    }

    // Form cluster manually
    let controller1 = nodes[0].cluster_controller();
    let mut initial_members = Vec::new();

    for node in &nodes {
        initial_members.push(ClusterNode {
            id: node.node_id().0,
            addr: format!("node-{}", node.node_id().0),
            raft_addr: None,
            iroh_addr: Some(node.endpoint_addr()),
        });
    }

    controller1.init(InitRequest { initial_members }).await?;
    info!("Cluster initialized with {} nodes", num_nodes);

    // Wait for leader election - give more time for election to complete
    info!("Waiting for leader election...");
    sleep(Duration::from_secs(10)).await;

    // Write test data
    let kv_store = nodes[0].kv_store();
    kv_store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "multi-node-key".to_string(),
                value: "multi-node-value".to_string(),
            },
        })
        .await?;

    // Give time for replication
    sleep(Duration::from_millis(500)).await;

    // Verify on all nodes
    for (i, node) in nodes.iter().enumerate() {
        let value = node
            .kv_store()
            .read(ReadRequest {
                key: "multi-node-key".to_string(),
            })
            .await?;
        assert_eq!(
            value.value,
            "multi-node-value",
            "Node {} should have replicated data",
            i + 1
        );
    }

    info!("Data replicated successfully to all {} nodes", num_nodes);

    // Clean shutdown
    for node in nodes.into_iter().rev() {
        node.shutdown().await?;
    }

    Ok(())
}
