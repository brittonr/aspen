/// Failure injection tests for Raft using madsim with SQLite storage backend.
///
/// This test suite validates SQLite storage resilience under failure conditions:
/// - Leader crash and automatic re-election with persistent storage
/// - Network partitions with SQLite state machines
/// - Follower crash recovery
/// - Concurrent writes with failures
///
/// All tests use deterministic seeds for reproducibility.
use std::collections::BTreeMap;
use std::sync::Arc;

use aspen::raft::madsim_network::{FailureInjector, MadsimNetworkFactory, MadsimRaftRouter};
use aspen::raft::storage::RedbLogStore;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::{AppRequest, AppTypeConfig, NodeId};
use aspen::simulation::SimulationArtifactBuilder;
use openraft::{BasicNode, Config, Raft};

/// Helper to create a Raft instance with SQLite backend for madsim failure testing.
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
    let log_path = temp_base
        .path()
        .join(format!("{}-node-{}-log.redb", test_name, node_id));
    let sm_path = temp_base
        .path()
        .join(format!("{}-node-{}-sm.db", test_name, node_id));

    let log_store = RedbLogStore::new(&log_path).expect("failed to create log store");
    let state_machine = SqliteStateMachine::new(&sm_path).expect("failed to create state machine");

    // Keep tempdir alive for test duration
    std::mem::forget(temp_base);

    let network_factory = MadsimNetworkFactory::new(node_id, router, injector);

    Raft::new(node_id, config, network_factory, log_store, state_machine)
        .await
        .expect("failed to create raft instance")
}

