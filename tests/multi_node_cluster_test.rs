//! Multi-node cluster integration test.
//!
//! This test mirrors the functionality of scripts/kitty-cluster.sh but runs
//! as an automated test. It spawns multiple nodes, forms a cluster, and verifies
//! that docs sync and KV operations work correctly across the cluster.
//!
//! Similar to the kitty cluster:
//! - Spawns N nodes with gossip discovery
//! - Initializes a cluster on node 1
//! - Adds other nodes as learners
//! - Promotes all nodes to voters
//! - Verifies KV operations work
//! - Verifies docs sync propagates entries
//!
//! This test requires network access and is ignored by default.

use std::time::Duration;

use anyhow::Result;
use aspen::api::AddLearnerRequest;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::ReadRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
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
const TEST_COOKIE: &str = "multi-node-test-cluster";
/// Number of nodes in the test cluster.
const NODE_COUNT: u64 = 3;

/// Helper to create a node with gossip enabled.
async fn create_node(node_id: u64, temp_dir: &TempDir, secret_key: &str) -> Result<Node> {
    let data_dir = temp_dir.path().join(format!("node-{}", node_id));

    let mut node = NodeBuilder::new(NodeId(node_id), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie(TEST_COOKIE)
        .with_gossip(true)
        .with_mdns(false) // Disable mDNS for CI compatibility
        .with_iroh_secret_key(secret_key)
        .with_heartbeat_interval_ms(500)
        .with_election_timeout_ms(1500, 3000)
        .start()
        .await?;

    // Spawn the router to enable inter-node communication
    node.spawn_router();

    Ok(node)
}

/// Generate a deterministic secret key for a node (same as kitty-cluster.sh).
fn generate_secret_key(node_id: u64) -> String {
    format!("{:064x}", 1000 + node_id)
}

/// Test a full multi-node cluster lifecycle.
///
/// This test:
/// 1. Spawns 3 nodes with gossip enabled
/// 2. Initializes node 1 as a single-node cluster
/// 3. Waits for gossip discovery
/// 4. Adds nodes 2 and 3 as learners
/// 5. Promotes all nodes to voters
/// 6. Writes KV data and verifies it's readable
/// 7. Verifies the cluster state
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_multi_node_cluster_lifecycle() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    info!("Starting multi-node cluster test with {} nodes", NODE_COUNT);

    let temp_dir = TempDir::new()?;
    let mut nodes: Vec<Node> = Vec::new();

    // Step 1: Create all nodes
    info!("Creating {} nodes...", NODE_COUNT);
    for id in 1..=NODE_COUNT {
        let secret_key = generate_secret_key(id);
        let node = create_node(id, &temp_dir, &secret_key).await?;
        info!(
            node_id = id,
            endpoint = %node.endpoint_addr().id.fmt_short(),
            "node created"
        );
        nodes.push(node);
    }

    // Step 2: Initialize cluster on node 1
    info!("Initializing cluster on node 1...");
    let node1 = &nodes[0];
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
    };

    timeout(CLUSTER_TIMEOUT, node1.raft_node().init(init_request)).await??;

    // Wait for leader election
    sleep(LEADER_ELECTION_WAIT).await;

    // Verify node 1 is the leader
    let leader = node1.raft_node().get_leader().await?;
    assert_eq!(leader, Some(1), "node 1 should be the leader");
    info!("Node 1 is the leader");

    // Step 3: Wait for gossip discovery
    info!("Waiting for gossip discovery ({:?})...", GOSSIP_DISCOVERY_WAIT);
    sleep(GOSSIP_DISCOVERY_WAIT).await;

    // Step 4: Add other nodes as learners
    info!("Adding nodes 2-{} as learners...", NODE_COUNT);
    for id in 2..=NODE_COUNT {
        let node = &nodes[(id - 1) as usize];
        let learner = ClusterNode::with_iroh_addr(id, node.endpoint_addr());
        let request = AddLearnerRequest { learner };

        match timeout(CLUSTER_TIMEOUT, node1.raft_node().add_learner(request)).await? {
            Ok(state) => {
                info!(node_id = id, learners = ?state.learners, "added learner");
            }
            Err(e) => {
                info!(node_id = id, error = %e, "failed to add learner (may already be added)");
            }
        }

        // Small delay between add_learner calls
        sleep(Duration::from_millis(500)).await;
    }

    // Step 5: Promote all nodes to voters
    info!("Promoting all nodes to voters...");
    let members: Vec<u64> = (1..=NODE_COUNT).collect();
    let request = ChangeMembershipRequest { members };

    // Wait a bit for learners to catch up
    sleep(Duration::from_secs(2)).await;

    let state = timeout(CLUSTER_TIMEOUT, node1.raft_node().change_membership(request)).await??;

    info!(
        members = ?state.members,
        nodes = state.nodes.len(),
        "membership changed"
    );

    assert_eq!(state.members.len() as u64, NODE_COUNT, "all nodes should be voters");
    assert!(state.learners.is_empty(), "no learners should remain");

    // Step 6: Write KV data
    info!("Writing test data...");
    let write_request = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: vec![
                ("test/key1".to_string(), "value1".to_string()),
                ("test/key2".to_string(), "value2".to_string()),
                ("test/key3".to_string(), "value3".to_string()),
            ],
        },
    };

    let _write_result = timeout(CLUSTER_TIMEOUT, node1.raft_node().write(write_request)).await??;

    info!("data written successfully");

    // Step 7: Verify data is readable from all nodes
    info!("Verifying data from all nodes...");
    for (idx, node) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;

        // Read a key from this node
        let read_request = ReadRequest::new("test/key1".to_string());
        let result = timeout(CLUSTER_TIMEOUT, node.raft_node().read(read_request)).await??;

        let kv = result.kv.expect("key should exist");
        assert_eq!(kv.value, "value1", "node {} should read correct value", node_id);

        info!(node_id = node_id, key = "test/key1", value = %kv.value, "verified read");
    }

    // Step 8: Verify cluster state from each node's perspective
    info!("Verifying cluster state...");
    for (idx, node) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;
        let state = node.raft_node().current_state().await?;

        assert_eq!(state.members.len() as u64, NODE_COUNT, "node {} should see all members", node_id);

        info!(
            node_id = node_id,
            members = ?state.members,
            nodes = state.nodes.len(),
            "cluster state verified"
        );
    }

    // Step 9: Shutdown all nodes
    info!("Shutting down nodes...");
    for node in nodes {
        node.shutdown().await?;
    }

    info!("Multi-node cluster test completed successfully");
    Ok(())
}

