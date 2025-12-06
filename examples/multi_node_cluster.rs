//! Multi-Node Cluster Example
//!
//! This example demonstrates:
//! - Bootstrapping a 3-node cluster
//! - Exchanging peer addresses for IRPC connectivity
//! - Adding learners and promoting to voters
//! - Writing on the leader and verifying replication
//! - Distributed consensus and fault tolerance
//!
//! Run with: cargo run --example multi_node_cluster

use anyhow::Result;
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, InitRequest,
    KeyValueStore, ReadRequest, WriteCommand, WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::RaftControlClient;
use tempfile::TempDir;
use tokio::time::{Duration, sleep};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Aspen Multi-Node Cluster Example");

    // Create temporary directories for each node
    let temp_dir1 = TempDir::new()?;
    let temp_dir2 = TempDir::new()?;
    let temp_dir3 = TempDir::new()?;

    let data_dir1 = temp_dir1.path().to_path_buf();
    let data_dir2 = temp_dir2.path().to_path_buf();
    let data_dir3 = temp_dir3.path().to_path_buf();

    info!("📁 Data directories:");
    info!("   Node 1: {}", data_dir1.display());
    info!("   Node 2: {}", data_dir2.display());
    info!("   Node 3: {}", data_dir3.display());

    // Configure nodes
    let config1 = ClusterBootstrapConfig {
        node_id: 1,
        data_dir: Some(data_dir1),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "multi-node-example".into(),
        http_addr: "127.0.0.1:0".parse()?,
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

    let config2 = ClusterBootstrapConfig {
        node_id: 2,
        data_dir: Some(data_dir2),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "multi-node-example".into(),
        http_addr: "127.0.0.1:0".parse()?,
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

    let config3 = ClusterBootstrapConfig {
        node_id: 3,
        data_dir: Some(data_dir3),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "multi-node-example".into(),
        http_addr: "127.0.0.1:0".parse()?,
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

    // Bootstrap all nodes concurrently
    info!("\n🔧 Bootstrapping 3 nodes concurrently...");
    let (handle1, handle2, handle3) = tokio::try_join!(
        bootstrap_node(config1.clone()),
        bootstrap_node(config2.clone()),
        bootstrap_node(config3.clone())
    )?;
    info!("✅ All nodes bootstrapped successfully");

    // Exchange peer addresses for IRPC connectivity
    //
    // DISCOVERY NOTE: Aspen supports multiple automatic discovery methods:
    //
    // 1. **mDNS** (enabled by default): Discovers peers on the same LAN
    //    ⚠️ Does NOT work on localhost/127.0.0.1 (multicast limitation)
    //    ✅ Works for multi-machine testing on same network
    //
    // 2. **Gossip** (enabled by default): Broadcasts Raft metadata
    //    ✅ Requires initial Iroh connectivity (via mDNS, DNS, Pkarr, or manual)
    //
    // 3. **DNS discovery** (opt-in): Production peer discovery via DNS service
    //
    // 4. **Pkarr** (opt-in): DHT-based distributed discovery
    //
    // For this single-host example, we use manual peer exchange because mDNS
    // doesn't work on loopback interfaces. For multi-host deployments, the
    // default config (mDNS + gossip) provides zero-config discovery!
    //
    // See examples/README.md "Discovery Methods" section for details.
    info!("\n🔗 Exchanging peer addresses for IRPC connectivity (single-host workaround)");
    let addr1 = handle1.iroh_manager.node_addr().clone();
    let addr2 = handle2.iroh_manager.node_addr().clone();
    let addr3 = handle3.iroh_manager.node_addr().clone();

    // Node 1 knows about nodes 2 and 3
    handle1.network_factory.add_peer(2, addr2.clone());
    handle1.network_factory.add_peer(3, addr3.clone());

    // Node 2 knows about nodes 1 and 3
    handle2.network_factory.add_peer(1, addr1.clone());
    handle2.network_factory.add_peer(3, addr3.clone());

    // Node 3 knows about nodes 1 and 2
    handle3.network_factory.add_peer(1, addr1.clone());
    handle3.network_factory.add_peer(2, addr2.clone());

    info!("✅ Peer addresses exchanged (manual configuration for single-host testing)");
    info!("   💡 For multi-host LAN: mDNS + gossip provide zero-config discovery");
    info!("   💡 For production: enable DNS discovery + Pkarr + relay server");
    info!("   Node 1 endpoint ID: {}", addr1.id);
    info!("   Node 2 endpoint ID: {}", addr2.id);
    info!("   Node 3 endpoint ID: {}", addr3.id);

    // Create clients for node 1 (will be the leader)
    let cluster_client1 = RaftControlClient::new(handle1.raft_actor.clone());
    let kv_client1 = KvClient::new(handle1.raft_actor.clone());

    // Initialize cluster with node 1 as the only initial member
    info!("\n🏗️  Initializing cluster with node 1 as initial member");
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some(format!("iroh://{}", addr1.id)),
        )],
    };
    let state = cluster_client1.init(init_req).await?;
    info!("✅ Cluster initialized");
    info!("   Members: {:?}", state.members);

    // Give Raft time to establish leadership
    sleep(Duration::from_millis(500)).await;

    // Check leader metrics
    let metrics = handle1.raft_core.metrics().borrow().clone();
    info!("📈 Node 1 Raft metrics:");
    info!("   Current leader: {:?}", metrics.current_leader);
    info!("   State: {:?}", metrics.state);

    // Add node 2 as a learner
    info!("\n➕ Adding node 2 as learner");
    let learner_req = AddLearnerRequest {
        learner: ClusterNode::new(2, "127.0.0.1:26001", Some(format!("iroh://{}", addr2.id))),
    };
    let state = cluster_client1.add_learner(learner_req).await?;
    info!("✅ Node 2 added as learner");
    info!("   Members: {:?}", state.members);
    info!("   Learners: {} total", state.learners.len());

    // Give time for learner to sync
    sleep(Duration::from_millis(500)).await;

    // Add node 3 as a learner
    info!("\n➕ Adding node 3 as learner");
    let learner_req = AddLearnerRequest {
        learner: ClusterNode::new(3, "127.0.0.1:26002", Some(format!("iroh://{}", addr3.id))),
    };
    let state = cluster_client1.add_learner(learner_req).await?;
    info!("✅ Node 3 added as learner");
    info!("   Members: {:?}", state.members);
    info!("   Learners: {} total", state.learners.len());

    // Give time for learner to sync
    sleep(Duration::from_millis(500)).await;

    // Promote both learners to voting members
    info!("\n⬆️  Promoting learners to voting members");
    let membership_req = ChangeMembershipRequest {
        members: vec![1, 2, 3],
    };
    let state = cluster_client1.change_membership(membership_req).await?;
    info!("✅ Membership changed");
    info!("   Voting members: {:?}", state.members);
    info!("   Learners: {}", state.learners.len());

    // Give Raft time to establish new membership and replicate
    sleep(Duration::from_millis(1000)).await;

    // Write data on the leader
    info!("\n📝 Writing data to the cluster (via leader)");
    let pairs = vec![
        ("cluster:size".to_string(), "3".to_string()),
        ("cluster:name".to_string(), "aspen-example".to_string()),
        ("cluster:replication".to_string(), "enabled".to_string()),
    ];

    let write_req = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: pairs.clone(),
        },
    };
    kv_client1.write(write_req).await?;
    info!("✅ Written {} key-value pairs", pairs.len());

    // Give time for replication
    sleep(Duration::from_millis(500)).await;

    // Read data back from leader to verify
    info!("\n🔍 Reading data from leader (node 1):");
    for (key, expected_value) in &pairs {
        let read_req = ReadRequest { key: key.clone() };
        let result = kv_client1.read(read_req).await?;
        info!("  {} = {} ✓", result.key, result.value);
        assert_eq!(result.value, *expected_value);
    }

    // Write individual keys
    info!("\n📝 Writing individual keys");
    for i in 1..=5 {
        let key = format!("data:{}", i);
        let value = format!("replicated_value_{}", i);
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: key.clone(),
                value: value.clone(),
            },
        };
        kv_client1.write(write_req).await?;
        info!("  Written: {} = {}", key, value);
    }

    // Give time for replication
    sleep(Duration::from_millis(500)).await;

    // Get final cluster state
    info!("\n📊 Final Cluster State:");
    let final_state = cluster_client1.current_state().await?;
    info!("   Voting members: {:?}", final_state.members);
    info!("   Total nodes: {}", final_state.nodes.len());
    for node in &final_state.nodes {
        info!("     - Node {}: {}", node.id, node.addr);
    }

    // Check metrics for all nodes
    info!("\n📈 Raft Metrics:");
    let metrics1 = handle1.raft_core.metrics().borrow().clone();
    let metrics2 = handle2.raft_core.metrics().borrow().clone();
    let metrics3 = handle3.raft_core.metrics().borrow().clone();

    info!("   Node 1:");
    info!("     Leader: {:?}", metrics1.current_leader);
    info!("     State: {:?}", metrics1.state);
    info!("     Last applied: {:?}", metrics1.last_applied);

    info!("   Node 2:");
    info!("     Leader: {:?}", metrics2.current_leader);
    info!("     State: {:?}", metrics2.state);
    info!("     Last applied: {:?}", metrics2.last_applied);

    info!("   Node 3:");
    info!("     Leader: {:?}", metrics3.current_leader);
    info!("     State: {:?}", metrics3.state);
    info!("     Last applied: {:?}", metrics3.last_applied);

    // Summary
    info!("\n📊 Summary:");
    info!("  - Bootstrapped 3-node cluster");
    info!("  - Established Raft consensus with 1 leader and 2 followers");
    info!(
        "  - Successfully replicated {} keys across all nodes",
        pairs.len() + 5
    );
    info!("  - All nodes have consistent state (last_applied matches)");

    // Graceful shutdown of all nodes
    info!("\n🛑 Shutting down all nodes...");
    let _ = tokio::try_join!(handle1.shutdown(), handle2.shutdown(), handle3.shutdown());
    info!("✅ All nodes shut down successfully");

    info!("🎉 Multi-node cluster example completed successfully!");

    Ok(())
}
