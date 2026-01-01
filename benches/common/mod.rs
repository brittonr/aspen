//! Common benchmark infrastructure for Aspen.
//!
//! Provides cluster setup helpers and workload generators following
//! Tiger Style principles (bounded resources, pre-allocated collections).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::storage::RedbLogStore;
use aspen::raft::types::NodeId;
use aspen::raft::types::RaftMemberInfo;
use aspen::testing::AspenRouter;
use aspen::testing::create_test_raft_member_info;
use openraft::Config;
use openraft::ServerState;
use tempfile::TempDir;

/// Default timeout for cluster operations.
pub fn default_timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Setup a single-node cluster for benchmarking.
///
/// Returns the router and the leader node ID.
/// The cluster is fully initialized and ready for operations.
#[allow(dead_code)]
pub async fn setup_single_node_cluster() -> Result<(AspenRouter, NodeId)> {
    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config);
    router.new_raft_node(0).await?;

    // Initialize single-node cluster
    let node = router.get_raft_handle(0)?;
    let mut members: BTreeMap<NodeId, RaftMemberInfo> = BTreeMap::new();
    members.insert(NodeId::from(0), create_test_raft_member_info(0));
    node.initialize(members).await?;

    // Wait for leader election
    router.wait(0, default_timeout()).state(ServerState::Leader, "node 0 is leader").await?;

    Ok((router, NodeId::from(0)))
}

/// Setup a three-node cluster for benchmarking.
///
/// Returns the router and the leader node ID.
/// The cluster is fully initialized with all nodes as voters.
#[allow(dead_code)]
pub async fn setup_three_node_cluster() -> Result<(AspenRouter, NodeId)> {
    let config = Arc::new(
        Config {
            enable_tick: true,
            heartbeat_interval: 100,
            election_timeout_min: 500,
            election_timeout_max: 1000,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config);

    // Create all nodes
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;
    router.new_raft_node(2).await?;

    // Initialize cluster with node 0 as initial leader
    let node0 = router.get_raft_handle(0)?;
    let mut members: BTreeMap<NodeId, RaftMemberInfo> = BTreeMap::new();
    members.insert(NodeId::from(0), create_test_raft_member_info(0));
    members.insert(NodeId::from(1), create_test_raft_member_info(1));
    members.insert(NodeId::from(2), create_test_raft_member_info(2));
    node0.initialize(members).await?;

    // Wait for leader election
    router.wait(0, default_timeout()).state(ServerState::Leader, "leader elected").await?;

    Ok((router, NodeId::from(0)))
}

/// Pre-populate a cluster with test data.
///
/// Writes `count` key-value pairs with format "key-{i}" -> "value-{i}".
/// Uses bounded batch size for Tiger Style compliance.
#[allow(dead_code)]
pub async fn populate_test_data(router: &AspenRouter, node_id: NodeId, count: usize) -> Result<()> {
    // Tiger Style: process in bounded batches to avoid memory spikes
    const BATCH_SIZE: usize = 100;

    for batch_start in (0..count).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(count);

        for i in batch_start..batch_end {
            let key = format!("key-{:06}", i);
            let value = format!("value-{:06}", i);
            router.write(node_id, key, value).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
    }

    // Wait for all writes to be applied
    let expected_log_index = count as u64 + 1; // +1 for initialization
    router
        .wait(node_id, default_timeout())
        .log_index_at_least(Some(expected_log_index), "data populated")
        .await?;

    Ok(())
}

/// Generate key-value pairs for batch operations.
///
/// Returns a pre-allocated Vec with the specified count.
/// Tiger Style: bounded size, pre-allocated capacity.
#[allow(dead_code)]
pub fn generate_kv_pairs(count: usize, prefix: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::with_capacity(count);
    for i in 0..count {
        pairs.push((format!("{}-{:06}", prefix, i), format!("value-{:06}", i)));
    }
    pairs
}

/// YCSB-style workload patterns.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum WorkloadPattern {
    /// 50% read, 50% update (session store pattern)
    ReadHeavy,
    /// 95% read, 5% update (photo tagging pattern)
    MostlyRead,
    /// 5% read, 95% write (write-heavy ingest)
    WriteHeavy,
    /// 50% read, 50% write (mixed workload)
    Mixed,
}

impl WorkloadPattern {
    /// Returns (read_percentage, write_percentage).
    #[allow(dead_code)]
    pub fn ratios(&self) -> (u8, u8) {
        match self {
            Self::ReadHeavy => (50, 50),
            Self::MostlyRead => (95, 5),
            Self::WriteHeavy => (5, 95),
            Self::Mixed => (50, 50),
        }
    }
}

/// Operation type for workload generation.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Operation {
    Read { key: String },
    Write { key: String, value: String },
}

