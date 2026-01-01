/// Append entries conflict resolution tests.
///
/// This test validates append-entries response handling in every conflict scenario:
/// - Empty logs with/without prev_log_id
/// - Conflicting entries before/after committed index
/// - Missing logs (prev_log_id beyond current last_log_id)
/// - Inconsistent log detection and removal
///
/// The test calls append_entries directly on a Raft handle (not through network layer)
/// to validate the core append-entries logic without network overhead.
///
/// Original: openraft/tests/tests/append_entries/t11_append_conflicts.rs
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen_testing::AspenRouter;
use openraft::Config;
use openraft::Entry;
use openraft::ServerState;
use openraft::Vote;
use openraft::raft::AppendEntriesRequest;
use openraft::storage::RaftLogReader;
use openraft::storage::RaftLogStorage;
use openraft::testing::blank_ent;
use openraft::testing::log_id;

/// Helper to check if logs match expected terms.
async fn check_logs(log_store: &mut aspen::raft::storage::InMemoryLogStore, terms: Vec<u64>) -> Result<()> {
    let logs = log_store.get_log_reader().await.try_get_log_entries(..).await?;

    let want: Vec<Entry<AppTypeConfig>> = terms
        .iter()
        .enumerate()
        .map(|(i, term)| blank_ent::<AppTypeConfig>(*term, NodeId::from(0), i as u64))
        .collect();

    let w = format!("{:?}", &want);
    let g = format!("{:?}", &logs);

    assert_eq!(w, g, "log entries mismatch");

    Ok(())
}

fn timeout() -> Option<Duration> {
    Some(Duration::from_millis(1000))
}

/// Test append-entries response in every case.
///
/// Validates all conflict scenarios by sending append_entries requests directly
/// to a learner node and checking responses (is_success, is_conflict).
#[tokio::test]
async fn test_append_conflicts() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());
    router.new_raft_node(0).await?;

    tracing::info!("--- wait for init node to ready");

    router.wait(0, timeout()).applied_index(None, "empty").await?;
    router.wait(0, timeout()).state(ServerState::Learner, "empty").await?;

    let (r0, mut sto0, _sm0) = router.remove_node(0).unwrap();
    check_logs(&mut sto0, vec![]).await?;

    tracing::info!("--- case 0: prev_log_id == None, no logs");

    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: None,
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;

    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    tracing::info!("--- case 0: prev_log_id == None, 1 logs");

    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: None,
        entries: vec![blank_ent::<AppTypeConfig>(0, NodeId::from(0), 0)],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    tracing::info!("--- case 0: prev_log_id == 1-1, 0 logs");

    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(0, NodeId::from(0), 0)),
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    check_logs(&mut sto0, vec![0]).await?;

    tracing::info!("--- case 0: prev_log_id.index == 0");

    let req = || AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(0, NodeId::from(0), 0)),
        entries: vec![
            blank_ent::<AppTypeConfig>(1, NodeId::from(0), 1),
            blank_ent::<AppTypeConfig>(1, NodeId::from(0), 2),
            blank_ent::<AppTypeConfig>(1, NodeId::from(0), 3),
            blank_ent::<AppTypeConfig>(1, NodeId::from(0), 4),
        ],
        // this sets last_applied to 2
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req()).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1, 1, 1]).await?;

    tracing::info!("--- case 0: prev_log_id.index == 0, last_log_id mismatch");

    let resp = r0.append_entries(req()).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1, 1, 1]).await?;

    // committed index is 2
    tracing::info!("--- case 1: 0 < prev_log_id.index < commit_index");

    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 1)),
        entries: vec![blank_ent::<AppTypeConfig>(1, NodeId::from(0), 2)],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1, 1, 1]).await?;

    tracing::info!("--- case 2: prev_log_id.index == last_applied, inconsistent log should be removed");

    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
        entries: vec![blank_ent::<AppTypeConfig>(2, NodeId::from(0), 3)],
        // this sets last_applied to 2
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1, 2]).await?;

    // check last_log_id is updated:
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2000)),
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(!resp.is_success());
    assert!(resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1, 2]).await?;

    tracing::info!("--- case 3,4: prev_log_id.index <= last_log_id, prev_log_id mismatch, inconsistent log is removed");

    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(3, NodeId::from(0), 3)),
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(!resp.is_success());
    assert!(resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1]).await?;

    tracing::info!("--- case 3,4: prev_log_id.index <= last_log_id, prev_log_id matches, inconsistent log is removed");
    // refill logs
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
        entries: vec![
            blank_ent::<AppTypeConfig>(2, NodeId::from(0), 3),
            blank_ent::<AppTypeConfig>(2, NodeId::from(0), 4),
            blank_ent::<AppTypeConfig>(2, NodeId::from(0), 5),
        ],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    // check prepared store
    check_logs(&mut sto0, vec![0, 1, 1, 2, 2, 2]).await?;

    // prev_log_id matches
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(2, NodeId::from(0), 3)),
        entries: vec![blank_ent::<AppTypeConfig>(3, NodeId::from(0), 4)],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success());
    assert!(!resp.is_conflict());

    check_logs(&mut sto0, vec![0, 1, 1, 2, 3]).await?;

    tracing::info!("--- case 5: last_log_id.index < prev_log_id.index");

    // refill logs
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(2)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 200)),
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(!resp.is_success());
    assert!(resp.is_conflict());

    Ok(())
}