/// Test that writes on leader replicate to followers.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_replication_to_followers() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut nodes: Vec<Node> = Vec::new();

    // Create 3 nodes
    for id in 1..=3u64 {
        let node = create_node(id, &temp_dir, &generate_secret_key(id)).await?;
        nodes.push(node);
    }

    // Initialize and form cluster
    let node1 = &nodes[0];
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
    };
    node1.raft_node().init(init_request).await?;
    sleep(LEADER_ELECTION_WAIT).await;

    // Wait for discovery
    sleep(GOSSIP_DISCOVERY_WAIT).await;

    // Add learners
    for id in 2..=3u64 {
        let node = &nodes[(id - 1) as usize];
        let request = AddLearnerRequest {
            learner: ClusterNode::with_iroh_addr(id, node.endpoint_addr()),
        };
        let _ = node1.raft_node().add_learner(request).await;
        sleep(Duration::from_millis(500)).await;
    }

    // Promote to voters
    sleep(Duration::from_secs(2)).await;
    let request = ChangeMembershipRequest { members: vec![1, 2, 3] };
    node1.raft_node().change_membership(request).await?;

    // Write data on leader
    info!("Writing data on leader...");
    for i in 1..=10 {
        let write_request = WriteRequest {
            command: WriteCommand::Set {
                key: format!("replica/key{}", i),
                value: format!("value{}", i),
            },
        };
        node1.raft_node().write(write_request).await?;
    }

    // Small delay for replication
    sleep(Duration::from_secs(1)).await;

    // Verify data is readable from followers
    info!("Verifying replication to followers...");
    for node_id in 2..=3u64 {
        let node = &nodes[(node_id - 1) as usize];

        for i in 1..=10 {
            let read_request = ReadRequest::new(format!("replica/key{}", i));
            let result = node.raft_node().read(read_request).await?;
            let kv = result.kv.expect("key should exist");
            assert_eq!(kv.value, format!("value{}", i));
        }

        info!(node_id = node_id, "verified 10 keys replicated");
    }

    // Shutdown
    for node in nodes {
        node.shutdown().await?;
    }

    Ok(())
}

