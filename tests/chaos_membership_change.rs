use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

/// Chaos Engineering Test: Membership Change During Leader Crash
///
/// This test validates Raft's most dangerous operation: membership changes during failures.
/// The test specifically targets the joint consensus phase (C-old → C-old,new → C-new) where
/// a leader crash can potentially cause split-brain if not handled correctly.
///
/// The test verifies:
/// - Membership change completes despite leader crash
/// - No split-brain occurs during joint consensus
/// - New configuration is properly replicated
/// - All nodes agree on final membership
///
/// Tiger Style: Fixed membership sizes and timeouts with deterministic behavior.
use aspen::raft::types::NodeId;
use aspen_core::SimulationArtifact;
use aspen::testing::AspenRouter;
use aspen::testing::create_test_raft_member_info;
use openraft::Config;
use openraft::ServerState;

#[tokio::test]
async fn test_membership_change_leader_crash_joint_consensus() {
    let start = Instant::now();
    let seed = 45678u64;
    let mut events = Vec::new();

    let result = run_membership_change_crash_test(&mut events).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let artifact = match &result {
        Ok(()) => SimulationArtifact::new("chaos_membership_change", seed, events, String::new())
            .with_duration_ms(duration_ms),
        Err(e) => SimulationArtifact::new("chaos_membership_change", seed, events, String::new())
            .with_failure(e.to_string())
            .with_duration_ms(duration_ms),
    };

    if let Err(e) = artifact.persist("docs/simulations") {
        eprintln!("Warning: failed to persist simulation artifact: {}", e);
    }

    result.expect("chaos test should succeed");
}

