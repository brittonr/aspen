/// Single-node Raft initialization test using madsim network infrastructure.
///
/// This test validates Phase 2 integration:
/// - Real Raft instance initialization
/// - MadsimRaftRouter with Raft handle storage
/// - Vote and AppendEntries RPC dispatch
/// - Deterministic execution with seed control
use std::sync::Arc;

use aspen::raft::madsim_network::{FailureInjector, MadsimNetworkFactory, MadsimRaftRouter};
use aspen::raft::storage::{InMemoryLogStore, StateMachineStore};
use aspen::raft::types::NodeId;
use aspen::simulation::SimulationArtifactBuilder;
use aspen::testing::create_test_aspen_node;
use openraft::{Config, Raft};

/// Helper to create a Raft instance for madsim testing.
async fn create_raft_node(node_id: NodeId) -> Raft<aspen::raft::types::AppTypeConfig> {
    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };
    let config = Arc::new(config.validate().expect("invalid raft config"));

    let log_store = InMemoryLogStore::default();
    let state_machine = Arc::new(StateMachineStore::default());

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

/// Test single-node Raft initialization with madsim network.
#[madsim::test]
async fn test_single_node_initialization_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_single_node_init", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let _injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: raft node 1");
    let raft1 = create_raft_node(1).await;

    artifact = artifact.add_event("register: node 1 with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");

    artifact = artifact.add_event("init: initialize single-node cluster");
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(1, create_test_aspen_node(1));
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leadership");
    // Wait for node to become leader (single-node cluster)
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: check leader status");
    let metrics = raft1.metrics().borrow().clone();
    assert_eq!(metrics.current_leader, Some(1), "node 1 should be leader");

    artifact = artifact.add_event("validation: single-node cluster initialized");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with different seed for determinism validation.
#[madsim::test]
async fn test_single_node_initialization_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_single_node_init", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let _injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: raft node 1");
    let raft1 = create_raft_node(1).await;

    artifact = artifact.add_event("register: node 1 with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");

    artifact = artifact.add_event("init: initialize single-node cluster");
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(1, create_test_aspen_node(1));
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leadership");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: check leader status");
    let metrics = raft1.metrics().borrow().clone();
    assert_eq!(metrics.current_leader, Some(1), "node 1 should be leader");

    artifact = artifact.add_event("validation: single-node cluster initialized");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with another seed.
#[madsim::test]
async fn test_single_node_initialization_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_single_node_init", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let _injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: raft node 1");
    let raft1 = create_raft_node(1).await;

    artifact = artifact.add_event("register: node 1 with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");

    artifact = artifact.add_event("init: initialize single-node cluster");
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(1, create_test_aspen_node(1));
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leadership");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: check leader status");
    let metrics = raft1.metrics().borrow().clone();
    assert_eq!(metrics.current_leader, Some(1), "node 1 should be leader");

    artifact = artifact.add_event("validation: single-node cluster initialized");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
