/// Test append entries with large inconsistent log gaps
///
/// This test validates that Raft correctly handles log conflicts when
/// there are many inconsistent entries (>50) between leader and follower.
/// The follower's conflicting entries should be overwritten with the
/// leader's authoritative log.
///
/// Original: openraft/tests/tests/append_entries/t11_append_inconsistent_log.rs
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen_testing::AspenRouter;
use openraft::Config;
use openraft::RaftLogReader;
use openraft::ServerState;
use openraft::storage::RaftLogStorage;
use openraft::storage::RaftLogStorageExt;
use openraft::testing::blank_ent;
use openraft::testing::membership_ent;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test replication with large conflicting log gaps
///
/// Creates artificial state where:
/// - Node-0 has 100 log entries at term 2
/// - Node-2 has 100 log entries at term 3
/// - Node-1 is isolated to prevent voting
/// - Node-2 becomes leader and overwrites node-0's conflicting logs
#[tokio::test]
async fn test_append_inconsistent_log() -> Result<()> {
    // Configure without automatic features
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            enable_elect: false, // Manual election control
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: setup initial cluster state");

    // Create storage with specific log states
    let (mut sto0, sm0) = router.new_store();
    let (mut sto1, sm1) = router.new_store();
    let (mut sto2, sm2) = router.new_store();

    // Initial membership configuration
    let membership = vec![BTreeSet::from([NodeId::from(0), NodeId::from(1), NodeId::from(2)])];
    let initial_log_index = 5u64;

    tracing::info!("--- section 2: create divergent log histories");

    // Node-0: Has logs up to index 100 at term 2
    {
        sto0.save_vote(&openraft::Vote::new(2, NodeId::from(0))).await?;

        // Initial logs
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, NodeId::from(0), 0)];
        for i in 1..=initial_log_index {
            if i == 1 {
                entries.push(membership_ent(1, NodeId::from(0), i, membership.clone()));
            } else {
                entries.push(blank_ent::<AppTypeConfig>(1, NodeId::from(0), i));
            }
        }

        // Add conflicting logs at term 2
        for i in initial_log_index + 1..=100 {
            entries.push(blank_ent::<AppTypeConfig>(2, NodeId::from(0), i));
        }

        sto0.blocking_append(entries).await?;
    }

    // Node-1: Has logs only up to initial point (will be isolated)
    {
        sto1.save_vote(&openraft::Vote::new(1, NodeId::from(1))).await?;

        let mut entries = vec![blank_ent::<AppTypeConfig>(0, NodeId::from(0), 0)];
        for i in 1..=initial_log_index {
            if i == 1 {
                entries.push(membership_ent(1, NodeId::from(0), i, membership.clone()));
            } else {
                entries.push(blank_ent::<AppTypeConfig>(1, NodeId::from(0), i));
            }
        }

        sto1.blocking_append(entries).await?;
    }

    // Node-2: Has logs up to index 100 at term 3 (will become leader)
    {
        sto2.save_vote(&openraft::Vote::new(3, NodeId::from(2))).await?;

        let mut entries = vec![blank_ent::<AppTypeConfig>(0, NodeId::from(0), 0)];
        for i in 1..=initial_log_index {
            if i == 1 {
                entries.push(membership_ent(1, NodeId::from(0), i, membership.clone()));
            } else {
                entries.push(blank_ent::<AppTypeConfig>(1, NodeId::from(0), i));
            }
        }

        // Add different logs at term 3
        for i in initial_log_index + 1..=100 {
            entries.push(blank_ent::<AppTypeConfig>(3, NodeId::from(2), i));
        }

        sto2.blocking_append(entries).await?;
    }

    tracing::info!("--- section 3: start nodes with divergent states");

    // Create nodes with pre-populated storage
    router.new_raft_node_with_storage(0, sto0, sm0).await?;
    router.new_raft_node_with_storage(1, sto1, sm1).await?;
    router.new_raft_node_with_storage(2, sto2, sm2).await?;

    // Isolate node-1 to prevent it from participating in election
    router.fail_node(1);

    tracing::info!("--- section 4: elect node-2 as leader");

    // Manually trigger election on node-2
    {
        let n2 = router.get_raft_handle(2)?;
        n2.trigger().elect().await?;
    }

    // Wait for node-2 to become leader
    router.wait(2, timeout()).state(ServerState::Leader, "node-2 becomes leader with term 3").await?;

    tracing::info!("--- section 5: wait for log conflict resolution");

    // Node-2 (leader) should replicate its logs to node-0
    // This requires overwriting node-0's conflicting entries from term 2

    // Wait for node-0 to sync with leader's log
    // Note: Leader appends a blank log entry when elected, so applied_index will be 101
    router
        .wait(0, Some(Duration::from_secs(5)))
        .applied_index(Some(101), "node-0 syncs to leader's log including blank leader entry")
        .await?;

    tracing::info!("--- section 6: verify conflict resolution");

    // Extract node-0's storage to verify logs were overwritten
    let (r0, mut sto0_after, _sm0_after) = router.remove_node(0).unwrap();
    r0.shutdown().await?;

    // Check specific log entry to verify it was overwritten
    let log_60 = sto0_after.get_log_reader().await.try_get_log_entries(60..=60).await?;

    assert_eq!(log_60.len(), 1, "should have exactly one entry at index 60");

    let entry_60 = &log_60[0];
    assert_eq!(
        entry_60.log_id.leader_id.term, 3,
        "log at index 60 should be from term 3 (leader's term), not term 2"
    );
    assert_eq!(entry_60.log_id.leader_id.node_id, NodeId(2), "log at index 60 should be from node-2 (the leader)");

    // Re-add node-0 to continue testing
    router.new_raft_node_with_storage(0, sto0_after, _sm0_after).await?;

    tracing::info!("--- section 7: test continued operation");

    // Write new data to verify cluster continues functioning
    router
        .write(2, "test_key".to_string(), "test_value".to_string())
        .await
        .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;

    // Verify replication still works
    router.wait(0, timeout()).applied_index(Some(102), "node-0 applies new entry").await?;

    let val = router.read(0, "test_key").await;
    assert_eq!(val, Some("test_value".to_string()), "node-0 should have new data after conflict resolution");

    Ok(())
}