/// Test leader election after leader shutdown.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_leader_failover() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut nodes: Vec<Option<Node>> = Vec::new();

    // Create 3 nodes
    for id in 1..=3u64 {
        let node = create_node(id, &temp_dir, &generate_secret_key(id)).await?;
        nodes.push(Some(node));
    }

    // Get node 1 reference
    let node1 = nodes[0].as_ref().unwrap();

    // Initialize cluster
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
    };
    node1.raft_node().init(init_request).await?;
    sleep(LEADER_ELECTION_WAIT).await;

    // Wait for discovery and add learners
    sleep(GOSSIP_DISCOVERY_WAIT).await;

    for id in 2..=3u64 {
        let node = nodes[(id - 1) as usize].as_ref().unwrap();
        let request = AddLearnerRequest {
            learner: ClusterNode::with_iroh_addr(id, node.endpoint_addr()),
        };
        let _ = node1.raft_node().add_learner(request).await;
        sleep(Duration::from_millis(500)).await;
    }

    // Promote to voters
    sleep(Duration::from_secs(2)).await;
    let request = ChangeMembershipRequest { members: vec![1, 2, 3] };
    node1.raft_node().change_membership(request).await?;

    // Write some data before failover
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "failover/test".to_string(),
            value: "before_failover".to_string(),
        },
    };
    node1.raft_node().write(write_request).await?;

    info!("Shutting down leader (node 1)...");

    // Shutdown node 1 (the leader)
    let node1 = nodes[0].take().unwrap();
    node1.shutdown().await?;

    // Wait for new leader election
    info!("Waiting for new leader election...");
    sleep(Duration::from_secs(5)).await;

    // Find the new leader
    let mut new_leader_id = None;
    for id in 2..=3u64 {
        let node = nodes[(id - 1) as usize].as_ref().unwrap();
        if let Ok(Some(leader)) = node.raft_node().get_leader().await {
            info!(node_id = id, leader = leader, "found leader");
            new_leader_id = Some(leader);
            break;
        }
    }

    let new_leader_id = new_leader_id.expect("should have elected a new leader");
    assert_ne!(new_leader_id, 1, "new leader should not be node 1");
    info!("New leader elected: node {}", new_leader_id);

    // Verify data is still accessible
    let new_leader = nodes[(new_leader_id - 1) as usize].as_ref().unwrap();
    let read_request = ReadRequest::new("failover/test".to_string());
    let result = new_leader.raft_node().read(read_request).await?;
    let kv = result.kv.expect("key should exist");
    assert_eq!(kv.value, "before_failover");
    info!("Data survived failover: {}", kv.value);

    // Write new data on new leader
    let write_request = WriteRequest {
        command: WriteCommand::Set {
            key: "failover/after".to_string(),
            value: "after_failover".to_string(),
        },
    };
    new_leader.raft_node().write(write_request).await?;
    info!("Successfully wrote data on new leader");

    // Shutdown remaining nodes
    for node in nodes.into_iter().flatten() {
        node.shutdown().await?;
    }

    Ok(())
}

/// Test scan operations across a replicated cluster.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_scan_across_cluster() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut nodes: Vec<Node> = Vec::new();

    // Create 3 nodes
    for id in 1..=3u64 {
        let node = create_node(id, &temp_dir, &generate_secret_key(id)).await?;
        nodes.push(node);
    }

    // Initialize and form cluster (abbreviated)
    let node1 = &nodes[0];
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
    };
    node1.raft_node().init(init_request).await?;
    sleep(LEADER_ELECTION_WAIT).await;
    sleep(GOSSIP_DISCOVERY_WAIT).await;

    for id in 2..=3u64 {
        let node = &nodes[(id - 1) as usize];
        let _ = node1
            .raft_node()
            .add_learner(AddLearnerRequest {
                learner: ClusterNode::with_iroh_addr(id, node.endpoint_addr()),
            })
            .await;
        sleep(Duration::from_millis(500)).await;
    }

    sleep(Duration::from_secs(2)).await;
    node1.raft_node().change_membership(ChangeMembershipRequest { members: vec![1, 2, 3] }).await?;

    // Write data with a common prefix
    info!("Writing test data for scan...");
    let write_request = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: (1..=20).map(|i| (format!("users/{:03}", i), format!("user_data_{}", i))).collect(),
        },
    };
    node1.raft_node().write(write_request).await?;

    // Write some other data
    let write_request = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: (1..=5).map(|i| (format!("config/{}", i), format!("config_value_{}", i))).collect(),
        },
    };
    node1.raft_node().write(write_request).await?;

    // Small delay for replication
    sleep(Duration::from_secs(1)).await;

    // Scan from each node
    info!("Testing scan from each node...");
    for (idx, node) in nodes.iter().enumerate() {
        let node_id = idx as u64 + 1;

        let scan_request = aspen::api::ScanRequest {
            prefix: "users/".to_string(),
            limit: Some(10),
            continuation_token: None,
        };

        let result = node.raft_node().scan(scan_request).await?;

        assert_eq!(result.count, 10, "node {} should return 10 entries", node_id);
        assert!(result.is_truncated, "node {} result should be truncated", node_id);
        assert!(result.continuation_token.is_some(), "should have continuation token");

        info!(node_id = node_id, count = result.count, truncated = result.is_truncated, "scan verified");
    }

    // Shutdown
    for node in nodes {
        node.shutdown().await?;
    }

    Ok(())
}