async fn run_membership_change_crash_test(events: &mut Vec<String>) -> anyhow::Result<()> {
    // Start with 3-node cluster (quorum = 2)
    // Use shorter timeouts for faster leader election in chaos scenarios
    let config = Arc::new(
        Config {
            heartbeat_interval: 500,    // 500ms heartbeat
            election_timeout_min: 1500, // 1.5s min election timeout
            election_timeout_max: 3000, // 3s max election timeout
            ..Default::default()
        }
        .validate()?,
    );
    let mut router = AspenRouter::new(config);

    // Create initial 3 nodes
    for i in 0..3 {
        router.new_raft_node(i).await?;
    }
    events.push("initial-cluster: 3 nodes (0,1,2)".into());

    // Initialize cluster
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    // Wait for initial leader
    router
        .wait(0, Some(Duration::from_millis(2000)))
        .state(ServerState::Leader, "initial leader elected")
        .await?;
    let initial_leader = router.leader().ok_or_else(|| anyhow::anyhow!("no initial leader"))?;
    events.push(format!("initial-leader: node {}", initial_leader));

    // Perform baseline writes before membership change
    for i in 0..3 {
        let key = format!("before-membership-{}", i);
        let value = format!("value-{}", i);
        router
            .write(initial_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("baseline-write: {}={}", key, value));
    }

    // Wait for baseline to commit (log index 1 for init + 3 writes = index 4)
    router
        .wait(initial_leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(4), "baseline committed")
        .await?;
    events.push("baseline-committed: 3 writes".into());

    // Add two new nodes as learners first (nodes 3 and 4)
    router.new_raft_node(3).await?;
    router.new_raft_node(4).await?;
    events.push("new-nodes-created: 3, 4".into());

    // Add learners via the leader
    let raft = router.get_raft_handle(initial_leader)?;
    raft.add_learner(NodeId(3), create_test_raft_member_info(3), true).await?;
    raft.add_learner(NodeId(4), create_test_raft_member_info(4), true).await?;
    events.push("learners-added: nodes 3, 4".into());

    // Wait for learners to catch up (2 add_learner ops = index 6)
    router
        .wait(3, Some(Duration::from_millis(2000)))
        .applied_index(Some(6), "learner 3 caught up")
        .await?;
    router
        .wait(4, Some(Duration::from_millis(2000)))
        .applied_index(Some(6), "learner 4 caught up")
        .await?;
    events.push("learners-synced: caught up to index 6".into());

    // Start membership change to add nodes 3,4 as voters
    // This enters joint consensus: C-old={0,1,2} → C-old,new={0,1,2,3,4}
    let new_members: BTreeSet<NodeId> = vec![0, 1, 2, 3, 4].into_iter().map(NodeId).collect();

    // Start the membership change in the background
    let raft_handle = router.get_raft_handle(initial_leader)?;
    let membership_future = tokio::spawn(async move { raft_handle.change_membership(new_members, false).await });
    events.push("membership-change-started: adding nodes 3,4 as voters".into());

    // CRITICAL: Crash the leader during joint consensus phase
    // Tiger Style: Fixed delay of 100ms to hit joint consensus
    tokio::time::sleep(Duration::from_millis(100)).await;
    router.fail_node(initial_leader);
    events.push(format!("leader-crashed-during-joint-consensus: node {}", initial_leader));

    // Wait for new leader election (should happen despite joint consensus)
    // Election timeout max is 3000ms, so wait longer to ensure election completes
    tokio::time::sleep(Duration::from_millis(10000)).await;

    let new_leader = router.leader().ok_or_else(|| anyhow::anyhow!("no new leader after crash"))?;

    if new_leader == initial_leader {
        anyhow::bail!("leader should have changed after crash");
    }
    events.push(format!("new-leader-elected: node {}", new_leader));

    // The membership change should either:
    // 1. Complete successfully (if it was far enough along)
    // 2. Be safely aborted (if it wasn't committed)
    // Either way, the cluster should be consistent

    // Wait for membership operations to settle
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Check if we need to retry the membership change
    let current_membership = {
        let raft = router.get_raft_handle(new_leader)?;
        let metrics = raft.metrics().borrow().clone();
        metrics.membership_config.voter_ids().collect::<BTreeSet<_>>()
    };

    if current_membership.len() == 3 {
        // Membership change was aborted, retry it
        events.push("membership-change-aborted: retrying with new leader".into());

        let raft = router.get_raft_handle(new_leader)?;
        let new_members: BTreeSet<NodeId> = vec![0, 1, 2, 3, 4].into_iter().map(NodeId).collect();
        raft.change_membership(new_members.clone(), false).await?;

        // Wait for completion - membership changes create 2 log entries
        // Just wait for the log to be applied
        tokio::time::sleep(Duration::from_millis(2000)).await;
        events.push("membership-change-completed: 5 voters".into());
    } else {
        events.push("membership-change-survived-crash: 5 voters".into());
    }

    // Drop the old membership future handle (it's either done or failed)
    drop(membership_future);

    // Recover the crashed node
    router.recover_node(initial_leader);
    events.push(format!("recovered-crashed-leader: node {}", initial_leader));

    // Wait for recovered node to catch up and accept new configuration
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Verify new configuration on all nodes
    for node_id in 0..5 {
        let raft = router.get_raft_handle(node_id)?;
        let metrics = raft.metrics().borrow().clone();
        let voters = metrics.membership_config.voter_ids().collect::<BTreeSet<_>>();

        if voters.len() != 5 {
            anyhow::bail!("node {} has wrong voter count: {} (expected 5)", node_id, voters.len());
        }

        for expected_voter in 0..5 {
            if !voters.contains(&NodeId(expected_voter)) {
                anyhow::bail!("node {} missing voter {} in configuration", node_id, expected_voter);
            }
        }
    }
    events.push("membership-verified: all nodes agree on 5-voter configuration".into());

    // Perform writes with new configuration to verify it works
    for i in 0..3 {
        let key = format!("new-config-{}", i);
        let value = format!("five-voters-{}", i);
        router
            .write(new_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("new-config-write: {}={}", key, value));
    }

    // Wait for writes to commit on all 5 nodes
    // We don't know the exact index, so just wait a bit and verify data
    tokio::time::sleep(Duration::from_millis(2000)).await;
    events.push("new-config-writes-committed: all 5 nodes synchronized".into());

    // Verify data consistency across all nodes
    for node_id in 0..5 {
        // Check baseline writes
        for i in 0..3 {
            let key = format!("before-membership-{}", i);
            let expected = format!("value-{}", i);
            match router.read(node_id, &key).await {
                Some(value) if value == expected => {}
                Some(value) => {
                    anyhow::bail!("node {} key {} wrong: got {}, expected {}", node_id, key, value, expected);
                }
                None => anyhow::bail!("node {} missing key {}", node_id, key),
            }
        }

        // Check new configuration writes
        for i in 0..3 {
            let key = format!("new-config-{}", i);
            let expected = format!("five-voters-{}", i);
            match router.read(node_id, &key).await {
                Some(value) if value == expected => {}
                Some(value) => {
                    anyhow::bail!("node {} key {} wrong: got {}, expected {}", node_id, key, value, expected);
                }
                None => anyhow::bail!("node {} missing key {}", node_id, key),
            }
        }
    }
    events.push("data-consistency-verified: all nodes have identical state".into());

    Ok(())
}
