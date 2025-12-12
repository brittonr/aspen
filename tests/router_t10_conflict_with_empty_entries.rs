/// Test conflict detection with empty append-entries
///
/// This test validates that append-entries correctly reports conflicts
/// even when the entries list is empty. The conflict detection should
/// be based on prev_log_id matching, not the presence of entries.
///
/// Original: openraft/tests/tests/append_entries/t10_conflict_with_empty_entries.rs
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::{AppTypeConfig, NodeId};
use aspen::testing::AspenRouter;
use openraft::raft::AppendEntriesRequest;
use openraft::storage::{RaftLogReader, RaftLogStorage};
use openraft::testing::{blank_ent, log_id};
use openraft::{Config, ServerState, Vote};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test that append-entries reports conflicts even with empty entries
///
/// Focused on core conflict detection behavior with empty entry lists.
/// The key insight: prev_log_id matching determines success, not entry presence.
#[tokio::test]
async fn test_conflict_with_empty_entries() -> Result<()> {
    // Configure without automatic features for precise control
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            enable_elect: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- setup single learner node");

    // Create a single node (not initialized, so it remains a learner)
    router.new_raft_node(0).await?;

    // Verify it's in learner state
    router
        .wait(0, timeout())
        .state(ServerState::Learner, "node-0 is learner")
        .await?;

    tracing::info!("--- test conflict with non-existent prev_log_id (empty entries)");

    // Extract node for direct RPC testing
    let (r0, mut sto0, _sm0) = router.remove_node(0).unwrap();

    // Test 1: Empty entries + non-existent prev_log_id → conflict
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(1)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 5)),
        entries: vec![],
        leader_commit: None,
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        !resp.is_success(),
        "empty entries with non-existent prev_log_id should fail"
    );
    assert!(
        resp.is_conflict(),
        "should report conflict even with empty entries"
    );

    tracing::info!("--- add initial logs");

    // Add 3 logs
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(1)),
        prev_log_id: None,
        entries: vec![
            blank_ent::<AppTypeConfig>(0, NodeId::from(0), 0),
            blank_ent::<AppTypeConfig>(1, NodeId::from(0), 1),
            blank_ent::<AppTypeConfig>(1, NodeId::from(0), 2),
        ],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success(), "should successfully feed initial logs");

    let logs = sto0
        .get_log_reader()
        .await
        .try_get_log_entries(0..=2)
        .await?;
    assert_eq!(logs.len(), 3, "should have 3 log entries");

    tracing::info!("--- test conflict with out-of-bounds prev_log_id (empty entries)");

    // Test 2: Empty entries + out-of-bounds prev_log_id → conflict
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(1)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 3)),
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        !resp.is_success(),
        "empty entries with out-of-bounds prev_log_id should fail"
    );
    assert!(
        resp.is_conflict(),
        "should report conflict for out-of-bounds index"
    );

    tracing::info!("--- test success with matching prev_log_id (empty entries)");

    // Test 3: Empty entries + matching prev_log_id → success
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, NodeId::from(1)),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
        entries: vec![],
        leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(0), 2)),
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        resp.is_success(),
        "empty entries with matching prev_log_id should succeed"
    );
    assert!(
        !resp.is_conflict(),
        "should not report conflict when prev_log_id matches"
    );

    // Verify logs unchanged (empty entries don't modify log)
    let logs = sto0
        .get_log_reader()
        .await
        .try_get_log_entries(0..=2)
        .await?;
    assert_eq!(logs.len(), 3, "logs should be unchanged after empty append");

    r0.shutdown().await?;

    Ok(())
}
