use aspen::simulation::SimulationArtifact;
use aspen::testing::AspenRouter;

use openraft::{Config, ServerState};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    // Use shorter timeouts for faster leader election in chaos scenarios
    let config = Arc::new(
        Config {
            heartbeat_interval: 500,           // 500ms heartbeat
            election_timeout_min: 1500,        // 1.5s min election timeout
            election_timeout_max: 3000,        // 3s max election timeout
            ..Default::default()
        }
        .validate()?,
    );
    let mut router = AspenRouter::new(config);

    // Create all 3 nodes
    for i in 0..3 {
        router.new_raft_node(i).await?;
    }
    events.push("cluster-created: 3 nodes".into());

    // Initialize and wait for leader
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    router.wait(&0, Some(Duration::from_millis(2000)))
        .state(ServerState::Leader, "initial leader elected")
        .await?;
    let initial_leader = router
        .leader()
        .ok_or_else(|| anyhow::anyhow!("no initial leader elected"))?;
    events.push(format!("initial-leader: node {}", initial_leader));

    // Perform baseline writes
    for i in 0..3 {
        let key = format!("before-crash-{}", i);
        let value = format!("value-{}", i);
        router
            .write(&initial_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("baseline-write: {}={}", key, value));
    }

    // Wait for baseline writes to be committed (log index starts at 1 after init, so 3 writes = index 4)
    router.wait(&initial_leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(4), "baseline writes committed")
        .await?;
    events.push("baseline-committed: 3 writes".into());

    // Crash the current leader
    router.fail_node(initial_leader);
    events.push(format!("leader-crashed: node {}", initial_leader));

    // Wait for followers to detect leader failure and start election
    // Election timeout max is 3000ms, so wait longer to ensure election completes
    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;

    // Check all node states after election
    for node_id in 0..3 {
        if let Ok(raft) = router.get_raft_handle(&node_id) {
            let metrics = raft.metrics().borrow().clone();
            events.push(format!(
                "node-{}-state: {:?}, term: {}, current_leader: {:?}",
                node_id, metrics.state, metrics.current_term, metrics.current_leader
            ));
        }
    }

    let new_leader = router
        .leader()
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
            .write(&new_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("post-crash-write: {}={}", key, value));
    }

    // Wait for post-crash writes to be committed
    // New leader writes blank entry on election (index 5), then 3 writes (indices 6,7,8)
    router.wait(&new_leader, Some(Duration::from_millis(2000)))
        .applied_index(Some(8), "post-crash writes committed")
        .await?;
    events.push("post-crash-committed: 3 writes".into());

    // Recover the failed leader
    router.recover_node(initial_leader);
    events.push(format!("recovered-node: node {}", initial_leader));

    // Wait for recovered node to catch up
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    router.wait(&initial_leader, Some(Duration::from_millis(3000)))
        .applied_index(Some(8), "recovered node synced")
        .await?;
    events.push("recovered-node-synced: caught up to log index 8".into());

    // Verify consistency across all nodes
    for node_id in 0..3 {
        // Check baseline writes
        for i in 0..3 {
            let key = format!("before-crash-{}", i);
            let expected = format!("value-{}", i);
            match router.read(&node_id, &key).await {
                Some(value) if value == expected => {}
                Some(value) => {
                    anyhow::bail!(
                        "node {} key {} wrong: got {}, expected {}",
                        node_id,
                        key,
                        value,
                        expected
                    );
                }
                None => anyhow::bail!("node {} missing key {}", node_id, key),
            }
        }

        // Check post-crash writes
        for i in 0..3 {
            let key = format!("after-crash-{}", i);
            let expected = format!("new-leader-{}", i);
            match router.read(&node_id, &key).await {
                Some(value) if value == expected => {}
                Some(value) => {
                    anyhow::bail!(
                        "node {} key {} wrong: got {}, expected {}",
                        node_id,
                        key,
                        value,
                        expected
                    );
                }
                None => anyhow::bail!("node {} missing key {}", node_id, key),
            }
        }
    }
    events.push("consistency-verified: all nodes identical".into());

    Ok(())
}