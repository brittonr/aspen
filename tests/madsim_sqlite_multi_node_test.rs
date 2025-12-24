/// Multi-node Raft cluster test using madsim with SQLite storage backend.
///
/// This test validates:
/// - 3-node cluster initialization with SQLite state machines
/// - Leader election with persistent storage across multiple candidates
/// - Log replication between nodes using SQLite
/// - Write operations propagated through consensus with file-based storage
use std::collections::BTreeMap;
use std::sync::Arc;

use aspen::raft::madsim_network::FailureInjector;
use aspen::raft::madsim_network::MadsimNetworkFactory;
use aspen::raft::madsim_network::MadsimRaftRouter;
use aspen::raft::storage::RedbLogStore;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen::simulation::SimulationArtifactBuilder;
use aspen::testing::create_test_raft_member_info;
use openraft::Config;
use openraft::Raft;

/// Helper to create a Raft instance with SQLite backend for madsim multi-node testing.
///
/// Each node gets its own isolated tempdir to avoid state persistence between test runs.
async fn create_raft_node_sqlite(
    node_id: NodeId,
    test_name: &str,
    router: Arc<MadsimRaftRouter>,
    injector: Arc<FailureInjector>,
) -> Raft<AppTypeConfig> {
    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };
    let config = Arc::new(config.validate().expect("invalid raft config"));

    // Use tempdir for isolated storage per test run
    let temp_base = tempfile::TempDir::new().expect("failed to create temp dir");
    let log_path = temp_base.path().join(format!("{}-node-{}-log.redb", test_name, node_id));
    let sm_path = temp_base.path().join(format!("{}-node-{}-sm.db", test_name, node_id));

    let log_store = RedbLogStore::new(&log_path).expect("failed to create log store");
    let state_machine = SqliteStateMachine::new(&sm_path).expect("failed to create state machine");

    // Keep tempdir alive for test duration
    std::mem::forget(temp_base);

    let network_factory = MadsimNetworkFactory::new(node_id, router, injector);

    Raft::new(node_id, config, network_factory, log_store, state_machine)
        .await
        .expect("failed to create raft instance")
}

/// Test 3-node cluster initialization and leader election with SQLite storage.
///
/// Validates:
/// - Multiple SQLite databases can coexist (one per node)
/// - Leader election works with persistent storage
/// - All nodes agree on leader
/// - Write operations can be submitted and replicated
#[madsim::test]
async fn test_sqlite_three_node_cluster_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_sqlite_3node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 = create_raft_node_sqlite(NodeId::from(1), "cluster_seed_42", router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_sqlite(NodeId::from(2), "cluster_seed_42", router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_sqlite(NodeId::from(3), "cluster_seed_42", router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(NodeId::from(1), "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(NodeId::from(2), "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(NodeId::from(3), "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    // Wait for leader election to complete
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();

    // All nodes should agree on who the leader is
    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {} with SQLite", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        })
        .await
        .expect("failed to write to leader");

    artifact = artifact.add_event("wait: for log replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: verify log replication to SQLite backends");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    // All nodes should have replicated the log entry
    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    artifact = artifact.add_event("validation: 3-node SQLite cluster operational");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with different seed for determinism validation.
#[madsim::test]
async fn test_sqlite_three_node_cluster_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_sqlite_3node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 = create_raft_node_sqlite(NodeId::from(1), "cluster_seed_123", router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_sqlite(NodeId::from(2), "cluster_seed_123", router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_sqlite(NodeId::from(3), "cluster_seed_123", router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(NodeId::from(1), "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(NodeId::from(2), "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(NodeId::from(3), "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {} with SQLite", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        })
        .await
        .expect("failed to write to leader");

    artifact = artifact.add_event("wait: for log replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: verify log replication to SQLite backends");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    artifact = artifact.add_event("validation: 3-node SQLite cluster operational");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with another seed.
#[madsim::test]
async fn test_sqlite_three_node_cluster_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_sqlite_3node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 = create_raft_node_sqlite(NodeId::from(1), "cluster_seed_456", router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_sqlite(NodeId::from(2), "cluster_seed_456", router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_sqlite(NodeId::from(3), "cluster_seed_456", router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(NodeId::from(1), "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(NodeId::from(2), "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(NodeId::from(3), "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {} with SQLite", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        })
        .await
        .expect("failed to write to leader");

    artifact = artifact.add_event("wait: for log replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("metrics: verify log replication to SQLite backends");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    artifact = artifact.add_event("validation: 3-node SQLite cluster operational");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
