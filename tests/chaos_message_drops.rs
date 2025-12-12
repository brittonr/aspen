use aspen::raft::types::NodeId;
/// Chaos Engineering Test: Random Message Drops
///
/// This test simulates packet loss in the network to validate Raft's retry mechanisms.
/// Real networks drop 0.1-5% of packets; we test at higher rates (10-20%) to stress
/// the protocol's resilience.
///
/// The test verifies:
/// - Cluster maintains consensus despite message loss
/// - Retries eventually succeed in delivering critical messages
/// - No data loss or corruption from dropped messages
/// - Progress continues (albeit slower) with packet loss
///
/// Tiger Style: Fixed drop patterns with deterministic behavior.
///
/// Note: Since AspenRouter doesn't have native message drop support, we simulate
/// this by rapidly failing/recovering nodes to create intermittent connectivity.
use aspen::simulation::SimulationArtifact;
use aspen::testing::AspenRouter;

use openraft::{Config, ServerState};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_message_drops_append_entries() {
    let start = Instant::now();
    let seed = 56789u64;
    let mut events = Vec::new();

    let result = run_message_drops_test(&mut events).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let artifact = match &result {
        Ok(()) => SimulationArtifact::new("chaos_message_drops", seed, events, String::new())
            .with_duration_ms(duration_ms),
        Err(e) => SimulationArtifact::new("chaos_message_drops", seed, events, String::new())
            .with_failure(e.to_string())
            .with_duration_ms(duration_ms),
    };

    if let Err(e) = artifact.persist("docs/simulations") {
        eprintln!("Warning: failed to persist simulation artifact: {}", e);
    }

    result.expect("chaos test should succeed");
}

async fn run_message_drops_test(events: &mut Vec<String>) -> anyhow::Result<()> {
    // Create 5-node cluster for better testing of message drops
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config);

    // Create all 5 nodes
    for i in 0..5 {
        router.new_raft_node(i).await?;
    }
    events.push("cluster-created: 5 nodes".into());

    // Initialize cluster
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    // Wait for initial leader
    router
        .wait(0, Some(Duration::from_millis(2000)))
        .state(ServerState::Leader, "initial leader elected")
        .await?;
    let leader = router
        .leader()
        .ok_or_else(|| anyhow::anyhow!("no initial leader"))?;
    events.push(format!("initial-leader: node {}", leader));

    // Baseline: writes with no message drops
    for i in 0..5 {
        let key = format!("no-drops-{}", i);
        let value = format!("reliable-{}", i);
        router
            .write(leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("baseline-write: {}={}", key, value));
    }

    // Wait for baseline to commit (init + 5 writes = index 6)
    router
        .wait(leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(6), "baseline committed")
        .await?;
    events.push("baseline-committed: 5 writes with reliable network".into());

    // Phase 1: Configure 10% message drop rate
    events.push("phase1-started: configuring 10% message drops".into());
    router.set_global_message_drop_rate(10);

    // Perform writes with 10% packet loss
    for i in 0..10 {
        let key = format!("with-drops-{}", i);
        let value = format!("unreliable-{}", i);
        router
            .write(leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("drop-phase-write: {}={}", key, value));
    }

    events.push("phase1-completed: 10 writes with 10% drops".into());

    // Let the cluster stabilize
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Phase 2: Heavier message drops (20% loss rate)
    events.push("phase2-started: configuring 20% message drops".into());
    router.set_global_message_drop_rate(20);

    // Perform writes during heavy message drops
    for i in 0..5 {
        let key = format!("heavy-drops-{}", i);
        let value = format!("very-unreliable-{}", i);
        router
            .write(leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("heavy-drop-write: {}={}", key, value));
    }

    events.push("phase2-completed: 5 writes with 20% drops".into());

    // Clear drop rates for final phase
    router.clear_message_drop_rates();

    // Let cluster fully stabilize after heavy message drops
    // Increased to 3000ms for better convergence
    tokio::time::sleep(Duration::from_millis(3000)).await;

    // Phase 3: Verify all data eventually converged despite message drops
    events.push("convergence-check: verifying eventual consistency".into());

    // Calculate expected final log index
    // init (1) + baseline (5) + phase1 (10) + phase2 (5) = 21
    let final_index = 21;

    // Wait for all nodes to catch up
    // Increased timeout to 10000ms for message drop recovery and CI reliability
    for node_id in 0..5 {
        router
            .wait(node_id, Some(Duration::from_millis(10000)))
            .applied_index(Some(final_index), "node caught up after drops")
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "node {} did not converge to index {} within 10000ms after message drops: {}",
                    node_id,
                    final_index,
                    e
                )
            })?;
    }
    events.push(format!(
        "all-nodes-converged: index {} despite message drops",
        final_index
    ));

    // Verify data consistency across all nodes
    for node_id in 0..5 {
        // Check baseline writes (no drops)
        for i in 0..5 {
            let key = format!("no-drops-{}", i);
            let expected = format!("reliable-{}", i);
            match router.read(node_id, &key).await {
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

        // Check phase 1 writes (10% drops)
        for i in 0..10 {
            let key = format!("with-drops-{}", i);
            let expected = format!("unreliable-{}", i);
            match router.read(node_id, &key).await {
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
                None => anyhow::bail!("node {} missing key {} after drops", node_id, key),
            }
        }

        // Check phase 2 writes (20% drops)
        for i in 0..5 {
            let key = format!("heavy-drops-{}", i);
            let expected = format!("very-unreliable-{}", i);
            match router.read(node_id, &key).await {
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
                None => anyhow::bail!("node {} missing key {} after heavy drops", node_id, key),
            }
        }
    }
    events.push("consistency-verified: all data intact despite message drops".into());

    // Final verification: cluster still functional after stress
    for i in 0..3 {
        let key = format!("post-stress-{}", i);
        let value = format!("recovered-{}", i);
        router
            .write(leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("post-stress-write: {}={}", key, value));
    }

    // Verify post-stress writes
    router
        .wait(leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(24), "post-stress writes committed")
        .await?;
    events.push("cluster-healthy: normal operation restored after message drop stress".into());

    Ok(())
}
