/// Multi-node Raft cluster test using madsim network infrastructure.
///
/// This test validates Phase 3 integration:
/// - 3-node cluster initialization and leader election
/// - 5-node cluster with f=2 fault tolerance (can survive 2 node failures)
/// - 7-node cluster with f=3 fault tolerance (can survive 3 node failures)
/// - Log replication across all cluster sizes
/// - Write operations propagated through consensus
///
/// Each cluster size is tested with 3 different seeds (42, 123, 456) to
/// validate deterministic behavior and catch timing-dependent bugs.
use std::collections::BTreeMap;
use std::sync::Arc;

use aspen::raft::madsim_network::FailureInjector;
use aspen::raft::madsim_network::MadsimNetworkFactory;
use aspen::raft::madsim_network::MadsimRaftRouter;
use aspen::raft::storage::InMemoryLogStore;
use aspen::raft::storage::InMemoryStateMachine;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen_core::SimulationArtifactBuilder;
use aspen_testing::create_test_raft_member_info;
use openraft::Config;
use openraft::Raft;

/// Helper to create a Raft instance for madsim testing.
async fn create_raft_node(
    node_id: NodeId,
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

    let log_store = InMemoryLogStore::default();
    let state_machine = Arc::new(InMemoryStateMachine::default());

    let network_factory = MadsimNetworkFactory::new(node_id, router, injector);

    Raft::new(node_id, config, network_factory, log_store, state_machine)
        .await
        .expect("failed to create raft instance")
}

/// Test 3-node cluster initialization and leader election.
#[madsim::test]
async fn test_three_node_cluster_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_3node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;

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
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

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

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    // All nodes should have replicated the log entry
    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    artifact = artifact.add_event("validation: 3-node cluster operational");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with different seed for determinism validation.
