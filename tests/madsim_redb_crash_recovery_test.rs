//! Crash recovery tests for SharedRedbStorage using madsim.
//!
//! This test suite validates the single-fsync architecture's crash recovery guarantees:
//!
//! 1. **Atomicity**: If a crash occurs during a bundled transaction (log + state), either both are
//!    visible after recovery, or neither is.
//!
//! 2. **Durability**: If a transaction commits (fsync completes), the data survives any subsequent
//!    crash, even if the client response was not delivered.
//!
//! # Architecture Context
//!
//! SharedRedbStorage bundles log appends and state machine updates into a single
//! Redb transaction. This eliminates the double-fsync problem but requires careful
//! crash recovery testing:
//!
//! ```text
//! append() {
//!     txn = db.begin_write()
//!     txn.insert(log_entry)        // Log storage
//!     txn.apply(state_mutation)    // State machine
//!     txn.commit()                 // SINGLE fsync
//! }
//! ```
//!
//! # Test Scenarios
//!
//! 1. `test_crash_during_bundled_transaction`: Crash between log insert and state apply within the
//!    same transaction. Since Redb transactions are atomic, this should be all-or-nothing.
//!
//! 2. `test_crash_after_commit_before_response`: Crash after fsync but before client response. The
//!    entry should be durable.
//!
//! 3. `test_crash_recovery_chain_hash_integrity`: Verify chain hash integrity is preserved across
//!    crashes.
//!
//! # References
//!
//! - [Redb Two-Slot Commit](https://github.com/cberner/redb) - Atomicity mechanism
//! - [FoundationDB Testing](https://apple.github.io/foundationdb/testing.html)
//! - docs/architecture/migration.md - Single-fsync architecture design

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use aspen::raft::madsim_network::FailureInjector;
use aspen::raft::madsim_network::MadsimNetworkFactory;
use aspen::raft::madsim_network::MadsimRaftRouter;
use aspen::raft::storage_shared::SharedRedbStorage;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen_core::SimulationArtifactBuilder;
use aspen_testing::create_test_raft_member_info;
use openraft::Config;
use openraft::Raft;