/// Generate a deterministic workload based on pattern.
///
/// Tiger Style: bounded size, pre-allocated capacity.
#[allow(dead_code)]
pub fn generate_workload(pattern: WorkloadPattern, count: usize, key_space: usize) -> Vec<Operation> {
    let (read_pct, _write_pct) = pattern.ratios();
    let mut ops = Vec::with_capacity(count);

    for i in 0..count {
        let key_idx = i % key_space;
        let key = format!("key-{:06}", key_idx);

        // Deterministic pattern based on operation index
        if (i * 100 / count) < read_pct as usize {
            ops.push(Operation::Read { key });
        } else {
            let value = format!("value-{:06}-updated", i);
            ops.push(Operation::Write { key, value });
        }
    }

    ops
}

// ====================================================================================
// Storage Layer Setup Helpers
// ====================================================================================

/// Setup an isolated RedbLogStore for benchmarking.
///
/// Returns the log store and temp directory (must keep TempDir alive for cleanup).
/// Tiger Style: RAII cleanup via TempDir.
#[allow(dead_code)]
pub fn setup_redb_log_store() -> Result<(RedbLogStore, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("raft-log.redb");
    let store = RedbLogStore::new(&db_path)?;
    Ok((store, temp_dir))
}

// ====================================================================================
// Value Generation Helpers
// ====================================================================================

/// Generate a value of specific size (repeating pattern).
///
/// Uses a pattern that doesn't compress well to simulate realistic data.
/// Tiger Style: Pre-allocated with exact capacity.
#[allow(dead_code)]
pub fn generate_value(size: usize) -> String {
    const PATTERN: &str = "0123456789abcdef";
    let full_repeats = size / PATTERN.len();
    let remainder = size % PATTERN.len();

    let mut value = String::with_capacity(size);
    for _ in 0..full_repeats {
        value.push_str(PATTERN);
    }
    value.push_str(&PATTERN[..remainder]);
    value
}

/// Common value sizes for benchmarking.
#[allow(dead_code)]
pub const VALUE_SIZE_SMALL: usize = 64;
#[allow(dead_code)]
pub const VALUE_SIZE_MEDIUM: usize = 1024;
#[allow(dead_code)]
pub const VALUE_SIZE_LARGE: usize = 65536;

// ====================================================================================
// Production Node Setup Helpers (Real Storage + Networking)
// ====================================================================================

use aspen::api::AddLearnerRequest;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::raft::storage::StorageBackend;

/// Setup a single production node with Redb storage for realistic benchmarking.
///
/// Unlike setup_single_node_cluster() which uses in-memory storage, this creates
/// a real node with:
/// - Redb single-fsync storage (log + state machine in one transaction)
/// - Iroh P2P networking (even for single node)
///
/// Returns the Node (caller must keep temp_dir alive for cleanup).
/// Tiger Style: Real I/O, production-like latencies (~2-3ms per write).
#[allow(dead_code)]
pub async fn setup_production_single_node(temp_dir: &TempDir) -> Result<Node> {
    let data_dir = temp_dir.path().join("node-1");

    let mut node = NodeBuilder::new(aspen::node::NodeId(1), &data_dir)
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false) // Disable gossip to avoid hanging on subscription
        .with_mdns(false) // Disable mDNS for faster startup
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;

    node.spawn_router();

    // Initialize single-node cluster
    let raft_node = node.raft_node();
    let endpoint_addr = node.endpoint_addr();
    raft_node
        .init(InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, endpoint_addr)],
        })
        .await?;

    // Wait for leader state using OpenRaft Wait API
    raft_node
        .raft()
        .wait(Some(Duration::from_secs(5)))
        .state(ServerState::Leader, "node becomes leader")
        .await?;

    Ok(node)
}


/// Setup a 3-node production cluster with real Iroh networking.
///
/// Creates a fully replicated cluster with:
/// - 3 nodes each with Redb single-fsync storage
/// - Real Iroh QUIC connections between nodes
/// - Node 1 as initial leader, nodes 2 and 3 as voters
///
/// Returns Vec of nodes with node[0] as the leader.
/// Tiger Style: Real network I/O, quorum replication latencies.
#[allow(dead_code)]
pub async fn setup_production_three_node(temp_dir: &TempDir) -> Result<Vec<Node>> {
    // Node 1 (will become leader)
    let mut node1 = NodeBuilder::new(aspen::node::NodeId(1), temp_dir.path().join("node-1"))
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false) // Disable gossip to avoid hanging on subscription
        .with_mdns(false) // Disable mDNS for faster startup
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;
    node1.spawn_router();
    // Wait for the endpoint to be fully online and Router to start accepting
    node1.handle().iroh_manager.endpoint().online().await;
    // Small delay to ensure Router's accept loop is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Node 2
    let mut node2 = NodeBuilder::new(aspen::node::NodeId(2), temp_dir.path().join("node-2"))
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;
    node2.spawn_router();
    // Wait for the endpoint to be fully online and Router to start accepting
    node2.handle().iroh_manager.endpoint().online().await;
    // Small delay to ensure Router's accept loop is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Node 3
    let mut node3 = NodeBuilder::new(aspen::node::NodeId(3), temp_dir.path().join("node-3"))
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;
    node3.spawn_router();
    // Wait for the endpoint to be fully online and Router to start accepting
    node3.handle().iroh_manager.endpoint().online().await;
    // Small delay to ensure Router's accept loop is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Initialize cluster with node1 as leader
    let raft1 = node1.raft_node();
    raft1
        .init(InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
        })
        .await?;

    // Wait for node1 to become leader using OpenRaft Wait API
    raft1
        .raft()
        .wait(Some(Duration::from_secs(10)))
        .state(ServerState::Leader, "node1 becomes leader")
        .await?;

    // Add node2 as learner
    raft1
        .add_learner(AddLearnerRequest {
            learner: ClusterNode::with_iroh_addr(2, node2.endpoint_addr()),
        })
        .await?;

    // Add node3 as learner
    raft1
        .add_learner(AddLearnerRequest {
            learner: ClusterNode::with_iroh_addr(3, node3.endpoint_addr()),
        })
        .await?;

    // Get current log index and wait for replication to catch up
    let metrics = raft1.raft().metrics().borrow().clone();
    let current_index = metrics.last_log_index.unwrap_or(0);
    raft1
        .raft()
        .wait(Some(Duration::from_secs(10)))
        .applied_index_at_least(Some(current_index), "learners catch up")
        .await?;

    // Promote learners to voters
    raft1.change_membership(ChangeMembershipRequest { members: vec![1, 2, 3] }).await?;

    // Wait for membership change to propagate to all nodes
    raft1
        .raft()
        .wait(Some(Duration::from_secs(10)))
        .voter_ids([NodeId::from(1), NodeId::from(2), NodeId::from(3)], "all nodes are voters")
        .await?;

    Ok(vec![node1, node2, node3])
}

