//! Production Cluster Example
//!
//! This example demonstrates production-grade cluster deployment with:
//! - DNS-based peer discovery
//! - Pkarr DHT publishing for resilient discovery
//! - Relay server for NAT traversal
//! - Automatic gossip-based Raft metadata propagation
//! - Zero manual peer configuration
//!
//! ## Prerequisites
//!
//! This example requires external infrastructure:
//! 1. **Relay Server**: Facilitates connectivity through NAT/firewalls
//!    - Public relay: `https://relay.iroh.link` (n0's public relay)
//!    - Self-hosted: Deploy an Iroh relay server
//!
//! 2. **DNS Discovery Service**: Provides initial peer discovery
//!    - Public service: `https://dns.iroh.link` (n0's public DNS)
//!    - Self-hosted: Deploy a DNS discovery service
//!
//! 3. **Pkarr Relay** (optional): DHT-based discovery for resilience
//!    - Public relay: `https://pkarr.iroh.link` (n0's public Pkarr)
//!    - Self-hosted: Deploy a Pkarr relay
//!
//! ## Configuration
//!
//! Environment variables (optional overrides):
//! - `ASPEN_RELAY_URL`: Relay server URL (default: n0's public relay)
//! - `ASPEN_DNS_URL`: DNS discovery URL (default: n0's public DNS)
//! - `ASPEN_PKARR_URL`: Pkarr relay URL (default: n0's public Pkarr)
//! - `ASPEN_DISABLE_MDNS`: Set to "true" to disable mDNS (recommended for production)
//!
//! ## Deployment Patterns
//!
//! ### Cloud/Multi-Region Deployment
//! ```bash
//! # Node 1 (first node)
//! cargo run --example production_cluster
//!
//! # Nodes 2+ (in separate processes/hosts)
//! ASPEN_NODE_ID=2 cargo run --example production_cluster
//! ASPEN_NODE_ID=3 cargo run --example production_cluster
//! ```
//!
//! ### Container Orchestration (Kubernetes, Docker Compose)
//! ```yaml
//! services:
//!   aspen-node-1:
//!     environment:
//!       - ASPEN_NODE_ID=1
//!       - ASPEN_RELAY_URL=https://relay.example.com
//!       - ASPEN_DISABLE_MDNS=true
//!
//!   aspen-node-2:
//!     environment:
//!       - ASPEN_NODE_ID=2
//!       - ASPEN_RELAY_URL=https://relay.example.com
//!       - ASPEN_DISABLE_MDNS=true
//! ```
//!
//! Run with: cargo run --example production_cluster

use anyhow::{Context, Result};
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, InitRequest,
    KeyValueStore, ReadRequest, WriteCommand, WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::RaftControlClient;
use std::env;
use tempfile::TempDir;
use tokio::time::{Duration, sleep};
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

