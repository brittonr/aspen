/// Chaos Engineering Test: Slow Network (High Latency)
///
/// This test validates Raft's behavior under high network latency conditions.
/// The test verifies:
/// - Cluster maintains stability despite slow network
/// - Writes eventually succeed even with high latency
/// - No split-brain occurs despite delayed heartbeats
/// - Cluster recovers quickly when network latency returns to normal
///
/// Tiger Style: Fixed latency parameters (200ms) with deterministic behavior.

use aspen::raft::types::*;
use aspen::simulation::{SimulationArtifact, SimulationStatus};
use aspen::testing::AspenRouter;

use std::time::Instant;

#[tokio::test]
async fn test_slow_network_high_latency() {
    let start = Instant::now();
    let seed = 34567u64;
    let mut events = Vec::new();

    let result = run_slow_network_test(&mut events).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let artifact = match &result {
        Ok(()) => SimulationArtifact::new("chaos_slow_network", seed, events, String::new())
            .with_duration_ms(duration_ms),
        Err(e) => SimulationArtifact::new("chaos_slow_network", seed, events, String::new())
            .with_failure(e.to_string())
            .with_duration_ms(duration_ms),
    };

    if let Err(e) = artifact.persist("docs/simulations") {
        eprintln!("Warning: failed to persist simulation artifact: {}", e);
    }

    result.expect("chaos test should succeed");
}

async fn run_slow_network_test(events: &mut Vec<String>) -> anyhow::Result<()> {
    // Create 3-node cluster
    let mut router = AspenRouter::builder().nodes(3).build().await?;
    events.push("cluster-created: 3 nodes".into());

    // Initialize with normal network
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    router.wait_for_stable_leadership(2000).await?;
    let leader = router
        .leader()
        .await
        .ok_or_else(|| anyhow::anyhow!("no leader elected"))?;
    events.push(format!("leader-elected: node {}", leader));

    // Baseline writes with normal network
    for i in 0..3 {
        let key = format!("normal-latency-{}", i);
        let value = format!("value-{}", i);
        router.write(leader, key.clone(), value.clone()).await?;
        events.push(format!("baseline-write: {}={}", key, value));
    }

    router.wait_for_log_commit(2, 1000).await?;
    events.push("baseline-committed: 3 writes at normal latency".into());

    // Introduce high network latency
    // Tiger Style: Fixed 200ms delay to simulate slow WAN
    router.set_network_delay(200);
    events.push("network-degraded: 200ms latency added".into());

    // Wait for cluster to adapt to slow network
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Verify cluster still has a leader despite slow network
    router.wait_for_stable_leadership(3000).await?;
    let slow_leader = router
        .leader()
        .await
        .ok_or_else(|| anyhow::anyhow!("lost leadership during slow network"))?;
    events.push(format!("leader-stable-despite-latency: node {}", slow_leader));

    // Writes should still succeed, just slower
    for i in 0..3 {
        let key = format!("high-latency-{}", i);
        let value = format!("slow-{}", i);
        router
            .write(slow_leader, key.clone(), value.clone())
            .await?;
        events.push(format!("slow-write: {}={}", key, value));
    }

    // Tiger Style: Increased timeout for slow network (3 seconds)
    router.wait_for_log_commit(5, 3000).await?;
    events.push("slow-writes-committed: 3 writes with 200ms latency".into());

    // Return network to normal
    router.set_network_delay(0);
    events.push("network-restored: latency removed".into());

    // Wait for cluster to stabilize at normal speed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Fast writes should succeed again
    for i in 0..3 {
        let key = format!("restored-{}", i);
        let value = format!("fast-again-{}", i);
        router
            .write(slow_leader, key.clone(), value.clone())
            .await?;
        events.push(format!("fast-write: {}={}", key, value));
    }

    router.wait_for_log_commit(8, 1000).await?;
    events.push("restored-writes-committed: 3 writes at normal latency".into());

    // Verify all writes are present on all nodes
    for node_id in 0..3 {
        // Normal latency writes
        for i in 0..3 {
            let key = format!("normal-latency-{}", i);
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

        // High latency writes
        for i in 0..3 {
            let key = format!("high-latency-{}", i);
            let expected = format!("slow-{}", i);
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

        // Restored writes
        for i in 0..3 {
            let key = format!("restored-{}", i);
            let expected = format!("fast-again-{}", i);
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
    events.push("consistency-verified: all writes present on all nodes".into());

    Ok(())
}
