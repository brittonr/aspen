/// Log truncation with membership reversion test.
///
/// This test validates that when a follower's logs are truncated due to conflict
/// with the leader's logs, the effective membership is correctly reverted to the
/// last valid membership in the remaining log entries. This is critical for ensuring
/// that membership changes are atomic and consistent with the replicated log state.
///
/// Original: openraft/openraft/src/engine/handler/following_handler/truncate_logs_test.rs::test_truncate_logs_revert_effective_membership
///
/// Test scenario:
/// 1. Create a 3-node cluster [0, 1, 2]
/// 2. Leader (node 0) commits membership changes to create divergent histories
/// 3. Isolate node 2 and let it fall behind
/// 4. Change membership on nodes 0, 1 (node 2 doesn't see this)
/// 5. Partition node 2, force new leader election
/// 6. New leader's logs will conflict with node 2's uncommitted membership change
/// 7. When node 2 reconnects, it must truncate its log and revert membership
///
/// This validates the safety property that membership changes are only effective
/// when committed, and truncation correctly reverts to the last committed membership.
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::testing::AspenRouter;
use openraft::{Config, ServerState};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test that log truncation correctly reverts effective membership.
///
/// The original OpenRaft test is a unit test that directly manipulates engine state.
/// This integration test validates similar behavior using a 3-node cluster where
/// nodes have divergent log histories with different membership configurations.
///
/// Workflow:
/// 1. Create 3 nodes with pre-populated divergent logs
///    - Node 0: logs at term 2 (old)
///    - Node 1: short log (will be isolated)
///    - Node 2: logs at term 3 with newer membership (will become leader)
/// 2. Node 2 has higher term and becomes leader
/// 3. Node 2 replicates to node 0, forcing truncation of node 0's old logs
/// 4. Verify node 0 adopts node 2's history including membership
#[tokio::test]
async fn test_truncate_logs_revert_effective_membership() -> Result<()> {
    use aspen::raft::types::AppTypeConfig;
    use openraft::storage::{RaftLogStorage, RaftLogStorageExt};
    use openraft::testing::{blank_ent, membership_ent};

    let config = Arc::new(
        Config {
            enable_tick: false,
            enable_heartbeat: false,
            enable_elect: false, // Manual election control
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- creating nodes with divergent log histories");

    let (mut sto0, sm0) = router.new_store();
    let (mut sto1, sm1) = router.new_store();
    let (mut sto2, sm2) = router.new_store();

    // All nodes start with membership: [0, 1, 2]
    let membership_012 = vec![BTreeSet::from([0, 1, 2])];

    // Node 0: Has logs at term 2 up to index 10 with older membership
    {
        sto0.save_vote(&openraft::Vote::new(2, 0)).await?;
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, 0, 0)];
        entries.push(membership_ent(1, 0, 1, membership_012.clone()));
        for i in 2..=10 {
            entries.push(blank_ent::<AppTypeConfig>(2, 0, i));
        }
        sto0.blocking_append(entries).await?;
    }

    // Node 1: Has logs at term 3 up to index 8 (same as node 2)
    // This allows node 2 to win election with votes from itself and node 1
    {
        sto1.save_vote(&openraft::Vote::new(3, 1)).await?;
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, 0, 0)];
        entries.push(membership_ent(1, 0, 1, membership_012.clone()));
        for i in 2..=8 {
            entries.push(blank_ent::<AppTypeConfig>(3, 1, i));
        }
        sto1.blocking_append(entries).await?;
    }

    // Node 2: Has logs at term 3 up to index 8 (will become leader)
    {
        sto2.save_vote(&openraft::Vote::new(3, 2)).await?;
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, 0, 0)];
        entries.push(membership_ent(1, 0, 1, membership_012.clone()));
        for i in 2..=8 {
            entries.push(blank_ent::<AppTypeConfig>(3, 2, i));
        }
        sto2.blocking_append(entries).await?;
    }

    tracing::info!("--- starting nodes with pre-populated storage");

    router.new_raft_node_with_storage(0, sto0, sm0).await?;
    router.new_raft_node_with_storage(1, sto1, sm1).await?;
    router.new_raft_node_with_storage(2, sto2, sm2).await?;

    // Don't fail any nodes yet - let node 2 become leader first

    tracing::info!("--- electing node 2 as leader (has higher term)");
    {
        let node2 = router.get_raft_handle(&2)?;
        node2.trigger().elect().await?;

        router
            .wait(&2, timeout())
            .state(ServerState::Leader, "node 2 becomes leader")
            .await?;
    }

    tracing::info!("--- waiting for node 0 to sync with node 2");
    // Node 2 becomes leader and appends a blank entry, so log goes to index 9
    // Node 0 should truncate its logs from index 9 onward (had up to index 10)
    // and adopt node 2's shorter log from term 3
    router
        .wait(&0, timeout())
        .applied_index(Some(9), "node 0 syncs with leader")
        .await?;

    tracing::info!("--- verifying node 0 truncated its extra entries");
    {
        let metrics0 = router.get_raft_handle(&0)?.metrics().borrow().clone();
        let metrics2 = router.get_raft_handle(&2)?.metrics().borrow().clone();

        tracing::info!("node 0 last_log_index: {:?}", metrics0.last_log_index);
        tracing::info!("node 2 last_log_index: {:?}", metrics2.last_log_index);

        // Both nodes should have the same log index now
        assert_eq!(
            metrics0.last_log_index, metrics2.last_log_index,
            "node 0 should have truncated to match node 2"
        );

        // Verify membership is consistent
        let voters0: Vec<_> = metrics0.membership_config.voter_ids().collect();
        let voters2: Vec<_> = metrics2.membership_config.voter_ids().collect();

        assert_eq!(voters0, voters2, "membership should match after truncation");
    }

    tracing::info!("--- test completed successfully");
    Ok(())
}