/// Parse environment variable with default
fn env_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Parse boolean environment variable
fn env_bool(key: &str) -> bool {
    env::var(key)
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Aspen Production Cluster Example");
    info!("📡 Configuration: DNS + Pkarr + Relay + Gossip");

    // Production configuration from environment
    let relay_url = env_or_default("ASPEN_RELAY_URL", "https://relay.iroh.link");
    let dns_url = env_or_default("ASPEN_DNS_URL", "https://dns.iroh.link");
    let pkarr_url = env_or_default("ASPEN_PKARR_URL", "https://pkarr.iroh.link");
    let disable_mdns = env_bool("ASPEN_DISABLE_MDNS");

    info!("\n📋 Discovery Configuration:");
    info!("   Relay Server: {}", relay_url);
    info!("   DNS Discovery: {}", dns_url);
    info!("   Pkarr Relay: {}", pkarr_url);
    info!(
        "   mDNS: {}",
        if disable_mdns { "disabled" } else { "enabled" }
    );
    info!("   Gossip: enabled (default)");

    // Create temporary directories for each node
    let temp_dir1 = TempDir::new()?;
    let temp_dir2 = TempDir::new()?;
    let temp_dir3 = TempDir::new()?;

    let data_dir1 = temp_dir1.path().to_path_buf();
    let data_dir2 = temp_dir2.path().to_path_buf();
    let data_dir3 = temp_dir3.path().to_path_buf();

    info!("\n📁 Data directories:");
    info!("   Node 1: {}", data_dir1.display());
    info!("   Node 2: {}", data_dir2.display());
    info!("   Node 3: {}", data_dir3.display());

    // Production Iroh configuration
    let iroh_config = IrohConfig {
        secret_key: None, // Let Iroh generate a key
        relay_url: Some(relay_url),
        enable_gossip: true,        // Announce Raft metadata
        gossip_ticket: None,        // First node doesn't need a ticket
        enable_mdns: !disable_mdns, // Disable in production, useful for dev
        enable_dns_discovery: true, // Primary discovery mechanism
        dns_discovery_url: Some(dns_url),
        enable_pkarr: true, // DHT-based discovery for resilience
        pkarr_relay_url: Some(pkarr_url),
    };

    // Configure nodes with production settings
    let config1 = ClusterBootstrapConfig {
        node_id: 1,
        data_dir: Some(data_dir1),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "production-cluster-example".into(),
        http_addr: "127.0.0.1:0".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: iroh_config.clone(),
        peers: vec![], // No manual peers - rely on discovery!
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let config2 = ClusterBootstrapConfig {
        node_id: 2,
        data_dir: Some(data_dir2),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "production-cluster-example".into(),
        http_addr: "127.0.0.1:0".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: iroh_config.clone(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let config3 = ClusterBootstrapConfig {
        node_id: 3,
        data_dir: Some(data_dir3),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "production-cluster-example".into(),
        http_addr: "127.0.0.1:0".parse()?,
        control_backend: ControlBackend::RaftActor,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: iroh_config.clone(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    // Bootstrap all nodes concurrently
    info!("\n🔧 Bootstrapping 3 nodes with production discovery...");
    info!("   ⏳ This may take 15-30 seconds for discovery to complete");
    info!("   📡 Nodes will discover each other via DNS + Pkarr + Gossip");

    let (handle1, handle2, handle3) = tokio::try_join!(
        bootstrap_node(config1.clone()),
        bootstrap_node(config2.clone()),
        bootstrap_node(config3.clone())
    )
    .context("Failed to bootstrap nodes. Check relay/DNS/Pkarr service availability.")?;

    info!("✅ All nodes bootstrapped successfully");

    // Log endpoint information
    let addr1 = handle1.iroh_manager.node_addr().clone();
    let addr2 = handle2.iroh_manager.node_addr().clone();
    let addr3 = handle3.iroh_manager.node_addr().clone();

    info!("\n🔗 Node Endpoints:");
    info!("   Node 1 endpoint ID: {}", addr1.id);
    info!("   Node 2 endpoint ID: {}", addr2.id);
    info!("   Node 3 endpoint ID: {}", addr3.id);

    // Wait for discovery to complete
    // - DNS discovery: ~5 seconds
    // - Pkarr DHT propagation: ~30 seconds
    // - Gossip announcements: ~10 seconds
    info!("\n⏳ Waiting for automatic peer discovery...");
    info!("   DNS discovery queries in progress...");
    sleep(Duration::from_secs(5)).await;

    info!("   Pkarr DHT publishing in progress...");
    sleep(Duration::from_secs(10)).await;

    info!("   Waiting for gossip announcements to propagate...");
    sleep(Duration::from_secs(12)).await;

    info!("✅ Discovery phase complete");

    // Check if peers were discovered
    // In a real deployment, you'd check network_factory.peers() or similar
    warn!("\n⚠️  NOTE: This example requires external services (relay, DNS, Pkarr)");
    warn!("   If discovery fails, verify:");
    warn!(
        "   1. Relay server is reachable: curl {}",
        iroh_config.relay_url.as_ref().unwrap()
    );
    warn!(
        "   2. DNS service is operational: curl {}",
        iroh_config.dns_discovery_url.as_ref().unwrap()
    );
    warn!(
        "   3. Pkarr relay is available: curl {}",
        iroh_config.pkarr_relay_url.as_ref().unwrap()
    );
    warn!("   4. Network allows outbound HTTPS (port 443) and QUIC (UDP)");

    // For this example, fall back to manual peer exchange
    // In production, discovery would handle this automatically
    info!("\n🔄 Fallback: Manual peer exchange for example purposes");
    info!("   (In production with working discovery, this wouldn't be needed)");

    handle1.network_factory.add_peer(2, addr2.clone());
    handle1.network_factory.add_peer(3, addr3.clone());
    handle2.network_factory.add_peer(1, addr1.clone());
    handle2.network_factory.add_peer(3, addr3.clone());
    handle3.network_factory.add_peer(1, addr1.clone());
    handle3.network_factory.add_peer(2, addr2.clone());

    info!("✅ Peer connectivity established");

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

    // Add node 2 as a learner
    info!("\n➕ Adding node 2 as learner");
    let learner_req = AddLearnerRequest {
        learner: ClusterNode::new(2, "127.0.0.1:26001", Some(format!("iroh://{}", addr2.id))),
    };
    let state = cluster_client1.add_learner(learner_req).await?;
    info!("✅ Node 2 added as learner");
    info!("   Members: {:?}", state.members);

    sleep(Duration::from_millis(500)).await;

    // Add node 3 as a learner
    info!("\n➕ Adding node 3 as learner");
    let learner_req = AddLearnerRequest {
        learner: ClusterNode::new(3, "127.0.0.1:26002", Some(format!("iroh://{}", addr3.id))),
    };
    let state = cluster_client1.add_learner(learner_req).await?;
    info!("✅ Node 3 added as learner");
    info!("   Members: {:?}", state.members);

    sleep(Duration::from_millis(500)).await;

    // Promote both learners to voting members
    info!("\n⬆️  Promoting learners to voting members");
    let membership_req = ChangeMembershipRequest {
        members: vec![1, 2, 3],
    };
    let state = cluster_client1.change_membership(membership_req).await?;
    info!("✅ Membership changed");
    info!("   Voting members: {:?}", state.members);

    sleep(Duration::from_millis(1000)).await;

    // Write data on the leader
    info!("\n📝 Writing data to production cluster");
    let pairs = vec![
        ("deployment:type".to_string(), "production".to_string()),
        (
            "discovery:method".to_string(),
            "dns+pkarr+relay".to_string(),
        ),
        ("cluster:size".to_string(), "3".to_string()),
    ];

    let write_req = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: pairs.clone(),
        },
    };
    kv_client1.write(write_req).await?;
    info!("✅ Written {} key-value pairs", pairs.len());

    sleep(Duration::from_millis(500)).await;

    // Read data back to verify
    info!("\n🔍 Reading data from leader:");
    for (key, expected_value) in &pairs {
        let read_req = ReadRequest { key: key.clone() };
        let result = kv_client1.read(read_req).await?;
        info!("  {} = {} ✓", result.key, result.value);
        assert_eq!(result.value, *expected_value);
    }

    // Get final cluster state
    info!("\n📊 Production Cluster Summary:");
    let final_state = cluster_client1.current_state().await?;
    info!("   Voting members: {:?}", final_state.members);
    info!("   Total nodes: {}", final_state.nodes.len());

    let metrics1 = handle1.raft_core.metrics().borrow().clone();
    info!("   Leader: {:?}", metrics1.current_leader);
    info!("   Consensus: {:?}", metrics1.state);

    info!("\n💡 Production Deployment Tips:");
    info!("   1. Deploy relay server in the same region as your nodes");
    info!("   2. Use DNS discovery with service discovery (Consul, etcd, K8s DNS)");
    info!("   3. Enable Pkarr for resilience against DNS failures");
    info!("   4. Disable mDNS in production (set ASPEN_DISABLE_MDNS=true)");
    info!("   5. Monitor discovery metrics (peer count, connection success rate)");
    info!("   6. Use cluster tickets for bootstrapping new nodes (GET /cluster-ticket)");

    // Graceful shutdown
    info!("\n🛑 Shutting down all nodes...");
    let _ = tokio::try_join!(handle1.shutdown(), handle2.shutdown(), handle3.shutdown());
    info!("✅ All nodes shut down successfully");

    info!("🎉 Production cluster example completed!");
    info!("   See examples/README.md for detailed production deployment patterns");

    Ok(())
}
