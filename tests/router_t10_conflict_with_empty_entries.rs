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
use aspen::raft::types::AppTypeConfig;
use aspen::testing::AspenRouter;
use openraft::{Config, ServerState, Vote};
use openraft::raft::AppendEntriesRequest;
use openraft::storage::{RaftLogStorage, RaftLogReader};
use openraft::testing::{blank_ent, log_id};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test that append-entries reports conflicts even with empty entries
///
/// Tests two scenarios:
/// 1. Empty entries with non-existent prev_log_id -> should report conflict
/// 2. Empty entries with mismatched prev_log_id -> should report conflict
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

    tracing::info!("--- section 1: setup single learner node");

    // Create a single node (not initialized, so it remains a learner)
    router.new_raft_node(0).await?;

    // Verify it's in learner state
    router
        .wait(&0, timeout())
        .state(ServerState::Learner, "node-0 is learner")
        .await?;

    tracing::info!("--- section 2: test conflict with non-existent prev_log_id");

    // Extract node for direct RPC testing
    let (r0, mut sto0, sm0) = router.remove_node(0).unwrap();

    // Send append-entries with prev_log_id pointing to non-existent log
    // and empty entries list
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, 1),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, 1, 5)),
        entries: vec![],  // Empty entries
        leader_commit: None,  // No logs committed yet
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        !resp.is_success(),
        "append-entries should fail with non-existent prev_log_id"
    );
    assert!(
        resp.is_conflict(),
        "response should indicate conflict even with empty entries"
    );

    tracing::info!("--- section 3: add some logs to the node");

    // Feed logs using append_entries (this properly updates vote and logs)
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, 1),
        prev_log_id: None,
        entries: vec![
            blank_ent::<AppTypeConfig>(0, 0, 0),
            blank_ent::<AppTypeConfig>(1, 0, 1),
            blank_ent::<AppTypeConfig>(1, 0, 2),
        ],
        leader_commit: Some(log_id::<AppTypeConfig>(1, 0, 2)),
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success(), "should successfully feed logs");

    // Verify logs were added
    let logs = sto0
        .get_log_reader()
        .await
        .try_get_log_entries(0..=2)
        .await?;
    assert_eq!(logs.len(), 3, "should have 3 log entries");

    tracing::info!("--- section 4: test conflict with mismatched prev_log_id");

    // Send append-entries with prev_log_id that exists but doesn't match
    // Node has log at index 2 with term 1, we claim it's at index 3
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, 1),  // Keep same vote to avoid bumping term
        prev_log_id: Some(log_id::<AppTypeConfig>(1, 1, 3)),
        entries: vec![],  // Still empty entries
        leader_commit: Some(log_id::<AppTypeConfig>(1, 0, 2)),  // Only logs 0-2 exist
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        !resp.is_success(),
        "append-entries should fail with mismatched prev_log_id"
    );
    assert!(
        resp.is_conflict(),
        "response should indicate conflict for mismatch even with empty entries"
    );

    tracing::info!("--- section 5: test success with matching prev_log_id");

    // Send append-entries with correct prev_log_id and empty entries
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, 1),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, 0, 2)),
        entries: vec![],  // Empty entries
        leader_commit: Some(log_id::<AppTypeConfig>(1, 0, 2)),
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        resp.is_success(),
        "append-entries should succeed with matching prev_log_id"
    );
    assert!(
        !resp.is_conflict(),
        "response should not indicate conflict when prev_log_id matches"
    );

    // Shutdown the extracted node
    r0.shutdown().await?;

    // Re-add node to router for cleanup
    router.new_raft_node_with_storage(0, sto0, sm0).await?;

    tracing::info!("--- section 6: test with entries after conflict detection");

    // Extract node again for final test
    let (r0, mut sto0, _sm0) = router.remove_node(0).unwrap();

    // Add more logs using append_entries
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(1, 1),
        prev_log_id: Some(log_id::<AppTypeConfig>(1, 0, 2)),
        entries: vec![
            blank_ent::<AppTypeConfig>(1, 0, 3),
            blank_ent::<AppTypeConfig>(1, 0, 4),
            blank_ent::<AppTypeConfig>(1, 0, 5),
        ],
        leader_commit: Some(log_id::<AppTypeConfig>(1, 0, 2)),  // Only committed up to 2 before these entries
    };

    let resp = r0.append_entries(req).await?;
    assert!(resp.is_success(), "should successfully append more logs");

    // Test conflict with entries this time (not empty)
    let req = AppendEntriesRequest {
        vote: Vote::new_committed(2, 1),
        prev_log_id: Some(log_id::<AppTypeConfig>(2, 1, 4)),
        entries: vec![
            blank_ent::<AppTypeConfig>(2, 1, 5),
            blank_ent::<AppTypeConfig>(2, 1, 6),
        ],
        leader_commit: Some(log_id::<AppTypeConfig>(1, 0, 5)),  // Only logs 0-5 exist in term 1
    };

    let resp = r0.append_entries(req).await?;

    assert!(
        !resp.is_success(),
        "append-entries should fail with mismatched prev_log_id even with entries"
    );
    assert!(
        resp.is_conflict(),
        "conflict detection should work with non-empty entries too"
    );

    // Verify logs weren't modified
    let logs = sto0
        .get_log_reader()
        .await
        .try_get_log_entries(0..=5)
        .await?;
    assert_eq!(logs.len(), 6, "logs should not be modified on conflict");
    assert_eq!(
        logs[4].log_id.leader_id.term, 1,
        "log at index 4 should still be from term 1"
    );

    r0.shutdown().await?;

    Ok(())
}