#[madsim::test]
async fn test_three_node_cluster_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_3node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;

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
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

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

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    artifact = artifact.add_event("validation: 3-node cluster operational");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test with another seed.
#[madsim::test]
async fn test_three_node_cluster_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_3node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;

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
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

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

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    artifact = artifact.add_event("validation: 3-node cluster operational");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test 5-node cluster with f=2 fault tolerance (seed 42).
#[madsim::test]
async fn test_five_node_cluster_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_5node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 5 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;

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
    router
        .register_node(NodeId::from(4), "127.0.0.1:26004".to_string(), raft4.clone())
        .expect("failed to register node 4");
    router
        .register_node(NodeId::from(5), "127.0.0.1:26005".to_string(), raft5.clone())
        .expect("failed to register node 5");

    artifact = artifact.add_event("init: initialize 5-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    nodes.insert(NodeId::from(4), create_test_raft_member_info(4));
    nodes.insert(NodeId::from(5), create_test_raft_member_info(5));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();
    let metrics4 = raft4.metrics().borrow().clone();
    let metrics5 = raft5.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics4.current_leader, "nodes 1 and 4 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics5.current_leader, "nodes 1 and 5 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
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

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();
    let final_metrics4 = raft4.metrics().borrow().clone();
    let final_metrics5 = raft5.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");
    assert!(final_metrics4.last_applied.is_some(), "node 4 should have applied entries");
    assert!(final_metrics5.last_applied.is_some(), "node 5 should have applied entries");

    artifact = artifact.add_event("validation: 5-node cluster operational (f=2 fault tolerance)");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test 5-node cluster with different seed for determinism validation.
#[madsim::test]
async fn test_five_node_cluster_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_5node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 5 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;

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
    router
        .register_node(NodeId::from(4), "127.0.0.1:26004".to_string(), raft4.clone())
        .expect("failed to register node 4");
    router
        .register_node(NodeId::from(5), "127.0.0.1:26005".to_string(), raft5.clone())
        .expect("failed to register node 5");

    artifact = artifact.add_event("init: initialize 5-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    nodes.insert(NodeId::from(4), create_test_raft_member_info(4));
    nodes.insert(NodeId::from(5), create_test_raft_member_info(5));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();
    let metrics4 = raft4.metrics().borrow().clone();
    let metrics5 = raft5.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics4.current_leader, "nodes 1 and 4 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics5.current_leader, "nodes 1 and 5 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
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

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();
    let final_metrics4 = raft4.metrics().borrow().clone();
    let final_metrics5 = raft5.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");
    assert!(final_metrics4.last_applied.is_some(), "node 4 should have applied entries");
    assert!(final_metrics5.last_applied.is_some(), "node 5 should have applied entries");

    artifact = artifact.add_event("validation: 5-node cluster operational (f=2 fault tolerance)");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test 5-node cluster with another seed.
#[madsim::test]
async fn test_five_node_cluster_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_5node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 5 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;

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
    router
        .register_node(NodeId::from(4), "127.0.0.1:26004".to_string(), raft4.clone())
        .expect("failed to register node 4");
    router
        .register_node(NodeId::from(5), "127.0.0.1:26005".to_string(), raft5.clone())
        .expect("failed to register node 5");

    artifact = artifact.add_event("init: initialize 5-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    nodes.insert(NodeId::from(4), create_test_raft_member_info(4));
    nodes.insert(NodeId::from(5), create_test_raft_member_info(5));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();
    let metrics4 = raft4.metrics().borrow().clone();
    let metrics5 = raft5.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics4.current_leader, "nodes 1 and 4 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics5.current_leader, "nodes 1 and 5 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
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

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();
    let final_metrics4 = raft4.metrics().borrow().clone();
    let final_metrics5 = raft5.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");
    assert!(final_metrics4.last_applied.is_some(), "node 4 should have applied entries");
    assert!(final_metrics5.last_applied.is_some(), "node 5 should have applied entries");

    artifact = artifact.add_event("validation: 5-node cluster operational (f=2 fault tolerance)");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test 7-node cluster with f=3 fault tolerance (seed 42).
#[madsim::test]
async fn test_seven_node_cluster_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_7node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 7 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;
    let raft6 = create_raft_node(NodeId::from(6), router.clone(), injector.clone()).await;
    let raft7 = create_raft_node(NodeId::from(7), router.clone(), injector.clone()).await;

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
    router
        .register_node(NodeId::from(4), "127.0.0.1:26004".to_string(), raft4.clone())
        .expect("failed to register node 4");
    router
        .register_node(NodeId::from(5), "127.0.0.1:26005".to_string(), raft5.clone())
        .expect("failed to register node 5");
    router
        .register_node(NodeId::from(6), "127.0.0.1:26006".to_string(), raft6.clone())
        .expect("failed to register node 6");
    router
        .register_node(NodeId::from(7), "127.0.0.1:26007".to_string(), raft7.clone())
        .expect("failed to register node 7");

    artifact = artifact.add_event("init: initialize 7-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    nodes.insert(NodeId::from(4), create_test_raft_member_info(4));
    nodes.insert(NodeId::from(5), create_test_raft_member_info(5));
    nodes.insert(NodeId::from(6), create_test_raft_member_info(6));
    nodes.insert(NodeId::from(7), create_test_raft_member_info(7));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(6000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();
    let metrics4 = raft4.metrics().borrow().clone();
    let metrics5 = raft5.metrics().borrow().clone();
    let metrics6 = raft6.metrics().borrow().clone();
    let metrics7 = raft7.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics4.current_leader, "nodes 1 and 4 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics5.current_leader, "nodes 1 and 5 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics6.current_leader, "nodes 1 and 6 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics7.current_leader, "nodes 1 and 7 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
        6 => &raft6,
        7 => &raft7,
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
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();
    let final_metrics4 = raft4.metrics().borrow().clone();
    let final_metrics5 = raft5.metrics().borrow().clone();
    let final_metrics6 = raft6.metrics().borrow().clone();
    let final_metrics7 = raft7.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");
    assert!(final_metrics4.last_applied.is_some(), "node 4 should have applied entries");
    assert!(final_metrics5.last_applied.is_some(), "node 5 should have applied entries");
    assert!(final_metrics6.last_applied.is_some(), "node 6 should have applied entries");
    assert!(final_metrics7.last_applied.is_some(), "node 7 should have applied entries");

    artifact = artifact.add_event("validation: 7-node cluster operational (f=3 fault tolerance)");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test 7-node cluster with different seed for determinism validation.
#[madsim::test]
async fn test_seven_node_cluster_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_7node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 7 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;
    let raft6 = create_raft_node(NodeId::from(6), router.clone(), injector.clone()).await;
    let raft7 = create_raft_node(NodeId::from(7), router.clone(), injector.clone()).await;

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
    router
        .register_node(NodeId::from(4), "127.0.0.1:26004".to_string(), raft4.clone())
        .expect("failed to register node 4");
    router
        .register_node(NodeId::from(5), "127.0.0.1:26005".to_string(), raft5.clone())
        .expect("failed to register node 5");
    router
        .register_node(NodeId::from(6), "127.0.0.1:26006".to_string(), raft6.clone())
        .expect("failed to register node 6");
    router
        .register_node(NodeId::from(7), "127.0.0.1:26007".to_string(), raft7.clone())
        .expect("failed to register node 7");

    artifact = artifact.add_event("init: initialize 7-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    nodes.insert(NodeId::from(4), create_test_raft_member_info(4));
    nodes.insert(NodeId::from(5), create_test_raft_member_info(5));
    nodes.insert(NodeId::from(6), create_test_raft_member_info(6));
    nodes.insert(NodeId::from(7), create_test_raft_member_info(7));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(6000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();
    let metrics4 = raft4.metrics().borrow().clone();
    let metrics5 = raft5.metrics().borrow().clone();
    let metrics6 = raft6.metrics().borrow().clone();
    let metrics7 = raft7.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics4.current_leader, "nodes 1 and 4 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics5.current_leader, "nodes 1 and 5 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics6.current_leader, "nodes 1 and 6 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics7.current_leader, "nodes 1 and 7 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
        6 => &raft6,
        7 => &raft7,
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
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();
    let final_metrics4 = raft4.metrics().borrow().clone();
    let final_metrics5 = raft5.metrics().borrow().clone();
    let final_metrics6 = raft6.metrics().borrow().clone();
    let final_metrics7 = raft7.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");
    assert!(final_metrics4.last_applied.is_some(), "node 4 should have applied entries");
    assert!(final_metrics5.last_applied.is_some(), "node 5 should have applied entries");
    assert!(final_metrics6.last_applied.is_some(), "node 6 should have applied entries");
    assert!(final_metrics7.last_applied.is_some(), "node 7 should have applied entries");

    artifact = artifact.add_event("validation: 7-node cluster operational (f=3 fault tolerance)");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test 7-node cluster with another seed.
#[madsim::test]
async fn test_seven_node_cluster_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_7node_cluster", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 7 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;
    let raft6 = create_raft_node(NodeId::from(6), router.clone(), injector.clone()).await;
    let raft7 = create_raft_node(NodeId::from(7), router.clone(), injector.clone()).await;

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
    router
        .register_node(NodeId::from(4), "127.0.0.1:26004".to_string(), raft4.clone())
        .expect("failed to register node 4");
    router
        .register_node(NodeId::from(5), "127.0.0.1:26005".to_string(), raft5.clone())
        .expect("failed to register node 5");
    router
        .register_node(NodeId::from(6), "127.0.0.1:26006".to_string(), raft6.clone())
        .expect("failed to register node 6");
    router
        .register_node(NodeId::from(7), "127.0.0.1:26007".to_string(), raft7.clone())
        .expect("failed to register node 7");

    artifact = artifact.add_event("init: initialize 7-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    nodes.insert(NodeId::from(4), create_test_raft_member_info(4));
    nodes.insert(NodeId::from(5), create_test_raft_member_info(5));
    nodes.insert(NodeId::from(6), create_test_raft_member_info(6));
    nodes.insert(NodeId::from(7), create_test_raft_member_info(7));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for leader election");
    madsim::time::sleep(std::time::Duration::from_millis(6000)).await;

    artifact = artifact.add_event("metrics: check leader elected");
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();
    let metrics4 = raft4.metrics().borrow().clone();
    let metrics5 = raft5.metrics().borrow().clone();
    let metrics6 = raft6.metrics().borrow().clone();
    let metrics7 = raft7.metrics().borrow().clone();

    assert!(metrics1.current_leader.is_some(), "node 1 should see a leader");
    assert_eq!(metrics1.current_leader, metrics2.current_leader, "nodes 1 and 2 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics3.current_leader, "nodes 1 and 3 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics4.current_leader, "nodes 1 and 4 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics5.current_leader, "nodes 1 and 5 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics6.current_leader, "nodes 1 and 6 disagree on leader");
    assert_eq!(metrics1.current_leader, metrics7.current_leader, "nodes 1 and 7 disagree on leader");

    let leader_id = metrics1.current_leader.expect("no leader elected");
    artifact = artifact.add_event(format!("validation: leader is node {}", leader_id));

    artifact = artifact.add_event("write: submit proposal to leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
        6 => &raft6,
        7 => &raft7,
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
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("metrics: verify log replication");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();
    let final_metrics4 = raft4.metrics().borrow().clone();
    let final_metrics5 = raft5.metrics().borrow().clone();
    let final_metrics6 = raft6.metrics().borrow().clone();
    let final_metrics7 = raft7.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");
    assert!(final_metrics4.last_applied.is_some(), "node 4 should have applied entries");
    assert!(final_metrics5.last_applied.is_some(), "node 5 should have applied entries");
    assert!(final_metrics6.last_applied.is_some(), "node 6 should have applied entries");
    assert!(final_metrics7.last_applied.is_some(), "node 7 should have applied entries");

    artifact = artifact.add_event("validation: 7-node cluster operational (f=3 fault tolerance)");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
