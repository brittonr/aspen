//! Key-Value Operations Example
//!
//! This example demonstrates:
//! - Bootstrapping a single-node cluster
//! - Writing key-value pairs (Set and SetMulti)
//! - Reading values back
//! - Handling NotFound errors
//! - Linearizable read consistency
//!
//! Run with: cargo run --example kv_operations

use anyhow::Result;
use aspen::api::{
    ClusterController, ClusterNode, InitRequest, KeyValueStore, KeyValueStoreError, ReadRequest,
    WriteCommand, WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::RaftControlClient;
use tempfile::TempDir;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Aspen Key-Value Operations Example");

    // Create temporary directory for data
    let temp_dir = TempDir::new()?;
    let data_dir = temp_dir.path().to_path_buf();
    info!("📁 Data directory: {}", data_dir.display());

    // Configure the node
    let config = ClusterBootstrapConfig {
        node_id: 100,
        data_dir: Some(data_dir.clone()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "kv-operations-example".into(),
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
    };

    info!("🔧 Bootstrapping node {}", config.node_id);
    let handle = bootstrap_node(config.clone()).await?;
    info!("✅ Node bootstrapped successfully");

    // Create clients
    let cluster_client = RaftControlClient::new(handle.raft_actor.clone());
    let kv_client = KvClient::new(handle.raft_actor.clone());

    // Initialize cluster
    info!("🏗️  Initializing cluster");
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            100,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };
    cluster_client.init(init_req).await?;
    info!("✅ Cluster initialized");

    // Example 1: Simple Set and Read
    info!("\n📝 Example 1: Simple Set and Read");
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: "app:name".into(),
            value: "Aspen".into(),
        },
    };
    kv_client.write(write_req).await?;
    info!("✅ Written: app:name = Aspen");

    let read_req = ReadRequest {
        key: "app:name".into(),
    };
    let result = kv_client.read(read_req).await?;
    info!(
        "✅ Read: {} = {} (linearizable read)",
        result.key, result.value
    );

    // Example 2: Multiple individual writes
    info!("\n📝 Example 2: Multiple Individual Writes");
    for i in 1..=5 {
        let key = format!("counter:{}", i);
        let value = format!("value_{}", i);
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: key.clone(),
                value: value.clone(),
            },
        };
        kv_client.write(write_req).await?;
        info!("✅ Written: {} = {}", key, value);
    }

    // Example 3: Bulk write with SetMulti
    info!("\n📝 Example 3: Bulk Write with SetMulti");
    let pairs = vec![
        ("config:timeout".to_string(), "30".to_string()),
        ("config:retries".to_string(), "3".to_string()),
        ("config:host".to_string(), "localhost".to_string()),
        ("config:port".to_string(), "8080".to_string()),
    ];
    let write_req = WriteRequest {
        command: WriteCommand::SetMulti {
            pairs: pairs.clone(),
        },
    };
    kv_client.write(write_req).await?;
    info!("✅ Written {} key-value pairs atomically", pairs.len());

    // Read back all config values
    info!("🔍 Reading back config values:");
    for (key, expected_value) in &pairs {
        let read_req = ReadRequest { key: key.clone() };
        let result = kv_client.read(read_req).await?;
        info!("  {} = {} ✓", result.key, result.value);
        assert_eq!(result.value, *expected_value);
    }

    // Example 4: Reading all counters
    info!("\n📝 Example 4: Reading All Counters");
    for i in 1..=5 {
        let key = format!("counter:{}", i);
        let read_req = ReadRequest { key: key.clone() };
        let result = kv_client.read(read_req).await?;
        info!("  {} = {}", result.key, result.value);
    }

    // Example 5: Handling NotFound errors
    info!("\n📝 Example 5: Handling NotFound Errors");
    let nonexistent_keys = vec!["missing:key1", "missing:key2", "foo:bar"];
    for key in nonexistent_keys {
        let read_req = ReadRequest {
            key: key.to_string(),
        };
        match kv_client.read(read_req).await {
            Ok(result) => {
                warn!("Unexpected: {} = {}", result.key, result.value);
            }
            Err(KeyValueStoreError::NotFound { key }) => {
                info!("✅ Key '{}' not found (as expected)", key);
            }
            Err(e) => {
                error!("❌ Unexpected error: {}", e);
                return Err(e.into());
            }
        }
    }

    // Example 6: Overwrite existing key
    info!("\n📝 Example 6: Overwriting Existing Key");
    let key = "app:version";
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: key.into(),
            value: "1.0.0".into(),
        },
    };
    kv_client.write(write_req).await?;
    info!("✅ Written: {} = 1.0.0", key);

    let read_req = ReadRequest { key: key.into() };
    let result = kv_client.read(read_req).await?;
    info!("✅ Read: {} = {}", result.key, result.value);

    // Overwrite with new value
    let write_req = WriteRequest {
        command: WriteCommand::Set {
            key: key.into(),
            value: "2.0.0".into(),
        },
    };
    kv_client.write(write_req).await?;
    info!("✅ Overwritten: {} = 2.0.0", key);

    let read_req = ReadRequest { key: key.into() };
    let result = kv_client.read(read_req).await?;
    info!("✅ Read: {} = {} (updated value)", result.key, result.value);
    assert_eq!(result.value, "2.0.0");

    // Summary
    info!("\n📊 Summary:");
    info!("  - Demonstrated Set and SetMulti commands");
    info!("  - All reads are linearizable (go through Raft consensus)");
    info!("  - Handled NotFound errors gracefully");
    info!("  - Showed atomic multi-key writes");

    // Shutdown
    info!("\n🛑 Shutting down...");
    handle.shutdown().await?;
    info!("✅ Shutdown complete");

    info!("🎉 Key-value operations example completed successfully!");

    Ok(())
}
