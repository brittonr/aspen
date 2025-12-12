use aspen::raft::types::NodeId;
use aspen::simulation::SimulationArtifact;
use aspen::testing::AspenRouter;

use openraft::{Config, ServerState};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test]
#[ignore] // TODO: Needs enable_tick: false or longer timeouts - 200ms latency causes heartbeat timeouts
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
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config);

    // Create all 3 nodes
    for i in 0..3 {
        router.new_raft_node(i).await?;
    }
    events.push("cluster-created: 3 nodes".into());

    // Initialize with normal network
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    router
        .wait(0, Some(Duration::from_millis(2000)))
        .state(ServerState::Leader, "initial leader elected")
        .await?;
    let leader = router
        .leader()
        .ok_or_else(|| anyhow::anyhow!("no leader elected"))?;
    events.push(format!("leader-elected: node {}", leader));

    // Baseline writes with normal network
    for i in 0..3 {
        let key = format!("normal-latency-{}", i);
        let value = format!("value-{}", i);
        router
            .write(leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("baseline-write: {}={}", key, value));
    }

    // Wait for baseline writes to be committed (log index starts at 1 after init, so 3 writes = index 4)
    router
        .wait(leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(4), "baseline writes committed")
        .await?;
    events.push("baseline-committed: 3 writes at normal latency".into());

    // Introduce high network latency
    // Tiger Style: Fixed 200ms delay to simulate slow WAN
    router.set_global_network_delay(200);
    events.push("network-degraded: 200ms latency added".into());

    // Wait for cluster to adapt to slow network (longer wait for CI)
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // Verify cluster still has a leader despite slow network
    router
        .wait(0, Some(Duration::from_millis(5000)))
        .current_leader(leader, "leader stable despite latency")
        .await?;
    let slow_leader = router
        .leader()
        .ok_or_else(|| anyhow::anyhow!("lost leadership during slow network"))?;
    events.push(format!(
        "leader-stable-despite-latency: node {}",
        slow_leader
    ));

    // Writes should still succeed, just slower
    for i in 0..3 {
        let key = format!("high-latency-{}", i);
        let value = format!("slow-{}", i);
        router
            .write(slow_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("slow-write: {}={}", key, value));
    }

    // Increased timeout for slow network: 200ms latency * 2 (RTT) * 2 (quorum) + overhead
    // Use 10 seconds to account for CI slowness and multiple round trips
    router
        .wait(slow_leader, Some(Duration::from_millis(10000)))
        .applied_index(Some(7), "slow writes committed")
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "slow writes did not commit within 10000ms (200ms network latency): {}",
                e
            )
        })?;
    events.push("slow-writes-committed: 3 writes with 200ms latency".into());

    // Return network to normal
    router.clear_network_delays();
    events.push("network-restored: latency removed".into());

    // Wait for cluster to stabilize at normal speed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Fast writes should succeed again
    for i in 0..3 {
        let key = format!("restored-{}", i);
        let value = format!("fast-again-{}", i);
        router
            .write(slow_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("fast-write: {}={}", key, value));
    }

    // Fast commits again (3 more writes = index 10)
    // Increased timeout for CI reliability
    router
        .wait(slow_leader, Some(Duration::from_millis(2000)))
        .applied_index(Some(10), "restored writes committed")
        .await?;
    events.push("restored-writes-committed: 3 writes at normal latency".into());

    // Verify all writes are present on all nodes
    for node_id in 0..3 {
        // Normal latency writes
        for i in 0..3 {
            let key = format!("normal-latency-{}", i);
            let expected = format!("value-{}", i);
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

        // High latency writes
        for i in 0..3 {
            let key = format!("high-latency-{}", i);
            let expected = format!("slow-{}", i);
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

        // Restored writes
        for i in 0..3 {
            let key = format!("restored-{}", i);
            let expected = format!("fast-again-{}", i);
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
    }
    events.push("consistency-verified: all writes present on all nodes".into());

    Ok(())
}