/// Setup a 3-node production cluster with Redb single-fsync storage.
///
/// Creates a fully replicated cluster with:
/// - 3 nodes each with SharedRedbStorage (single-fsync)
/// - Real Iroh QUIC connections between nodes
/// - Node 1 as initial leader, nodes 2 and 3 as voters
///
/// Expected to significantly outperform SQLite-based 3-node cluster due to
/// single-fsync architecture (~5-8ms vs ~19ms per write).
///
/// Returns Vec of nodes with node[0] as the leader.
/// Tiger Style: Real network I/O, optimized single-fsync latencies.
#[allow(dead_code)]
pub async fn setup_production_three_node_redb(temp_dir: &TempDir) -> Result<Vec<Node>> {
    // Node 1 (will become leader)
    let mut node1 = NodeBuilder::new(aspen::node::NodeId(1), temp_dir.path().join("node-1"))
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false) // Disable gossip to avoid hanging on subscription
        .with_mdns(false) // Disable mDNS for faster startup
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;
    node1.spawn_router();
    // Wait for the endpoint to be fully online and Router to start accepting
    node1.handle().iroh_manager.endpoint().online().await;
    // Small delay to ensure Router's accept loop is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Node 2
    let mut node2 = NodeBuilder::new(aspen::node::NodeId(2), temp_dir.path().join("node-2"))
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;
    node2.spawn_router();
    // Wait for the endpoint to be fully online and Router to start accepting
    node2.handle().iroh_manager.endpoint().online().await;
    // Small delay to ensure Router's accept loop is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Node 3
    let mut node3 = NodeBuilder::new(aspen::node::NodeId(3), temp_dir.path().join("node-3"))
        .with_storage(StorageBackend::Redb)
        .with_cookie("bench-cluster")
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .start()
        .await?;
    node3.spawn_router();
    // Wait for the endpoint to be fully online and Router to start accepting
    node3.handle().iroh_manager.endpoint().online().await;
    // Small delay to ensure Router's accept loop is running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Initialize cluster with node1 as leader
    let raft1 = node1.raft_node();
    raft1
        .init(InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
        })
        .await?;

    // Wait for node1 to become leader using OpenRaft Wait API
    raft1
        .raft()
        .wait(Some(Duration::from_secs(10)))
        .state(ServerState::Leader, "node1 becomes leader")
        .await?;

    // Add node2 as learner
    raft1
        .add_learner(AddLearnerRequest {
            learner: ClusterNode::with_iroh_addr(2, node2.endpoint_addr()),
        })
        .await?;

    // Add node3 as learner
    raft1
        .add_learner(AddLearnerRequest {
            learner: ClusterNode::with_iroh_addr(3, node3.endpoint_addr()),
        })
        .await?;

    // Get current log index and wait for replication to catch up
    let metrics = raft1.raft().metrics().borrow().clone();
    let current_index = metrics.last_log_index.unwrap_or(0);
    raft1
        .raft()
        .wait(Some(Duration::from_secs(10)))
        .applied_index_at_least(Some(current_index), "learners catch up")
        .await?;

    // Promote learners to voters
    raft1.change_membership(ChangeMembershipRequest { members: vec![1, 2, 3] }).await?;

    // Wait for membership change to propagate to all nodes
    raft1
        .raft()
        .wait(Some(Duration::from_secs(10)))
        .voter_ids([NodeId::from(1), NodeId::from(2), NodeId::from(3)], "all nodes are voters")
        .await?;

    Ok(vec![node1, node2, node3])
}
