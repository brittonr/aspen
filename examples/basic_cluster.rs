//! Basic Cluster Example
//!
//! This example demonstrates:
//! - Bootstrapping a single Aspen node
//! - Initializing a cluster with one member
//! - Querying cluster state
//! - Graceful shutdown
//!
//! Run with: cargo run --example basic_cluster

use anyhow::Result;
use aspen::api::{ClusterController, ClusterNode, InitRequest};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::raft::RaftControlClient;
use tempfile::TempDir;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Aspen Basic Cluster Example");

    // Create temporary directory for data
    let temp_dir = TempDir::new()?;
    let data_dir = temp_dir.path().to_path_buf();
    info!("📁 Data directory: {}", data_dir.display());

    // Configure the node
    let config = ClusterBootstrapConfig {
        node_id: 1,
        data_dir: Some(data_dir.clone()),
        host: "127.0.0.1".into(),
        ractor_port: 0, // OS-assigned port
        cookie: "basic-cluster-example".into(),
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

    info!("🔧 Bootstrapping node {}", config.node_id);
    let handle = bootstrap_node(config.clone()).await?;
    info!("✅ Node {} bootstrapped successfully", config.node_id);

    // Verify node is registered in metadata store
    let metadata = handle
        .metadata_store
        .get_node(config.node_id)?
        .expect("Node should be registered");
    info!(
        "📊 Node metadata: id={}, status={:?}",
        metadata.node_id, metadata.status
    );

    // Create cluster controller client
    let cluster_client = RaftControlClient::new(handle.raft_actor.clone());

    // Initialize the cluster with this single node
    info!("🏗️  Initializing cluster with single member");
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };

    let cluster_state = cluster_client.init(init_req).await?;
    info!("✅ Cluster initialized successfully");
    info!("📊 Cluster state:");
    info!("   Members: {:?}", cluster_state.members);
    info!("   Nodes: {} total", cluster_state.nodes.len());
    for node in &cluster_state.nodes {
        info!("     - Node {}: {}", node.id, node.addr);
    }

    // Query current cluster state
    let current_state = cluster_client.current_state().await?;
    info!("🔍 Current cluster state:");
    info!("   Voting members: {:?}", current_state.members);
    info!("   Learners: {}", current_state.learners.len());

    // Check Raft metrics
    let metrics = handle.raft_core.metrics().borrow().clone();
    info!("📈 Raft metrics:");
    info!("   Node ID: {}", metrics.id);
    info!("   Current term: {:?}", metrics.current_term);
    info!("   Current leader: {:?}", metrics.current_leader);
    info!("   State: {:?}", metrics.state);

    // Keep a reference to the metadata store before shutdown
    let metadata_store = handle.metadata_store.clone();

    // Graceful shutdown
    info!("🛑 Shutting down node...");
    handle.shutdown().await?;
    info!("✅ Node shut down successfully");

    // Verify node status was updated
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let metadata = metadata_store
        .get_node(config.node_id)?
        .expect("Node should still be in metadata");
    info!(
        "📊 Final node status: {:?} (expected: Offline)",
        metadata.status
    );

    info!("🎉 Basic cluster example completed successfully!");

    Ok(())
}
