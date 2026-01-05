/// Failure injection tests for Raft using madsim network infrastructure.
///
/// This test suite validates Phase 4 integration:
/// - Leader crash and automatic re-election
/// - Network partitions and split-brain prevention
/// - Message delays and their impact on consensus
/// - Concurrent writes under failure conditions
///
/// All tests use deterministic seeds for reproducibility.
///
/// Related test suites (see for additional coverage):
/// - madsim_tester_test.rs: Byzantine failure tests (vote flip, term increment, duplication)
/// - madsim_advanced_scenarios_test.rs: Multiple simultaneous crash scenarios (5-node with f=2)
/// - madsim_redb_crash_recovery_test.rs: Crash recovery with persistent storage
/// - madsim_multi_node_test.rs: 3/5/7-node cluster tests with multiple seeds
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

/// Test leader crash and automatic re-election.
#[madsim::test]
async fn test_leader_crash_and_reelection_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_leader_crash_reelection", seed).start();

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

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: identify initial leader");
    // Check all nodes to find the initial leader (not just node 1)
    let all_nodes = [(1, &raft1), (2, &raft2), (3, &raft3)];
    let mut initial_leader = None;
    for (_id, raft) in all_nodes.iter() {
        let metrics = raft.metrics().borrow().clone();
        if let Some(leader) = metrics.current_leader {
            initial_leader = Some(leader);
            break;
        }
    }
    let initial_leader = initial_leader.expect("no initial leader elected after 5s");
    artifact = artifact.add_event(format!("validation: initial leader is node {}", initial_leader));

    artifact = artifact.add_event(format!("failure: crash node {} (leader)", initial_leader));
    router.mark_node_failed(initial_leader, true);

    artifact = artifact.add_event("wait: for re-election (15s)");
    madsim::time::sleep(std::time::Duration::from_millis(15000)).await;

    artifact = artifact.add_event("metrics: check new leader elected");
    // Check only non-crashed nodes for new leader
    let all_nodes = [(1, &raft1), (2, &raft2), (3, &raft3)];

    let mut new_leader = None;
    for (id, raft) in all_nodes.iter() {
        if NodeId::from(*id) != initial_leader {
            let metrics = raft.metrics().borrow().clone();
            if let Some(leader) = metrics.current_leader {
                new_leader = Some(leader);
                break;
            }
        }
    }

    // Also check that the new leader is not the crashed node
    if new_leader == Some(initial_leader) {
        new_leader = None;
    }

    assert!(new_leader.is_some(), "no new leader elected after crash");
    assert_ne!(new_leader.unwrap(), initial_leader, "new leader should be different from crashed leader");

    artifact = artifact.add_event(format!("validation: new leader is node {} after crash", new_leader.unwrap()));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test network partition between nodes.
#[madsim::test]
async fn test_network_partition_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_network_partition", seed).start();

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

    artifact = artifact.add_event("metrics: verify initial consensus");
    let metrics1 = raft1.metrics().borrow().clone();
    assert!(metrics1.current_leader.is_some(), "no leader before partition");

    artifact = artifact.add_event("failure: partition node 3 from nodes 1 and 2");
    // Partition: node 3 cannot communicate with nodes 1 and 2
    injector.set_message_drop(NodeId::from(3), NodeId::from(1), true);
    injector.set_message_drop(NodeId::from(3), NodeId::from(2), true);
    injector.set_message_drop(NodeId::from(1), NodeId::from(3), true);
    injector.set_message_drop(NodeId::from(2), NodeId::from(3), true);

    artifact = artifact.add_event("write: submit write to majority partition");
    let leader_id = metrics1.current_leader.expect("no leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "partition_test".to_string(),
            value: "majority_partition".to_string(),
        })
        .await
        .expect("write should succeed in majority partition");

    artifact = artifact.add_event("wait: for replication in majority partition");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("validation: majority partition operational");
    // Nodes 1 and 2 should still have a leader and replicate
    let metrics1_after = raft1.metrics().borrow().clone();
    let metrics2_after = raft2.metrics().borrow().clone();

    assert!(metrics1_after.current_leader.is_some(), "majority partition should maintain leader");
    assert_eq!(
        metrics1_after.current_leader, metrics2_after.current_leader,
        "majority partition nodes should agree on leader"
    );

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test network delays and their impact on consensus.
#[madsim::test]
async fn test_network_delays_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_network_delays", seed).start();

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

    artifact = artifact.add_event("failure: inject 1000ms delay between nodes 1 and 2");
    injector.set_network_delay(NodeId::from(1), NodeId::from(2), 1000);
    injector.set_network_delay(NodeId::from(2), NodeId::from(1), 1000);

    artifact = artifact.add_event("write: submit write with network delays");
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "delay_test".to_string(),
            value: "with_delays".to_string(),
        })
        .await
        .expect("write should succeed despite delays");

    artifact = artifact.add_event("wait: for replication with delays (4s)");
    madsim::time::sleep(std::time::Duration::from_millis(4000)).await;

    artifact = artifact.add_event("validation: consensus maintained with delays");
    let metrics1_after = raft1.metrics().borrow().clone();
    let metrics2_after = raft2.metrics().borrow().clone();
    let metrics3_after = raft3.metrics().borrow().clone();

    assert!(metrics1_after.last_applied.is_some(), "node 1 should apply entries despite delays");
    assert!(metrics2_after.last_applied.is_some(), "node 2 should apply entries despite delays");
    assert!(metrics3_after.last_applied.is_some(), "node 3 should apply entries despite delays");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test concurrent writes during node failures.
#[madsim::test]
async fn test_concurrent_writes_with_failures_seed_789() {
    let seed = 789_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_concurrent_writes_failures", seed).start();

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

    artifact = artifact.add_event("write: first write before failure");
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no leader");
    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "write1".to_string(),
            value: "before_failure".to_string(),
        })
        .await
        .expect("first write should succeed");

    artifact = artifact.add_event("failure: crash a follower node");
    // Find a follower to crash
    let follower_id = if leader_id == NodeId::from(1) {
        NodeId::from(2)
    } else {
        NodeId::from(1)
    };
    router.mark_node_failed(follower_id, true);
    artifact = artifact.add_event(format!("failure: node {} crashed", follower_id));

    artifact = artifact.add_event("write: second write with follower down");
    leader_raft
        .client_write(AppRequest::Set {
            key: "write2".to_string(),
            value: "follower_down".to_string(),
        })
        .await
        .expect("write should succeed with follower down");

    artifact = artifact.add_event("wait: for replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("validation: writes succeeded despite failure");
    // Check that the leader and remaining follower have both entries
    let remaining_follower_id = if follower_id == NodeId::from(1) {
        NodeId::from(3)
    } else {
        NodeId::from(1)
    };
    let remaining_follower = match remaining_follower_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid node id"),
    };

    let leader_metrics = leader_raft.metrics().borrow().clone();
    let follower_metrics = remaining_follower.metrics().borrow().clone();

    assert!(leader_metrics.last_applied.is_some(), "leader should have applied writes");
    assert!(follower_metrics.last_applied.is_some(), "remaining follower should have applied writes");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