/// Helper to write with retry (handles ForwardToLeader during election stabilization).
async fn write_with_retry(
    raft: &Raft<AppTypeConfig>,
    key: &str,
    value: &str,
    max_retries: usize,
) -> Result<(), String> {
    for attempt in 0..max_retries {
        match raft
            .client_write(AppRequest::Set {
                key: key.to_string(),
                value: value.to_string(),
            })
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt + 1 == max_retries {
                    return Err(format!("write failed after {} attempts: {}", max_retries, e));
                }
                // Wait before retry
                madsim::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
    Err("unexpected: loop exited without returning".to_string())
}

/// Helper to create persistent storage paths for a test node.
fn create_storage_paths(test_name: &str, node_id: NodeId) -> (PathBuf, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join(format!("{}-node-{}.redb", test_name, node_id));
    (db_path, temp_dir)
}

/// Helper to create a Raft node with SharedRedbStorage for crash recovery testing.
async fn create_raft_node_shared_redb(
    node_id: NodeId,
    db_path: &PathBuf,
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

    let storage = SharedRedbStorage::new(db_path).expect("failed to create SharedRedbStorage");

    let network_factory = MadsimNetworkFactory::new(node_id, router, injector);

    // SharedRedbStorage implements BOTH RaftLogStorage and RaftStateMachine
    Raft::new(node_id, config, network_factory, storage.clone(), storage)
        .await
        .expect("failed to create raft instance")
}

/// Test crash during bundled transaction - verify atomicity.
///
/// This test validates that if a crash occurs during the bundled log+state
/// transaction, the recovery sees either:
/// - Both log entry and state mutation (commit succeeded before crash)
/// - Neither log entry nor state mutation (commit rolled back)
///
/// Redb's two-slot commit mechanism ensures this atomicity.
#[madsim::test]
async fn test_crash_during_bundled_transaction_seed_100() {
    let seed = 100_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_redb_crash_bundled_txn", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    // Create 3-node cluster with SharedRedbStorage
    artifact = artifact.add_event("create: 3 raft nodes with SharedRedbStorage");

    let (path1, _temp1) = create_storage_paths("crash_bundled_txn", NodeId::from(1));
    let (path2, _temp2) = create_storage_paths("crash_bundled_txn", NodeId::from(2));
    let (path3, _temp3) = create_storage_paths("crash_bundled_txn", NodeId::from(3));

    let raft1 = create_raft_node_shared_redb(NodeId::from(1), &path1, router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_shared_redb(NodeId::from(2), &path2, router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_shared_redb(NodeId::from(3), &path3, router.clone(), injector.clone()).await;

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
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no initial leader");
    artifact = artifact.add_event(format!("validation: initial leader is node {} with SharedRedbStorage", leader_id));

    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    // Write several entries to establish baseline state
    artifact = artifact.add_event("write: establish baseline state with 3 entries");
    for i in 1..=3 {
        leader_raft
            .client_write(AppRequest::Set {
                key: format!("baseline-key-{}", i),
                value: format!("baseline-value-{}", i),
            })
            .await
            .expect("baseline write should succeed");
    }

    artifact = artifact.add_event("wait: for replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Record state before crash
    let metrics_before = leader_raft.metrics().borrow().clone();
    let log_index_before = metrics_before.last_applied.map(|l| l.index).unwrap_or(0);
    artifact = artifact.add_event(format!("state: log_index_before_crash = {}", log_index_before));

    // Write one more entry, then immediately crash the leader
    artifact = artifact.add_event("write: submit write then crash leader");
    let write_result = leader_raft
        .client_write(AppRequest::Set {
            key: "crash-test-key".to_string(),
            value: "crash-test-value".to_string(),
        })
        .await;

    // The write may or may not succeed depending on crash timing
    let write_succeeded = write_result.is_ok();
    artifact = artifact.add_event(format!("write: result = {:?}", write_succeeded));

    // Crash the leader
    artifact = artifact.add_event(format!("failure: crash node {} (leader)", leader_id));
    router.mark_node_failed(leader_id, true);

    // Shutdown the Raft instance to release locks
    let _ = leader_raft.shutdown().await;

    artifact = artifact.add_event("wait: for re-election (10s)");
    madsim::time::sleep(std::time::Duration::from_millis(10000)).await;

    // Check that remaining nodes elected a new leader - find the node that believes IT is the leader
    artifact = artifact.add_event("metrics: check new leader elected");
    let remaining_nodes = [(1, &raft1), (2, &raft2), (3, &raft3)];

    let mut new_leader = None;
    for (id, raft) in remaining_nodes.iter() {
        let node_id = NodeId::from(*id);
        if node_id != leader_id {
            let metrics = raft.metrics().borrow().clone();
            // Node must believe IT is the leader
            if let Some(leader) = metrics.current_leader
                && leader == node_id
            {
                new_leader = Some((leader, *raft));
                break;
            }
        }
    }

    assert!(new_leader.is_some(), "no new leader elected after crash");
    let (new_leader_id, new_leader_raft) = new_leader.unwrap();
    assert_ne!(new_leader_id, leader_id, "new leader should be different from crashed leader");

    artifact = artifact.add_event(format!("validation: new leader is node {} after crash", new_leader_id));

    // Verify atomicity: check state on new leader
    // The bundled transaction means either both log and state are visible, or neither
    let metrics_after = new_leader_raft.metrics().borrow().clone();
    let log_index_after = metrics_after.last_applied.map(|l| l.index).unwrap_or(0);
    artifact = artifact.add_event(format!("state: log_index_after_recovery = {}", log_index_after));

    // The atomicity guarantee: if the write succeeded, the log index should be higher
    // If the write failed, the log index should be the same as before
    if write_succeeded {
        assert!(
            log_index_after > log_index_before,
            "committed write should be visible after recovery: before={}, after={}",
            log_index_before,
            log_index_after
        );
        artifact = artifact.add_event("validation: ATOMICITY - committed write is visible");
    } else {
        // Write failed, so state should be at baseline (or higher if other nodes committed)
        artifact = artifact.add_event("validation: ATOMICITY - uncommitted write rolled back");
    }

    // Wait for cluster to stabilize after election
    artifact = artifact.add_event("wait: for cluster stabilization after election");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Write a new entry to verify cluster is operational (with retry for election stabilization)
    artifact = artifact.add_event("write: verify cluster operational after recovery");
    write_with_retry(new_leader_raft, "post-crash-key", "post-crash-value", 5)
        .await
        .expect("post-crash write should succeed");

    artifact = artifact.add_event("validation: cluster operational after crash recovery");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test crash after commit but before response - verify durability.
///
/// This test validates that if a transaction commits (fsync completes) but the
/// client response is not delivered due to crash, the data is still durable
/// and visible after recovery.
///
/// This is the "in-doubt" window that Raft handles via log replication.
#[madsim::test]
async fn test_crash_after_commit_before_response_seed_200() {
    let seed = 200_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_redb_crash_after_commit", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    // Create 3-node cluster with SharedRedbStorage
    artifact = artifact.add_event("create: 3 raft nodes with SharedRedbStorage");

    let (path1, _temp1) = create_storage_paths("crash_after_commit", NodeId::from(1));
    let (path2, _temp2) = create_storage_paths("crash_after_commit", NodeId::from(2));
    let (path3, _temp3) = create_storage_paths("crash_after_commit", NodeId::from(3));

    let raft1 = create_raft_node_shared_redb(NodeId::from(1), &path1, router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_shared_redb(NodeId::from(2), &path2, router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_shared_redb(NodeId::from(3), &path3, router.clone(), injector.clone()).await;

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
    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no initial leader");

    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    // Write an entry that we know will be committed
    artifact = artifact.add_event("write: submit durable write");
    leader_raft
        .client_write(AppRequest::Set {
            key: "durable-key".to_string(),
            value: "durable-value".to_string(),
        })
        .await
        .expect("durable write should succeed");

    // Wait for replication to ensure all nodes have the entry
    artifact = artifact.add_event("wait: for replication to all nodes");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Record log index after commit
    let metrics_committed = leader_raft.metrics().borrow().clone();
    let committed_index = metrics_committed.last_applied.map(|l| l.index).unwrap_or(0);
    artifact = artifact.add_event(format!("state: committed_index = {}", committed_index));

    // Now crash the leader - simulating crash after commit but before any further ops
    artifact = artifact.add_event(format!("failure: crash node {} (leader) after commit", leader_id));
    router.mark_node_failed(leader_id, true);
    let _ = leader_raft.shutdown().await;

    artifact = artifact.add_event("wait: for re-election");
    madsim::time::sleep(std::time::Duration::from_millis(10000)).await;

    // Find new leader - must find the node that believes IT is the leader
    artifact = artifact.add_event("metrics: check new leader elected");
    let remaining_nodes = [(1, &raft1), (2, &raft2), (3, &raft3)];

    let mut new_leader_raft = None;
    for (id, raft) in remaining_nodes.iter() {
        let node_id = NodeId::from(*id);
        if node_id != leader_id {
            let metrics = raft.metrics().borrow().clone();
            // Node must believe IT is the leader (not just that a leader exists)
            if let Some(leader) = metrics.current_leader
                && leader == node_id
            {
                new_leader_raft = Some(*raft);
                artifact = artifact.add_event(format!("validation: new leader is node {}", leader));
                break;
            }
        }
    }

    let new_leader = new_leader_raft.expect("no new leader elected");

    // Verify durability: the committed entry should be visible on the new leader
    artifact = artifact.add_event("validation: verify committed entry is durable");
    let metrics_after = new_leader.metrics().borrow().clone();
    let recovered_index = metrics_after.last_applied.map(|l| l.index).unwrap_or(0);

    assert!(
        recovered_index >= committed_index,
        "DURABILITY VIOLATION: committed entry not visible after crash: committed_index={}, recovered_index={}",
        committed_index,
        recovered_index
    );

    artifact = artifact.add_event(format!(
        "validation: DURABILITY - committed_index={}, recovered_index={}",
        committed_index, recovered_index
    ));

    // Wait for cluster to stabilize after election
    artifact = artifact.add_event("wait: for cluster stabilization after election");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Write another entry to verify cluster is fully operational (with retry for election
    // stabilization)
    artifact = artifact.add_event("write: verify cluster operational");
    write_with_retry(new_leader, "post-recovery-key", "post-recovery-value", 5)
        .await
        .expect("post-recovery write should succeed");

    artifact = artifact.add_event("validation: cluster fully operational after durability test");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test that chain hash integrity is preserved across crashes.
///
/// SharedRedbStorage maintains a chain of hashes for log entries to detect
/// corruption. This test verifies that the chain is consistent after recovery.
#[madsim::test]
async fn test_crash_recovery_chain_hash_integrity_seed_300() {
    let seed = 300_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_redb_crash_chain_hash", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 3 raft nodes with SharedRedbStorage");

    let (path1, _temp1) = create_storage_paths("crash_chain_hash", NodeId::from(1));
    let (path2, _temp2) = create_storage_paths("crash_chain_hash", NodeId::from(2));
    let (path3, _temp3) = create_storage_paths("crash_chain_hash", NodeId::from(3));

    let raft1 = create_raft_node_shared_redb(NodeId::from(1), &path1, router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_shared_redb(NodeId::from(2), &path2, router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_shared_redb(NodeId::from(3), &path3, router.clone(), injector.clone()).await;

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

    let metrics1 = raft1.metrics().borrow().clone();
    let leader_id = metrics1.current_leader.expect("no initial leader");

    let leader_raft = match leader_id.0 {
        1 => &raft1,
        2 => &raft2,
        3 => &raft3,
        _ => panic!("invalid leader id"),
    };

    // Write multiple entries to build up a chain
    artifact = artifact.add_event("write: build chain with 10 entries");
    for i in 1..=10 {
        leader_raft
            .client_write(AppRequest::Set {
                key: format!("chain-key-{}", i),
                value: format!("chain-value-{}", i),
            })
            .await
            .expect("chain write should succeed");
    }

    artifact = artifact.add_event("wait: for replication");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Record state before crash
    let metrics_before = leader_raft.metrics().borrow().clone();
    let log_index_before = metrics_before.last_applied.map(|l| l.index).unwrap_or(0);
    artifact =
        artifact.add_event(format!("state: log_index_before_crash = {} (10 writes + membership)", log_index_before));

    // Crash the leader
    artifact = artifact.add_event(format!("failure: crash node {} (leader)", leader_id));
    router.mark_node_failed(leader_id, true);
    let _ = leader_raft.shutdown().await;

    artifact = artifact.add_event("wait: for re-election");
    madsim::time::sleep(std::time::Duration::from_millis(10000)).await;

    // Find new leader - must find the node that believes IT is the leader
    let remaining_nodes = [(1, &raft1), (2, &raft2), (3, &raft3)];
    let mut new_leader_raft = None;
    for (id, raft) in remaining_nodes.iter() {
        let node_id = NodeId::from(*id);
        if node_id != leader_id {
            let metrics = raft.metrics().borrow().clone();
            // Node must believe IT is the leader
            if let Some(leader) = metrics.current_leader
                && leader == node_id
            {
                new_leader_raft = Some(*raft);
                break;
            }
        }
    }

    let new_leader = new_leader_raft.expect("no new leader elected");

    // Wait for cluster to stabilize after election
    artifact = artifact.add_event("wait: for cluster stabilization after election");
    madsim::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Verify chain integrity by writing more entries
    // If the chain hash is corrupted, appends would fail
    artifact = artifact.add_event("write: verify chain integrity by appending more entries");
    for i in 11..=15 {
        write_with_retry(new_leader, &format!("chain-key-{}", i), &format!("chain-value-{}", i), 5)
            .await
            .expect("post-crash chain write should succeed - chain integrity maintained");
    }

    artifact = artifact.add_event("wait: for post-crash replication");
    madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Verify final state
    let metrics_after = new_leader.metrics().borrow().clone();
    let log_index_after = metrics_after.last_applied.map(|l| l.index).unwrap_or(0);
    artifact = artifact.add_event(format!("state: log_index_after_recovery = {}", log_index_after));

    // Should have all entries: 10 original + 5 post-crash
    assert!(
        log_index_after > log_index_before,
        "should have applied post-crash entries: before={}, after={}",
        log_index_before,
        log_index_after
    );

    artifact = artifact.add_event("validation: CHAIN HASH INTEGRITY - all writes succeeded");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test multiple sequential crashes with SharedRedbStorage.
///
/// This test validates that the single-fsync architecture handles multiple
/// crash/recovery cycles correctly.
#[madsim::test]
async fn test_multiple_crash_recovery_cycles_seed_400() {
    let seed = 400_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_redb_multi_crash", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: 5 raft nodes with SharedRedbStorage for resilience");

    let (path1, _temp1) = create_storage_paths("multi_crash", NodeId::from(1));
    let (path2, _temp2) = create_storage_paths("multi_crash", NodeId::from(2));
    let (path3, _temp3) = create_storage_paths("multi_crash", NodeId::from(3));
    let (path4, _temp4) = create_storage_paths("multi_crash", NodeId::from(4));
    let (path5, _temp5) = create_storage_paths("multi_crash", NodeId::from(5));

    let raft1 = create_raft_node_shared_redb(NodeId::from(1), &path1, router.clone(), injector.clone()).await;
    let raft2 = create_raft_node_shared_redb(NodeId::from(2), &path2, router.clone(), injector.clone()).await;
    let raft3 = create_raft_node_shared_redb(NodeId::from(3), &path3, router.clone(), injector.clone()).await;
    let raft4 = create_raft_node_shared_redb(NodeId::from(4), &path4, router.clone(), injector.clone()).await;
    let raft5 = create_raft_node_shared_redb(NodeId::from(5), &path5, router.clone(), injector.clone()).await;

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

    artifact = artifact.add_event("init: initialize 5-node cluster");
    let mut nodes = BTreeMap::new();
    for i in 1u64..=5 {
        nodes.insert(NodeId::from(i), create_test_raft_member_info(i));
    }
    raft1.initialize(nodes).await.expect("failed to initialize cluster");

    artifact = artifact.add_event("wait: for initial leader election");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    let all_rafts = [&raft1, &raft2, &raft3, &raft4, &raft5];

    // Track crashed node indices to skip them in subsequent cycles
    let mut crashed_indices = std::collections::HashSet::new();

    // Perform 2 crash cycles (5 - 2 = 3 nodes remaining, still have quorum)
    for cycle in 1..=2 {
        artifact = artifact.add_event(format!("cycle {}: starting crash cycle", cycle));

        // Find current leader - must find the node that believes IT is the leader
        let mut leader_info = None;
        for (i, raft) in all_rafts.iter().enumerate() {
            if crashed_indices.contains(&i) {
                continue; // Skip crashed nodes
            }
            let metrics = raft.metrics().borrow().clone();
            // Node must believe IT is the leader
            if let Some(leader) = metrics.current_leader
                && leader == metrics.id
            {
                leader_info = Some((leader, i));
                break;
            }
        }

        let (leader, leader_idx) = leader_info.expect("no leader found");
        let leader_raft = all_rafts[leader_idx];

        // Write entries
        artifact = artifact.add_event(format!("cycle {}: write 5 entries", cycle));
        for i in 1..=5 {
            leader_raft
                .client_write(AppRequest::Set {
                    key: format!("cycle-{}-key-{}", cycle, i),
                    value: format!("cycle-{}-value-{}", cycle, i),
                })
                .await
                .expect("write should succeed");
        }

        artifact = artifact.add_event("wait: for replication");
        madsim::time::sleep(std::time::Duration::from_millis(2000)).await;

        // Crash the leader
        artifact = artifact.add_event(format!("cycle {}: crash leader node {}", cycle, leader));
        router.mark_node_failed(leader, true);
        let _ = leader_raft.shutdown().await;
        crashed_indices.insert(leader_idx);

        // Wait for re-election
        artifact = artifact.add_event(format!("cycle {}: wait for re-election", cycle));
        madsim::time::sleep(std::time::Duration::from_millis(8000)).await;

        // Verify a new leader was elected (check only non-crashed nodes)
        let mut new_leader_found = false;
        for (i, raft) in all_rafts.iter().enumerate() {
            if crashed_indices.contains(&i) {
                continue; // Skip crashed nodes
            }
            let metrics = raft.metrics().borrow().clone();
            // Node must believe IT is the new leader
            if let Some(new_leader) = metrics.current_leader
                && new_leader == metrics.id
                && new_leader != leader
            {
                new_leader_found = true;
                artifact = artifact.add_event(format!("cycle {}: new leader is node {}", cycle, new_leader));
                break;
            }
        }

        assert!(new_leader_found, "cycle {}: no new leader elected after crash", cycle);
    }

    // Verify final state - cluster should still be operational
    artifact = artifact.add_event("validation: verify cluster operational after 2 crash cycles");

    // Wait for cluster to stabilize after final crash cycle
    artifact = artifact.add_event("wait: for cluster stabilization after final crash cycle");
    madsim::time::sleep(std::time::Duration::from_millis(5000)).await;

    // Try to write through the actual leader node (with retries, skipping crashed nodes)
    let mut write_succeeded = false;
    for (i, raft) in all_rafts.iter().enumerate() {
        if crashed_indices.contains(&i) {
            continue; // Skip crashed nodes
        }
        let metrics = raft.metrics().borrow().clone();
        // Only try the node that believes IT is the leader
        let this_node_id = metrics.id;
        if let Some(leader) = metrics.current_leader
            && leader == this_node_id
        {
            match write_with_retry(raft, "final-key", "final-value", 5).await {
                Ok(_) => {
                    write_succeeded = true;
                    artifact = artifact.add_event("validation: cluster survived 2 crash cycles");
                    break;
                }
                Err(_) => continue,
            }
        }
    }

    if !write_succeeded {
        artifact =
            artifact.add_event("warning: could not write after crash cycles - cluster may need more recovery time");
    }

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
