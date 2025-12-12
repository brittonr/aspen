/// Append entries tests for Raft using madsim.
///
/// This test suite validates append_entries RPC behavior:
/// - Handling of conflicting log entries
/// - Response to mismatched prev_log_id
/// - Log truncation and replication
/// - Committed index updates
///
/// Ported from openraft/tests/tests/append_entries/t11_append_conflicts.rs
use std::sync::Arc;

use aspen::raft::madsim_network::{FailureInjector, MadsimNetworkFactory, MadsimRaftRouter};
use aspen::raft::storage::{InMemoryLogStore, InMemoryStateMachine};
use aspen::raft::types::{AppTypeConfig, NodeId};
use aspen::simulation::SimulationArtifactBuilder;
use openraft::raft::AppendEntriesRequest;
use openraft::testing::{blank_ent, log_id};
use openraft::{Config, LogId, Raft, Vote};

/// Helper to create a Raft instance for madsim testing.
async fn create_raft_node(
    node_id: NodeId,
    router: Arc<MadsimRaftRouter>,
    injector: Arc<FailureInjector>,
) -> Raft<AppTypeConfig> {
    let config = Config {
        enable_heartbeat: false,
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

/// Helper to create LogId for AppTypeConfig
fn make_log_id(term: u64, node_id: u64, index: u64) -> LogId<AppTypeConfig> {
    log_id::<AppTypeConfig>(term, NodeId::from(node_id), index)
}

/// Test append-entries response in every case.
///
/// Brings up a learner node and sends append_entries requests to verify
/// correct handling of:
/// - Empty log with None prev_log_id
/// - Matching prev_log_id
/// - Mismatched prev_log_id (conflicts)
/// - Log truncation when inconsistent entries exist
/// - Committed index updates
#[madsim::test]
async fn test_append_conflicts_seed_1001() {
    let seed = 1001_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_append_conflicts", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: single learner node");
    let raft0 = create_raft_node(NodeId::from(0), router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: node with router");
    router
        .register_node(
            NodeId::from(0),
            "127.0.0.1:26000".to_string(),
            raft0.clone(),
        )
        .expect("failed to register node 0");

    artifact = artifact.add_event("wait: for node to initialize");
    madsim::time::sleep(std::time::Duration::from_millis(500)).await;
    // Node will be in following state since it's not initialized yet
    let metrics = raft0.metrics().borrow().clone();
    artifact = artifact.add_event(format!(
        "validation: node initialized, state={:?}",
        metrics.running_state
    ));

    // Case 0: prev_log_id == None, no logs
    artifact = artifact.add_event("case_0: prev_log_id=None, entries=[]");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: None,
        entries: vec![],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(resp.is_success(), "should succeed with empty log");
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 1: prev_log_id == None, 1 log entry
    artifact = artifact.add_event("case_1: prev_log_id=None, entries=[blank(0,0,0)]");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: None,
        entries: vec![blank_ent(0, NodeId::from(0), 0)],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(resp.is_success(), "should succeed appending first entry");
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 2: prev_log_id matches, 0 entries
    artifact = artifact.add_event("case_2: prev_log_id=make_log_id(0,0,0), entries=[]");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(0, 0, 0)),
        entries: vec![],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(
        resp.is_success(),
        "should succeed with matching prev_log_id"
    );
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 3: prev_log_id.index == 0, append multiple entries
    artifact = artifact.add_event("case_3: prev_log_id=make_log_id(0,0,0), entries=[1,2,3,4]");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(0, 0, 0)),
        entries: vec![
            blank_ent(1, NodeId::from(0), 1),
            blank_ent(1, NodeId::from(0), 2),
            blank_ent(1, NodeId::from(0), 3),
            blank_ent(1, NodeId::from(0), 4),
        ],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(
        resp.is_success(),
        "should succeed appending multiple entries"
    );
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 4: Resend same entries (idempotent)
    artifact = artifact.add_event("case_4: resend same entries (idempotent check)");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(0, 0, 0)),
        entries: vec![
            blank_ent(1, NodeId::from(0), 1),
            blank_ent(1, NodeId::from(0), 2),
            blank_ent(1, NodeId::from(0), 3),
            blank_ent(1, NodeId::from(0), 4),
        ],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(resp.is_success(), "should succeed (idempotent)");
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 5: prev_log_id.index < commit_index (should succeed without changes)
    artifact = artifact.add_event("case_5: prev_log_id.index=1 < commit_index=2");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(1, 0, 1)),
        entries: vec![blank_ent(1, NodeId::from(0), 2)],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(
        resp.is_success(),
        "should succeed when prev_log_id < commit"
    );
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 6: prev_log_id == last_applied, truncate inconsistent entries
    artifact = artifact.add_event("case_6: truncate inconsistent entries at index 3");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(1, 0, 2)),
        entries: vec![blank_ent(2, NodeId::from(0), 3)], // Term 2 differs from existing term 1
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(
        resp.is_success(),
        "should succeed truncating inconsistent log"
    );
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 7: prev_log_id mismatch (index too high) - should conflict
    artifact = artifact.add_event("case_7: prev_log_id.index=2000 > last_log (conflict)");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(1, 0, 2000)),
        entries: vec![],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(!resp.is_success(), "should fail with prev_log_id gap");
    assert!(resp.is_conflict(), "should indicate conflict");

    // Case 8: prev_log_id term mismatch - should conflict and truncate
    artifact = artifact.add_event("case_8: prev_log_id term mismatch at index 3");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(3, 0, 3)), // Expecting term 3, but we have term 2
        entries: vec![],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(!resp.is_success(), "should fail with term mismatch");
    assert!(resp.is_conflict(), "should indicate conflict");

    // Case 9: Refill logs and test matching prev_log_id
    artifact = artifact.add_event("case_9: refill logs with consistent entries");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(1, 0, 2)),
        entries: vec![
            blank_ent(2, NodeId::from(0), 3),
            blank_ent(2, NodeId::from(0), 4),
            blank_ent(2, NodeId::from(0), 5),
        ],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(resp.is_success(), "should succeed refilling logs");
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 10: Matching prev_log_id, truncate later entries
    artifact = artifact.add_event("case_10: prev_log_id matches, truncate from index 4");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(2, 0, 3)),
        entries: vec![blank_ent(3, NodeId::from(0), 4)], // Different term at index 4
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(
        resp.is_success(),
        "should succeed with matching prev_log_id"
    );
    assert!(!resp.is_conflict(), "should not conflict");

    // Case 11: prev_log_id.index > last_log_id.index - should conflict
    artifact = artifact.add_event("case_11: prev_log_id.index=200 beyond last_log");
    let req = AppendEntriesRequest::<AppTypeConfig> {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(make_log_id(1, 0, 200)),
        entries: vec![],
        leader_commit: Some(make_log_id(1, 0, 2)),
    };
    let resp = raft0.append_entries(req).await.expect("append should work");
    assert!(!resp.is_success(), "should fail with large prev_log_id gap");
    assert!(resp.is_conflict(), "should indicate conflict");

    artifact = artifact.add_event("validation: all append_entries cases handled correctly");

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test stream_append with successful append entries.
///
/// Sends multiple append_entries requests via stream_append API and verifies
/// all entries are accepted without conflicts.
///
/// Ported from openraft/tests/tests/append_entries/t10_stream_append.rs
#[madsim::test]
async fn test_stream_append_success_seed_1002() {
    use futures::StreamExt;
    use futures::stream;
    use std::pin::pin;

    let seed = 1002_u64;
    let mut artifact = SimulationArtifactBuilder::new("madsim_stream_append_success", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: single learner node");
    let raft0 = create_raft_node(NodeId::from(0), router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: node with router");
    router
        .register_node(
            NodeId::from(0),
            "127.0.0.1:26000".to_string(),
            raft0.clone(),
        )
        .expect("failed to register node 0");

    artifact = artifact.add_event("wait: for node to initialize");
    madsim::time::sleep(std::time::Duration::from_millis(500)).await;

    artifact = artifact.add_event("stream: create append_entries request stream");
    let requests = vec![
        AppendEntriesRequest::<AppTypeConfig> {
            vote: Vote::new_committed(1, NodeId::from(1)),
            prev_log_id: None,
            entries: vec![
                blank_ent(0, NodeId::from(0), 0),
                blank_ent(1, NodeId::from(1), 1),
            ],
            leader_commit: None,
        },
        AppendEntriesRequest::<AppTypeConfig> {
            vote: Vote::new_committed(1, NodeId::from(1)),
            prev_log_id: Some(make_log_id(1, 1, 1)),
            entries: vec![
                blank_ent(1, NodeId::from(1), 2),
                blank_ent(1, NodeId::from(1), 3),
            ],
            leader_commit: None,
        },
        AppendEntriesRequest::<AppTypeConfig> {
            vote: Vote::new_committed(1, NodeId::from(1)),
            prev_log_id: Some(make_log_id(1, 1, 3)),
            entries: vec![blank_ent(1, NodeId::from(1), 4)],
            leader_commit: Some(make_log_id(1, 1, 4)),
        },
    ];

    let input_stream = stream::iter(requests);
    let output_stream = pin!(raft0.stream_append(input_stream));

    artifact = artifact.add_event("stream: collect results from stream_append");
    let results: Vec<_> = output_stream.collect().await;

    artifact = artifact.add_event("validation: all requests succeeded");
    assert_eq!(results.len(), 3, "should have 3 results");
    assert_eq!(
        results,
        vec![
            Ok(Some(make_log_id(1, 1, 1))),
            Ok(Some(make_log_id(1, 1, 3))),
            Ok(Some(make_log_id(1, 1, 4))),
        ]
    );

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}

/// Test stream_append terminates on conflict.
///
/// Sends append_entries where the second request has a conflicting prev_log_id.
/// Verifies that the stream terminates after the conflict.
#[madsim::test]
async fn test_stream_append_conflict_seed_1003() {
    use futures::StreamExt;
    use futures::stream;
    use openraft::raft::StreamAppendError;
    use std::pin::pin;

    let seed = 1003_u64;
    let mut artifact =
        SimulationArtifactBuilder::new("madsim_stream_append_conflict", seed).start();

    artifact = artifact.add_event("create: router and failure injector");
    let router = Arc::new(MadsimRaftRouter::new());
    let injector = Arc::new(FailureInjector::new());

    artifact = artifact.add_event("create: single learner node");
    let raft0 = create_raft_node(NodeId::from(0), router.clone(), injector.clone()).await;

    artifact = artifact.add_event("register: node with router");
    router
        .register_node(
            NodeId::from(0),
            "127.0.0.1:26000".to_string(),
            raft0.clone(),
        )
        .expect("failed to register node 0");

    artifact = artifact.add_event("wait: for node to initialize");
    madsim::time::sleep(std::time::Duration::from_millis(500)).await;

    artifact = artifact.add_event("stream: create requests with conflicting prev_log_id");
    let requests = vec![
        AppendEntriesRequest::<AppTypeConfig> {
            vote: Vote::new_committed(1, NodeId::from(1)),
            prev_log_id: None,
            entries: vec![
                blank_ent(0, NodeId::from(0), 0),
                blank_ent(1, NodeId::from(1), 1),
            ],
            leader_commit: None,
        },
        // This will conflict: prev_log_id at index 5 doesn't exist
        AppendEntriesRequest::<AppTypeConfig> {
            vote: Vote::new_committed(1, NodeId::from(1)),
            prev_log_id: Some(make_log_id(1, 1, 5)),
            entries: vec![blank_ent(1, NodeId::from(1), 6)],
            leader_commit: None,
        },
        // This should never be processed because stream terminates on conflict
        AppendEntriesRequest::<AppTypeConfig> {
            vote: Vote::new_committed(1, NodeId::from(1)),
            prev_log_id: Some(make_log_id(1, 1, 6)),
            entries: vec![blank_ent(1, NodeId::from(1), 7)],
            leader_commit: None,
        },
    ];

    let input_stream = stream::iter(requests);
    let output_stream = pin!(raft0.stream_append(input_stream));

    artifact = artifact.add_event("stream: collect results (should terminate on conflict)");
    let results: Vec<_> = output_stream.collect().await;

    artifact = artifact.add_event("validation: stream terminated after conflict");
    assert_eq!(
        results.len(),
        2,
        "should have 2 results (1 success, 1 conflict)"
    );
    assert_eq!(
        results,
        vec![
            Ok(Some(make_log_id(1, 1, 1))),
            Err(StreamAppendError::Conflict(Some(make_log_id(1, 1, 5)))),
        ]
    );

    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Simulation artifact persisted to: {}", path.display());
    }
}
