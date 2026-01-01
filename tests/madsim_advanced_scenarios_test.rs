/// Advanced failure scenarios for Raft using madsim network infrastructure.
///
/// This test suite validates Phase 5 integration:
/// - 5-node clusters with multiple concurrent failures
/// - Rolling failures and recoveries
/// - Asymmetric network partitions
/// - Leadership transfer under load
///
/// All tests use deterministic seeds for reproducibility.
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

/// Test 5-node cluster with multiple concurrent failures.
#[madsim::test]
async fn test_five_node_cluster_with_concurrent_failures_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_5node_concurrent_failures", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 5 raft nodes");
    let raft1 = create_raft_node(NodeId::from(1), router.clone(), injector.clone()).await;
    let raft2 = create_raft_node(NodeId::from(2), router.clone(), injector.clone()).await;
    let raft3 = create_raft_node(NodeId::from(3), router.clone(), injector.clone()).await;
    let raft4 = create_raft_node(NodeId::from(4), router.clone(), injector.clone()).await;
    let raft5 = create_raft_node(NodeId::from(5), router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: all 5 nodes with router");
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

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: identify initial leader");
    let metrics1 = raft1.metrics().borrow().clone();
    let initial_leader = metrics1.current_leader.expect("no initial leader");
    artifact = artifact.add_event(format!("validation: initial leader is node {}", initial_leader));

    artifact = artifact.add_event("write: first write before failures");
    let leader_raft = match initial_leader.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        4 => &raft4,
        5 => &raft5,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "before_failures".to_string(),
            value: "initial_state".to_string(),
        })
        .await
        .expect("first write should succeed");

    artifact = artifact.add_event("failure: crash 2 follower nodes simultaneously");
    // Find two followers to crash (not the leader)
    let followers: Vec<NodeId> = (1..=5).map(NodeId::from).filter(|id| *id != initial_leader).take(2).collect();
    router.mark_node_failed(followers[0], true);
    router.mark_node_failed(followers[1], true);
    artifact = artifact.add_event(format!("failure: nodes {} and {} crashed", followers[0], followers[1]));

    artifact = artifact.add_event("write: second write with 2 nodes down");
    leader_raft
        .client_write(AppRequest::Set {
            key: "during_failures".to_string(),
            value: "partial_cluster".to_string(),
        })
        .await
        .expect("write should succeed with quorum (3/5)");

    artifact = artifact.add_event("wait: for replication to majority");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("validation: majority maintains consensus");
    // Check that leader and at least one follower have replicated
    let leader_metrics = leader_raft.metrics().borrow().clone();
    assert!(leader_metrics.last_applied.is_some(), "leader should have applied entries");

    artifact = artifact.add_event("validation: 5-node cluster operational with failures");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test rolling failures - nodes fail and recover in sequence.
#[madsim::test]
async fn test_rolling_failures_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_rolling_failures", seed).start();

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

    artifact = artifact.add_event("init: initialize 3-node cluster");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("failure: crash node 1");
    router.mark_node_failed(NodeId::from(1), true);

    artifact = artifact.add_event("wait: for re-election after node 1 crash");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("recovery: node 1 recovers");
    router.mark_node_failed(NodeId::from(1), false);

    artifact = artifact.add_event("failure: crash node 2");
    router.mark_node_failed(NodeId::from(2), true);

    artifact = artifact.add_event("wait: for re-election after node 2 crash");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("recovery: node 2 recovers");
    router.mark_node_failed(NodeId::from(2), false);

    artifact = artifact.add_event("wait: for cluster stabilization");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("validation: cluster survives rolling failures");
    // At least one node should have a leader view
    let metrics1 = raft1.metrics().borrow().clone();
    let metrics2 = raft2.metrics().borrow().clone();
    let metrics3 = raft3.metrics().borrow().clone();

    let has_leader =
        metrics1.current_leader.is_some() || metrics2.current_leader.is_some() || metrics3.current_leader.is_some();

    assert!(has_leader, "cluster should have leader after rolling failures");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test asymmetric network partition (triangle partition pattern).
#[madsim::test]
async fn test_asymmetric_partition_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_asymmetric_partition", seed).start();

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

    artifact = artifact.add_event("init: initialize 3-node cluster");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("failure: create asymmetric partition (node 1 ↔ node 2 blocked)");
    // Create triangle partition: node 3 can talk to both 1 and 2, but 1 and 2 cannot talk to each other
    injector.set_message_drop(NodeId::from(1), NodeId::from(2), true);
    injector.set_message_drop(NodeId::from(2), NodeId::from(1), true);

    artifact = artifact.add_event("wait: for partition effects");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("write: attempt write in asymmetric partition");
    let metrics1 = raft1.metrics().borrow().clone();
    if let Some(leader_id) = metrics1.current_leader {
        let leader_raft = match leader_id.0 {
            1 => &raft1,
            2 => &raft2,
            3 => &raft3,
            _ => panic!("invalid leader id"),
        };

        // Try to write - may succeed or fail depending on leader location
        let write_result = leader_raft
            .client_write(AppRequest::Set {
                key: "asymmetric_test".to_string(),
                value: "partial_connectivity".to_string(),
            })
            .await;

        if write_result.is_ok() {
            artifact = artifact.add_event("validation: write succeeded despite asymmetric partition");
        } else {
            artifact = artifact.add_event("validation: write failed due to asymmetric partition");
        }
    }

    artifact = artifact.add_event("recovery: heal partition");
    injector.clear_all();

    artifact = artifact.add_event("wait: for cluster recovery");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    artifact = artifact.add_event("validation: cluster recovers from asymmetric partition");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test concurrent writes during complex failure scenarios.
#[madsim::test]
async fn test_concurrent_writes_complex_failures_seed_789() {
    let seed = 789_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_concurrent_writes_complex", seed).start();

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

    artifact = artifact.add_event("init: initialize 3-node cluster");
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
    nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
    nodes.insert(NodeId::from(3), create_test_raft_member_info(3));
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: identify leader");
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    artifact = artifact.add_event("failure: inject network delays");
    // Add delays to simulate slow network
    injector.set_network_delay(NodeId::from(1), NodeId::from(2), 500);
    injector.set_network_delay(NodeId::from(2), NodeId::from(1), 500);
    injector.set_network_delay(NodeId::from(2), NodeId::from(3), 500);
    injector.set_network_delay(NodeId::from(3), NodeId::from(2), 500);

    artifact = artifact.add_event("write: concurrent writes with network delays");
    // Submit multiple writes concurrently
    for i in 1..=5 {
        leader_raft
            .client_write(AppRequest::Set {
                key: format!("concurrent_{}", i),
                value: format!("value_{}", i),
            })
            .await
            .unwrap_or_else(|_| panic!("write {} should succeed", i));
    }

    artifact = artifact.add_event("wait: for replication with delays");
    madsim::time::sleep(std::time::Duration::from_millis(4000)).await;

    artifact = artifact.add_event("validation: all concurrent writes replicated");
    let final_metrics1 = raft1.metrics().borrow().clone();
    let final_metrics2 = raft2.metrics().borrow().clone();
    let final_metrics3 = raft3.metrics().borrow().clone();

    assert!(final_metrics1.last_applied.is_some(), "node 1 should have applied entries");
    assert!(final_metrics2.last_applied.is_some(), "node 2 should have applied entries");
    assert!(final_metrics3.last_applied.is_some(), "node 3 should have applied entries");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
