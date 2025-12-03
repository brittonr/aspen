/// Chaos Engineering Test: Leader Crash During Normal Operation
///
/// This test validates Raft's leader election mechanism when the current leader
/// crashes unexpectedly. The test verifies:
/// - New leader is elected after old leader failure
/// - Election completes within timeout window
/// - Writes continue succeeding after new leader elected
/// - Failed leader can rejoin and sync state
///
/// Tiger Style: Fixed timing parameters with deterministic behavior.

use aspen::raft::types::*;
use aspen::simulation::{SimulationArtifact, SimulationStatus};
use aspen::testing::AspenRouter;

use std::time::Instant;

#[tokio::test]
async fn test_leader_crash_triggers_election() {
    let start = Instant::now();
    let seed = 23456u64;
    let mut events = Vec::new();

    let result = run_leader_crash_test(&mut events).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let artifact = match &result {
        Ok(()) => SimulationArtifact::new("chaos_leader_crash", seed, events, String::new())
            .with_duration_ms(duration_ms),
        Err(e) => SimulationArtifact::new("chaos_leader_crash", seed, events, String::new())
            .with_failure(e.to_string())
            .with_duration_ms(duration_ms),
    };

    if let Err(e) = artifact.persist("docs/simulations") {
        eprintln!("Warning: failed to persist simulation artifact: {}", e);
    }

    result.expect("chaos test should succeed");
}

async fn run_leader_crash_test(events: &mut Vec<String>) -> anyhow::Result<()> {
    // Create 3-node cluster (quorum = 2)
    let mut router = AspenRouter::builder().nodes(3).build().await?;
    events.push("cluster-created: 3 nodes".into());

    // Initialize and wait for leader
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    router.wait_for_stable_leadership(2000).await?;
    let initial_leader = router
        .leader()
        .await
        .ok_or_else(|| anyhow::anyhow!("no initial leader elected"))?;
    events.push(format!("initial-leader: node {}", initial_leader));

    // Perform baseline writes
    for i in 0..3 {
        let key = format!("before-crash-{}", i);
        let value = format!("value-{}", i);
        router
            .write(initial_leader, key.clone(), value.clone())
            .await?;
        events.push(format!("baseline-write: {}={}", key, value));
    }

    router.wait_for_log_commit(2, 1000).await?;
    events.push("baseline-committed: 3 writes".into());

    // Crash the current leader
    router.fail_node(initial_leader);
    events.push(format!("leader-crashed: node {}", initial_leader));

    // Wait for new leader election
    // Tiger Style: Fixed timeout of 3 seconds (generous for election + heartbeat)
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    router.wait_for_stable_leadership(2000).await?;
    let new_leader = router
        .leader()
        .await
        .ok_or_else(|| anyhow::anyhow!("no new leader elected after crash"))?;

    // Verify new leader is different from crashed leader
    if new_leader == initial_leader {
        anyhow::bail!(
            "new leader {} should be different from crashed leader {}",
            new_leader,
            initial_leader
        );
    }
    events.push(format!("new-leader-elected: node {}", new_leader));

    // Writes should succeed with new leader
    for i in 0..3 {
        let key = format!("after-crash-{}", i);
        let value = format!("new-leader-{}", i);
        router
            .write(new_leader, key.clone(), value.clone())
            .await?;
        events.push(format!("post-crash-write: {}={}", key, value));
    }

    router.wait_for_log_commit(5, 1000).await?;
    events.push("post-crash-committed: 3 writes".into());

    // Recover the failed leader
    router.recover_node(initial_leader);
    events.push(format!("recovered-node: node {}", initial_leader));

    // Wait for recovered node to catch up
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    router.wait_for_log_commit(5, 2000).await?;
    events.push("recovered-node-synced: caught up to log index 5".into());

    // Verify consistency across all nodes
    for node_id in 0..3 {
        // Check baseline writes
        for i in 0..3 {
            let key = format!("before-crash-{}", i);
            let expected = format!("value-{}", i);
            match router.read(node_id, key.clone()).await {
                Ok(Some(value)) if value == expected => {}
                Ok(Some(value)) => {
                    anyhow::bail!(
                        "node {} key {} wrong: got {}, expected {}",
                        node_id,
                        key,
                        value,
                        expected
                    );
                }
                Ok(None) => anyhow::bail!("node {} missing key {}", node_id, key),
                Err(e) => anyhow::bail!("read error on node {}: {}", node_id, e),
            }
        }

        // Check post-crash writes
        for i in 0..3 {
            let key = format!("after-crash-{}", i);
            let expected = format!("new-leader-{}", i);
            match router.read(node_id, key.clone()).await {
                Ok(Some(value)) if value == expected => {}
                Ok(Some(value)) => {
                    anyhow::bail!(
                        "node {} key {} wrong: got {}, expected {}",
                        node_id,
                        key,
                        value,
                        expected
                    );
                }
                Ok(None) => anyhow::bail!("node {} missing key {}", node_id, key),
                Err(e) => anyhow::bail!("read error on node {}: {}", node_id, e),
            }
        }
    }
    events.push("consistency-verified: all nodes identical".into());

    Ok(())
}
