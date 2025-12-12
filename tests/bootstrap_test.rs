use std::time::Duration;

use aspen::cluster::bootstrap::{bootstrap_node, load_config};
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::cluster::metadata::NodeStatus;
use tempfile::TempDir;
use tokio::time::sleep;

/// Test that a single node can bootstrap successfully.
#[tokio::test]
async fn test_bootstrap_single_node() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    let config = ClusterBootstrapConfig {
        node_id: 100, // Use high node ID to avoid collision with other tests
        data_dir: Some(data_dir.clone()),
        host: "127.0.0.1".into(),
        ractor_port: 0, // OS-assigned port
        cookie: "test-cookie".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::Deterministic,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    // Bootstrap the node
    let handle = bootstrap_node(config.clone()).await.unwrap();

    // Verify node is registered in metadata store
    let metadata = handle.metadata_store.get_node(100).unwrap().unwrap();
    assert_eq!(metadata.node_id, 100);
    assert_eq!(metadata.status, NodeStatus::Online);

    // Verify Raft core is initialized
    let metrics = handle.raft_core.metrics().borrow().clone();
    assert_eq!(metrics.id, 100);

    // Shutdown gracefully
    handle.shutdown().await.unwrap();
}

/// Test that multiple nodes can bootstrap concurrently.
#[tokio::test]
async fn test_bootstrap_multiple_nodes() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    let temp_dir3 = TempDir::new().unwrap();

    let config1 = ClusterBootstrapConfig {
        node_id: 201, // Use high node IDs to avoid collision
        data_dir: Some(temp_dir1.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-cookie".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::Deterministic,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir1.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir1.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let config2 = ClusterBootstrapConfig {
        node_id: 202,
        data_dir: Some(temp_dir2.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-cookie".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::Deterministic,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir2.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir2.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let config3 = ClusterBootstrapConfig {
        node_id: 203,
        data_dir: Some(temp_dir3.path().to_path_buf()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-cookie".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::Deterministic,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir3.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir3.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    // Bootstrap all nodes concurrently
    let (handle1, handle2, handle3) = tokio::try_join!(
        bootstrap_node(config1),
        bootstrap_node(config2),
        bootstrap_node(config3),
    )
    .unwrap();

    // Verify all nodes are registered
    assert_eq!(
        handle1
            .metadata_store
            .get_node(201)
            .unwrap()
            .unwrap()
            .node_id,
        201
    );
    assert_eq!(
        handle2
            .metadata_store
            .get_node(202)
            .unwrap()
            .unwrap()
            .node_id,
        202
    );
    assert_eq!(
        handle3
            .metadata_store
            .get_node(203)
            .unwrap()
            .unwrap()
            .node_id,
        203
    );

    // Shutdown all nodes
    let _ = tokio::try_join!(handle1.shutdown(), handle2.shutdown(), handle3.shutdown());
}

/// Test configuration loading with TOML file.
#[test]
fn test_load_config_from_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
        node_id = 42
        host = "192.168.1.1"
        ractor_port = 27000
        cookie = "production-cookie"
        http_addr = "0.0.0.0:8080"
        control_backend = "raft_actor"
        heartbeat_interval_ms = 1000
        election_timeout_min_ms = 2000
        election_timeout_max_ms = 5000

        [iroh]
        secret_key = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    "#;
    std::fs::write(&config_path, toml_content).unwrap();

    // Load with default CLI config (no explicit overrides)
    // Note: control_backend defaults to RaftActor in config.rs
    let cli_config = ClusterBootstrapConfig {
        node_id: 0, // No explicit override, TOML will provide value
        data_dir: None,
        host: "127.0.0.1".into(),      // Default value
        ractor_port: 26000,            // Default value
        cookie: "aspen-cookie".into(), // Default value
        http_addr: "127.0.0.1:8080".parse().unwrap(),
        control_backend: ControlBackend::default(), // Use actual default
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

    let config = load_config(Some(&config_path), cli_config).unwrap();

    // TOML values should override defaults
    assert_eq!(config.node_id, 42);
    assert_eq!(config.host, "192.168.1.1");
    assert_eq!(config.ractor_port, 27000);
    assert_eq!(config.cookie, "production-cookie");
    assert_eq!(config.control_backend, ControlBackend::RaftActor);
    assert_eq!(config.heartbeat_interval_ms, 1000);
    assert_eq!(
        config.iroh.secret_key,
        Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into())
    );
}

/// Test configuration precedence (env < TOML < CLI).
#[test]
fn test_config_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
        node_id = 1
        ractor_port = 26001
    "#;
    std::fs::write(&config_path, toml_content).unwrap();

    // CLI overrides
    let cli_config = ClusterBootstrapConfig {
        node_id: 2, // CLI override
        data_dir: None,
        host: "127.0.0.1".into(),
        ractor_port: 26000, // Default, should be overridden by TOML
        cookie: "test-cookie".into(),
        http_addr: "127.0.0.1:8080".parse().unwrap(),
        control_backend: ControlBackend::Deterministic,
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

    let config = load_config(Some(&config_path), cli_config).unwrap();

    // CLI should win for node_id
    assert_eq!(config.node_id, 2);

    // TOML should override default for ractor_port
    assert_eq!(config.ractor_port, 26001);
}

/// Test graceful shutdown updates node status.
#[tokio::test]
async fn test_shutdown_updates_status() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    let config = ClusterBootstrapConfig {
        node_id: 300, // Use high node ID to avoid collision
        data_dir: Some(data_dir.clone()),
        host: "127.0.0.1".into(),
        ractor_port: 0,
        cookie: "test-cookie".into(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        control_backend: ControlBackend::Deterministic,
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir.path().join("state-machine.db")),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await.unwrap();

    // Verify node is online
    let metadata = handle.metadata_store.get_node(300).unwrap().unwrap();
    assert_eq!(metadata.status, NodeStatus::Online);

    // Keep a reference to the metadata store
    let metadata_store = handle.metadata_store.clone();

    // Shutdown
    handle.shutdown().await.unwrap();

    // Give it a moment to update
    sleep(Duration::from_millis(100)).await;

    // Verify node status was updated to offline
    let metadata = metadata_store.get_node(300).unwrap().unwrap();
    assert_eq!(metadata.status, NodeStatus::Offline);
}