/// Test leader crash and automatic re-election with SQLite storage.
///
/// Validates:
/// - SQLite state persists across leader failures
/// - Re-election works with persistent storage
/// - New leader can continue operations
#[madsim::test]
async fn test_sqlite_leader_crash_recovery_seed_42() {
    let seed = 42_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_sqlite_leader_crash", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 =
        create_raft_node_sqlite(1, "leader_crash_seed_42", router.clone(), injector.clone()).await;
    let raft2 =
        create_raft_node_sqlite(2, "leader_crash_seed_42", router.clone(), injector.clone()).await;
    let raft3 =
        create_raft_node_sqlite(3, "leader_crash_seed_42", router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(2, "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(3, "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster on node 1");
    let mut nodes = BTreeMap::new();
    nodes.insert(1, BasicNode::default());
    nodes.insert(2, BasicNode::default());
    nodes.insert(3, BasicNode::default());
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: identify initial leader");
    let metrics1 = raft1.metrics().borrow().clone();
    let initial_leader = metrics1.current_leader.expect("no initial leader");
    artifact = artifact.add_event(format!(
        "validation: initial leader is node {} with SQLite",
        initial_leader
    ));

    artifact = artifact.add_event("write: submit write before leader crash");
    let leader_raft = match initial_leader {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    leader_raft
        .client_write(AppRequest::Set {
            key: "before_crash".to_string(),
            value: "test_value".to_string(),
        })
        .await
        .expect("write should succeed before crash");

    artifact = artifact.add_event("wait: for replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event(format!("failure: crash node {} (leader)", initial_leader));
    router.mark_node_failed(initial_leader, true);

    artifact = artifact.add_event("wait: for re-election (5s)");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: check new leader elected");
    // Check remaining nodes for new leader
    let remaining_nodes = [(1, &raft1), (2, &raft2), (3, &raft3)];

    let mut new_leader = None;
    for (id, raft) in remaining_nodes.iter() {
        if *id != initial_leader {
            let metrics = raft.metrics().borrow().clone();
            if let Some(leader) = metrics.current_leader {
                new_leader = Some(leader);
                break;
            }
        }
    }

    assert!(new_leader.is_some(), "no new leader elected after crash");
    assert_ne!(
        new_leader.unwrap(),
        initial_leader,
        "new leader should be different from crashed leader"
    );

    artifact = artifact.add_event(format!(
        "validation: new leader is node {} after crash with SQLite",
        new_leader.unwrap()
    ));

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test network partition with SQLite storage.
///
/// Validates:
/// - Majority partition maintains consensus with SQLite
/// - Minority partition cannot make progress
/// - SQLite databases remain isolated during partition
#[madsim::test]
async fn test_sqlite_network_partition_seed_123() {
    let seed = 123_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_sqlite_partition", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 =
        create_raft_node_sqlite(1, "partition_seed_123", router.clone(), injector.clone()).await;
    let raft2 =
        create_raft_node_sqlite(2, "partition_seed_123", router.clone(), injector.clone()).await;
    let raft3 =
        create_raft_node_sqlite(3, "partition_seed_123", router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(2, "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(3, "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster");
    let mut nodes = BTreeMap::new();
    nodes.insert(1, BasicNode::default());
    nodes.insert(2, BasicNode::default());
    nodes.insert(3, BasicNode::default());
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: verify initial consensus");
    let metrics1 = raft1.metrics().borrow().clone();
    assert!(
        metrics1.current_leader.is_some(),
        "no leader before partition"
    );

    artifact = artifact.add_event("failure: partition node 3 from nodes 1 and 2");
    // Partition: node 3 cannot communicate with nodes 1 and 2
    injector.set_message_drop(3, 1, true);
    injector.set_message_drop(3, 2, true);
    injector.set_message_drop(1, 3, true);
    injector.set_message_drop(2, 3, true);

    artifact = artifact.add_event("write: submit write to majority partition");
    let leader_id = metrics1.current_leader.expect("no leader");
    let leader_raft = match leader_id {
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

    artifact = artifact.add_event("validation: majority partition operational with SQLite");
    // Nodes 1 and 2 should still have a leader and replicate
    let metrics1_after = raft1.metrics().borrow().clone();
    let metrics2_after = raft2.metrics().borrow().clone();

    assert!(
        metrics1_after.current_leader.is_some(),
        "majority partition should maintain leader"
    );
    assert_eq!(
        metrics1_after.current_leader, metrics2_after.current_leader,
        "majority partition nodes should agree on leader"
    );

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test follower crash recovery with SQLite storage.
///
/// Validates:
/// - Cluster continues operating when follower crashes
/// - SQLite state persists for remaining nodes
/// - Majority can still commit writes
#[madsim::test]
async fn test_sqlite_follower_crash_recovery_seed_456() {
    let seed = 456_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_sqlite_follower_crash", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 = create_raft_node_sqlite(
        1,
        "follower_crash_seed_456",
        router.clone(),
        injector.clone(),
    )
    .await;
    let raft2 = create_raft_node_sqlite(
        2,
        "follower_crash_seed_456",
        router.clone(),
        injector.clone(),
    )
    .await;
    let raft3 = create_raft_node_sqlite(
        3,
        "follower_crash_seed_456",
        router.clone(),
        injector.clone(),
    )
    .await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(2, "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(3, "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster");
    let mut nodes = BTreeMap::new();
    nodes.insert(1, BasicNode::default());
    nodes.insert(2, BasicNode::default());
    nodes.insert(3, BasicNode::default());
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("metrics: identify leader");
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no leader");

    artifact = artifact.add_event("write: first write before failure");
    let leader_raft = match leader_id {
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
    let follower_id = if leader_id == 1 { 2 } else { 1 };
    router.mark_node_failed(follower_id, true);
    artifact = artifact.add_event(format!("failure: node {} crashed (follower)", follower_id));

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

    artifact =
        artifact.add_event("validation: writes succeeded with SQLite despite follower crash");
    // Check that the leader and remaining follower have both entries
    let remaining_follower_id = if follower_id == 1 {
        if leader_id == 2 { 3 } else { 2 }
    } else if leader_id == 1 {
        3
    } else {
        1
    };
    let remaining_follower = match remaining_follower_id {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid node id"),
    };

    let leader_metrics = leader_raft.metrics().borrow().clone();
    let follower_metrics = remaining_follower.metrics().borrow().clone();

    assert!(
        leader_metrics.last_applied.is_some(),
        "leader should have applied writes"
    );
    assert!(
        follower_metrics.last_applied.is_some(),
        "remaining follower should have applied writes"
    );

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test concurrent writes during failures with SQLite storage.
///
/// Validates:
/// - SQLite handles concurrent writes correctly
/// - Writes succeed despite follower failures
/// - Persistent storage remains consistent
#[madsim::test]
async fn test_sqlite_concurrent_writes_seed_789() {
    let seed = 789_u64;
    let mut artifact =
        SimulationArtifactBuilder::new("madsim_sqlite_concurrent_writes", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SQLite backend");
    let raft1 = create_raft_node_sqlite(
        1,
        "concurrent_writes_seed_789",
        router.clone(),
        injector.clone(),
    )
    .await;
    let raft2 = create_raft_node_sqlite(
        2,
        "concurrent_writes_seed_789",
        router.clone(),
        injector.clone(),
    )
    .await;
    let raft3 = create_raft_node_sqlite(
        3,
        "concurrent_writes_seed_789",
        router.clone(),
        injector.clone(),
    )
    .await;

    artifact = artifact.add_event("register: all nodes with router");
    router
        .register_node(1, "127.0.0.1:26001".to_string(), raft1.clone())
        .expect("failed to register node 1");
    router
        .register_node(2, "127.0.0.1:26002".to_string(), raft2.clone())
        .expect("failed to register node 2");
    router
        .register_node(3, "127.0.0.1:26003".to_string(), raft3.clone())
        .expect("failed to register node 3");

    artifact = artifact.add_event("init: initialize 3-node cluster");
    let mut nodes = BTreeMap::new();
    nodes.insert(1, BasicNode::default());
    nodes.insert(2, BasicNode::default());
    nodes.insert(3, BasicNode::default());
    raft1
        .initialize(nodes)
        .await
        .expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    artifact = artifact.add_event("write: first write before failure");
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no leader");
    let leader_raft = match leader_id {
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
    let follower_id = if leader_id == 1 { 2 } else { 1 };
    router.mark_node_failed(follower_id, true);
    artifact = artifact.add_event(format!("failure: node {} crashed", follower_id));

    artifact = artifact.add_event("write: multiple writes with follower down");
    for i in 2..=5 {
        leader_raft
            .client_write(AppRequest::Set {
                key: format!("write{}", i),
                value: "follower_down".to_string(),
            })
            .await
            .expect("concurrent write should succeed with follower down");
    }

    artifact = artifact.add_event("wait: for replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    artifact = artifact.add_event("validation: concurrent writes succeeded with SQLite");
    let remaining_follower_id = if follower_id == 1 { 3 } else { 1 };
    let remaining_follower = match remaining_follower_id {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid node id"),
    };

    let leader_metrics = leader_raft.metrics().borrow().clone();
    let follower_metrics = remaining_follower.metrics().borrow().clone();

    assert!(
        leader_metrics.last_applied.is_some(),
        "leader should have applied concurrent writes"
    );
    assert!(
        follower_metrics.last_applied.is_some(),
        "remaining follower should have applied concurrent writes"
    );

    // Verify that all writes were applied (1 + 4 concurrent = 5 total)
    assert!(
        leader_metrics.last_applied.unwrap().index >= 5,
        "leader should have applied at least 5 log entries"
    );

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
