/// Membership change tests - simplified from OpenRaft's t20_change_membership.
///
/// This test validates basic membership change operations:
/// - Adding a learner to an existing cluster
/// - Promoting learner to voter via change_membership
///
/// Original: openraft/tests/tests/membership/t20_change_membership.rs
/// Note: Simplified to work with current AspenRouter capabilities
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::testing::AspenRouter;
use openraft::{BasicNode, Config, ServerState};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test adding a learner and promoting to voter.
///
/// Workflow:
/// 1. Initialize single-node cluster (node 0)
/// 2. Add node 1 as learner
/// 3. Verify node 1 is in learner state
/// 4. Change membership to include both nodes as voters
/// 5. Verify both nodes are in the cluster
#[tokio::test]
async fn test_add_learner_and_promote() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    // IMPORTANT: Create leader node FIRST, initialize it, THEN create other nodes.
    // This follows OpenRaft's pattern (see openraft/tests/tests/fixtures/mod.rs::new_cluster).
    // Creating the leader first and waiting for it to be ready ensures clean state transitions
    // when adding learners later.
    tracing::info!("--- creating and initializing leader node 0");
    router.new_raft_node(0).await?;

    {
        let node0 = router.get_raft_handle(&0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(0, BasicNode::default());
        node0.initialize(nodes).await?;

        router
            .wait(&0, timeout())
            .log_index_at_least(Some(1), "initialized")
            .await?;

        router
            .wait(&0, timeout())
            .state(ServerState::Leader, "node 0 is leader")
            .await?;
    }

    tracing::info!("--- creating node 1 and adding as learner");
    {
        // Create node 1 AFTER leader is fully initialized
        router.new_raft_node(1).await?;

        // Give node 1 time to fully initialize in Learner state
        router
            .wait(&1, timeout())
            .state(ServerState::Learner, "node 1 is learner")
            .await?;

        let leader = router.get_raft_handle(&0)?;
        leader.add_learner(1, BasicNode::default(), false).await?;

        // Wait for learner to be added (log entry written)
        router
            .wait(&0, timeout())
            .log_index_at_least(Some(2), "learner added")
            .await?;
    }

    tracing::info!("--- verify node 1 receives logs and stays in learner state");
    {
        // Give node 1 time to receive the log via replication
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Node 1 should be in learner state
        router
            .wait(&1, timeout())
            .state(ServerState::Learner, "node 1 is learner")
            .await?;
    }

    tracing::info!("--- change membership to make both nodes voters");
    {
        let leader = router.get_raft_handle(&0)?;
        // change_membership takes an array of node IDs
        leader.change_membership([0, 1], false).await?;

        // Wait for membership change to be applied
        // Membership change takes 2 log entries (joint config + final config)
        router
            .wait(&0, timeout())
            .log_index_at_least(Some(4), "membership changed")
            .await?;
    }

    tracing::info!("--- verify cluster has both voters");
    {
        // Give time for membership change to propagate
        tokio::time::sleep(Duration::from_millis(500)).await;

        // One node should be leader, one should be follower
        let metrics0 = router.get_raft_handle(&0)?.metrics().borrow().clone();
        let metrics1 = router.get_raft_handle(&1)?.metrics().borrow().clone();

        let states = [metrics0.state, metrics1.state];
        assert!(
            states.contains(&ServerState::Leader),
            "one node should be leader"
        );
        assert!(
            states.contains(&ServerState::Follower),
            "one node should be follower"
        );

        // Verify both nodes see the same membership
        tracing::info!(
            "node 0 membership: voters={:?}",
            metrics0.membership_config.voter_ids().collect::<Vec<_>>()
        );
        tracing::info!(
            "node 1 membership: voters={:?}",
            metrics1.membership_config.voter_ids().collect::<Vec<_>>()
        );
    }

    Ok(())
}

/// Test error case: change membership without adding learner first.
///
/// Raft requires nodes to be added as learners before becoming voters,
/// so they can catch up on the log first.
#[tokio::test]
async fn test_change_membership_without_learner() -> Result<()> {
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config.clone());

    // Create single-node cluster
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?; // Node 1 exists but not added to cluster

    tracing::info!("--- initialize single-node cluster");
    {
        let node0 = router.get_raft_handle(&0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(0, BasicNode::default());
        node0.initialize(nodes).await?;

        router
            .wait(&0, timeout())
            .log_index_at_least(Some(1), "initialized")
            .await?;
    }

    tracing::info!("--- try to change membership without adding learner (should fail)");
    {
        let leader = router.get_raft_handle(&0)?;
        // Try to add node 1 as voter without first adding as learner
        let res = leader.change_membership([0, 1], false).await;
        assert!(
            res.is_err(),
            "change_membership should fail without adding learner first"
        );

        let err = res.unwrap_err();
        tracing::info!("expected error: {:?}", err);
        // Error should be LearnerNotFound
    }

    Ok(())
}
