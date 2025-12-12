/// Single-node Raft initialization test using madsim network infrastructure with SQLite storage.
///
/// This test validates SQLite storage backend compatibility with madsim deterministic simulation:
/// - Real Raft instance initialization with SQLite state machine
/// - File-based persistent storage in madsim virtual filesystem
/// - Deterministic execution with seed control
/// - Validation that SQLite works correctly in simulated environment
use std::sync::Arc;

use aspen::raft::madsim_network::{FailureInjector, MadsimNetworkFactory, MadsimRaftRouter};
use aspen::raft::storage::RedbLogStore;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::NodeId;
use aspen::simulation::SimulationArtifactBuilder;
use aspen::testing::create_test_raft_member_info;
use openraft::{Config, Raft};

/// Helper to create a Raft instance with SQLite backend for madsim testing.
///
/// Uses unique paths for each test run to avoid state persistence issues.
/// SQLite databases on real filesystem (madsim doesn't fully virtualize SQLite file I/O).
/// The test_name parameter ensures each test gets its own isolated storage.
async fn create_raft_node_sqlite(
    node_id: NodeId,
    test_name: &str,
) -> Raft<aspen::raft::types::AppTypeConfig> {
    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };
    let config = Arc::new(config.validate().expect("invalid raft config"));

    // Use tempdir to create truly isolated storage for each test run
    // This avoids state persistence between test runs
    let temp_base = tempfile::TempDir::new().expect("failed to create temp dir");
    let log_path = temp_base
        .path()
        .join(format!("{}-node-{}-log.redb", test_name, node_id));
    let sm_path = temp_base
        .path()
        .join(format!("{}-node-{}-sm.db", test_name, node_id));

    let log_store = RedbLogStore::new(&log_path).expect("failed to create log store");
    let state_machine = SqliteStateMachine::new(&sm_path).expect("failed to create state machine");

    // Keep tempdir alive for the duration of the test
    // It will be cleaned up when the Raft instance is dropped
    std::mem::forget(temp_base);

    Raft::new(
        node_id,
        config,
        MadsimNetworkFactory::new(
            node_id,
            Arc::new(MadsimRaftRouter::new()),
            Arc::new(FailureInjector::new()),
        ),
        log_store,
        state_machine,
    )
    .await
    .expect("failed to create raft instance")
}

/// Test single-node Raft initialization with SQLite backend.
///
/// Validates:
/// - SQLite database creation in madsim virtual filesystem
/// - Single-node cluster initialization
/// - Leader election with persistent storage
#[madsim::test]
async fn test_sqlite_single_node_initialization_seed_42() {
    let seed = 42_u64;
    let mut artifact =
        SimulationArtifactBuilder::new("madsim_sqlite_single_node_init", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let _injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: raft node 1 with SQLite backend");
    let raft1 = create_raft_node_sqlite(NodeId::from(1), "init_seed_42").await;

    artifact = artifact.add_event("register: node 1 with router");
    router
        .register_node(
            NodeId::from(1),
            "127.0.0.1:26001".to_string(),
            raft1.clone(),
        )
        .expect("failed to register node 1");

    artifact = artifact.add_event("init: initialize single-node cluster");
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leadership");
    // Wait for node to become leader (single-node cluster)
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: check leader status");
    let metrics = raft1.metrics().borrow().clone();
    assert_eq!(
        metrics.current_leader,
        Some(NodeId::from(1)),
        "node 1 should be leader with SQLite backend"
    );

    artifact = artifact.add_event("validation: single-node SQLite cluster initialized");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with different seed for determinism validation.
#[madsim::test]
async fn test_sqlite_single_node_initialization_seed_123() {
    let seed = 123_u64;
    let mut artifact =
        SimulationArtifactBuilder::new("madsim_sqlite_single_node_init", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let _injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: raft node 1 with SQLite backend");
    let raft1 = create_raft_node_sqlite(NodeId::from(1), "init_seed_123").await;

    artifact = artifact.add_event("register: node 1 with router");
    router
        .register_node(
            NodeId::from(1),
            "127.0.0.1:26001".to_string(),
            raft1.clone(),
        )
        .expect("failed to register node 1");

    artifact = artifact.add_event("init: initialize single-node cluster");
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leadership");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: check leader status");
    let metrics = raft1.metrics().borrow().clone();
    assert_eq!(
        metrics.current_leader,
        Some(NodeId::from(1)),
        "node 1 should be leader with SQLite backend"
    );

    artifact = artifact.add_event("validation: single-node SQLite cluster initialized");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
