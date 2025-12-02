/// Test leader stepdown when encountering higher vote during append-entries
///
/// This test validates that a leader correctly reverts to follower state
/// when it receives an append-entries response indicating a higher vote exists.
/// This is critical for election safety to prevent split-brain scenarios.
///
/// Original: openraft/tests/tests/append_entries/t10_see_higher_vote.rs

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::testing::AspenRouter;
use openraft::{BasicNode, Config, ServerState, Vote};
use openraft::storage::RaftLogStorage;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test that leader reverts to follower when seeing higher vote
///
/// Sets up a 2-node cluster where node-0 is leader, then manually
/// upgrades node-1's vote to a higher term. When leader tries to
/// replicate, it should see the higher vote and revert to follower.
#[tokio::test]
async fn test_leader_sees_higher_vote() -> Result<()> {
    // Create config with explicit control over elections
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            enable_elect: false,
            election_timeout_min: 500,
            election_timeout_max: 501,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: setup 2-node cluster");

    // Create nodes
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;

    // Initialize node-0 as single-node cluster
    {
        let n0 = router.get_raft_handle(&0)?;
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(0, BasicNode::default());
        n0.initialize(nodes).await?;
    }

    // Wait for node-0 to become leader
    router
        .wait(&0, timeout())
        .state(ServerState::Leader, "node-0 becomes leader")
        .await?;

    // Add node-1 as learner
    router.add_learner(0, 1).await?;

    // Wait for learner to be added
    router
        .wait(&1, timeout())
        .state(ServerState::Learner, "node-1 becomes learner")
        .await?;

    tracing::info!("--- section 2: manually upgrade node-1's vote to higher term");

    // Extract node-1's storage to directly modify its vote
    let (n1, mut sto1, sm1) = router.remove_node(1).unwrap();
    n1.shutdown().await?;

    // Save a higher vote (term 10 > current term)
    sto1.save_vote(&Vote::new(10, 1)).await?;

    // Re-add node-1 with the modified storage
    router.new_raft_node_with_storage(1, sto1, sm1).await?;

    tracing::info!("--- section 3: trigger leader to see higher vote");

    // Write to leader to trigger append-entries to node-1
    // This should cause the leader to see node-1's higher vote
    let write_result = router.write(&0, "key1".to_string(), "value1".to_string()).await;

    // The write should fail as the leader will step down
    assert!(write_result.is_err(), "write should fail when leader steps down");

    tracing::info!("--- section 4: verify leader reverted to follower");

    // Node-0 should now be a follower after seeing higher vote
    router
        .wait(&0, timeout())
        .state(ServerState::Follower, "node-0 becomes follower after seeing higher vote")
        .await?;

    // Verify node-0 has stored the higher vote
    let n0 = router.get_raft_handle(&0)?;
    let metrics = n0.metrics().borrow().clone();

    assert_eq!(
        metrics.vote,
        Vote::new(10, 1),
        "node-0 should have updated to the higher vote"
    );

    // Verify no leader exists (since we disabled elections)
    let leader = router.leader();
    assert_eq!(leader, None, "no leader should exist after stepdown");

    Ok(())
}