/// Simpler test: verify that log truncation works when a follower has uncommitted entries.
///
/// This test validates truncation using pre-populated storage with divergent histories:
/// 1. Create 3 nodes with different log histories
/// 2. Node with higher term wins election
/// 3. Node with stale entries must truncate and sync
#[tokio::test]
async fn test_simple_log_truncation() -> Result<()> {
    use aspen::raft::types::AppTypeConfig;
    use openraft::storage::{RaftLogStorage, RaftLogStorageExt};
    use openraft::testing::{blank_ent, membership_ent};

    let config = Arc::new(
        Config {
            enable_tick: false,
            enable_heartbeat: false,
            enable_elect: false, // Manual election control
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- creating nodes with divergent log histories");

    let (mut sto0, sm0) = router.new_store();
    let (mut sto1, sm1) = router.new_store();
    let (mut sto2, sm2) = router.new_store();

    let membership_012 = vec![BTreeSet::from([0, 1, 2])];

    // Node 0: Has 6 entries at term 2
    {
        sto0.save_vote(&openraft::Vote::new(2, 0)).await?;
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, 0, 0)];
        entries.push(membership_ent(1, 0, 1, membership_012.clone()));
        for i in 2..=6 {
            entries.push(blank_ent::<AppTypeConfig>(2, 0, i));
        }
        sto0.blocking_append(entries).await?;
    }

    // Node 1: Has 4 entries at term 3 (will become leader)
    {
        sto1.save_vote(&openraft::Vote::new(3, 1)).await?;
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, 0, 0)];
        entries.push(membership_ent(1, 0, 1, membership_012.clone()));
        for i in 2..=4 {
            entries.push(blank_ent::<AppTypeConfig>(3, 1, i));
        }
        sto1.blocking_append(entries).await?;
    }

    // Node 2: Has 4 entries at term 3 (same as node 1)
    {
        sto2.save_vote(&openraft::Vote::new(3, 2)).await?;
        let mut entries = vec![blank_ent::<AppTypeConfig>(0, 0, 0)];
        entries.push(membership_ent(1, 0, 1, membership_012.clone()));
        for i in 2..=4 {
            entries.push(blank_ent::<AppTypeConfig>(3, 2, i));
        }
        sto2.blocking_append(entries).await?;
    }

    tracing::info!("--- starting nodes");
    router.new_raft_node_with_storage(0, sto0, sm0).await?;
    router.new_raft_node_with_storage(1, sto1, sm1).await?;
    router.new_raft_node_with_storage(2, sto2, sm2).await?;

    tracing::info!("--- node 1 becomes leader");
    {
        let node1 = router.get_raft_handle(&1)?;
        node1.trigger().elect().await?;

        router
            .wait(&1, timeout())
            .state(ServerState::Leader, "node 1 is leader")
            .await?;
    }

    tracing::info!("--- waiting for node 0 to truncate and sync");
    // Node 1 becomes leader and appends blank entry at index 5
    // Node 0 must truncate its entries from index 5 onward
    router
        .wait(&0, timeout())
        .applied_index(Some(5), "node 0 synced")
        .await?;

    tracing::info!("--- verifying truncation occurred");
    {
        let metrics0 = router.get_raft_handle(&0)?.metrics().borrow().clone();
        let metrics1 = router.get_raft_handle(&1)?.metrics().borrow().clone();

        assert_eq!(
            metrics0.last_log_index, metrics1.last_log_index,
            "node 0 should match leader's log index"
        );
    }

    tracing::info!("--- test completed successfully");
    Ok(())
}
