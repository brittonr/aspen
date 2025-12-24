/// Election tests - ported from OpenRaft's elect tests.
///
/// This test validates election logic with log comparison:
/// - Node with more up-to-date log should win election
/// - Vote requests must have last_log >= local last_log
///
/// Original: openraft/tests/tests/elect/t10_elect_compare_last_log.rs
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen::testing::AspenRouter;
use openraft::Config;
use openraft::ServerState;
use openraft::Vote;
use openraft::storage::RaftLogStorage;
use openraft::storage::RaftLogStorageExt;
use openraft::testing::blank_ent;
use openraft::testing::membership_ent;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Election with log comparison - node with higher term in last log should win.
///
/// Setup:
/// - Node 0: last log entry at term 2, index 1
/// - Node 1: last log entry at term 1, index 2
///
/// Even though node 1 has more entries (index 2 vs 1), node 0 has a higher
/// term in its last entry (term 2 vs 1). According to Raft election rules,
/// node 0's log is more "up-to-date" and should win the election.
#[tokio::test]
async fn test_elect_compare_last_log() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    // Create stores and pre-populate with different log histories
    let (mut sto0, sm0) = router.new_store();
    let (mut sto1, sm1) = router.new_store();

    tracing::info!("--- setup node 0: last log at term=2, index=1");
    {
        sto0.save_vote(&Vote::new(10, NodeId::from(0))).await?;

        // Append membership entry at term 2, index 1
        // This gives node 0 a more recent term than node 1
        sto0.blocking_append([
            blank_ent(0, NodeId::from(0), 0),
            membership_ent(2, NodeId::from(0), 1, vec![BTreeSet::from([NodeId::from(0), NodeId::from(1)])]),
        ])
        .await?;
    }

    tracing::info!("--- setup node 1: last log at term=1, index=2");
    {
        sto1.save_vote(&Vote::new(10, NodeId::from(0))).await?;

        // Append entries up to index 2, but at term 1
        // Node 1 has MORE entries but OLDER term in last entry
        sto1.blocking_append([
            blank_ent(0, NodeId::from(0), 0),
            membership_ent(1, NodeId::from(0), 1, vec![BTreeSet::from([NodeId::from(0), NodeId::from(1)])]),
            blank_ent(1, NodeId::from(0), 2),
        ])
        .await?;
    }

    tracing::info!("--- bring up cluster and verify election");

    // Create nodes with pre-populated storage
    router.new_raft_node_with_storage(0, sto0, sm0).await?;
    router.new_raft_node_with_storage(1, sto1, sm1).await?;

    // Node 0 should become leader because its last log entry has higher term
    router.wait(0, timeout()).state(ServerState::Leader, "node 0 should win election").await?;

    // Verify node 1 is follower
    router.wait(1, timeout()).state(ServerState::Follower, "node 1 should be follower").await?;

    // Verify leader is node 0
    let leader_id = router.leader();
    assert_eq!(leader_id, Some(NodeId::from(0)), "node 0 should be leader (higher term in last log)");

    Ok(())
}
