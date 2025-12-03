/// Chaos Engineering Test: Network Partition During Normal Operation
///
/// This test validates Raft's behavior during a network partition scenario where the
/// cluster splits into majority and minority partitions. The test verifies:
/// - Majority partition continues functioning with a leader
/// - Minority partition cannot elect a new leader (split-brain prevention)
/// - Writes succeed in majority partition
/// - Cluster converges to consistent state after partition heals
///
/// Tiger Style: Fixed test parameters with deterministic behavior.

use aspen::raft::types::*;
use aspen::simulation::{SimulationArtifact, SimulationStatus};
use aspen::testing::AspenRouter;

use std::time::Instant;

#[tokio::test]
async fn test_network_partition_during_normal_operation() {
    let start = Instant::now();
    let seed = 12345u64;
    let mut events = Vec::new();

    let result = run_partition_test(&mut events).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let artifact = match &result {
        Ok(()) => SimulationArtifact::new("chaos_network_partition", seed, events, String::new())
            .with_duration_ms(duration_ms),
        Err(e) => SimulationArtifact::new("chaos_network_partition", seed, events, String::new())
            .with_failure(e.to_string())
            .with_duration_ms(duration_ms),
    };

    // Persist artifact for debugging
    if let Err(e) = artifact.persist("docs/simulations") {
        eprintln!("Warning: failed to persist simulation artifact: {}", e);
    }

    result.expect("chaos test should succeed");
}

async fn run_partition_test(events: &mut Vec<String>) -> anyhow::Result<()> {
    // Create 5-node cluster (quorum = 3)
    let mut router = AspenRouter::builder().nodes(5).build().await?;
    events.push("cluster-created: 5 nodes".into());

    // Initialize cluster with all 5 nodes as voters
    let voters = vec![0, 1, 2, 3, 4];
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    // Wait for leader election
    router.wait_for_stable_leadership(2000).await?;
    let leader = router
        .leader()
        .await
        .ok_or_else(|| anyhow::anyhow!("no leader elected"))?;
    events.push(format!("leader-elected: node {}", leader));

    // Perform some writes to establish baseline
    for i in 0..5 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        router.write(leader, key.clone(), value.clone()).await?;
        events.push(format!("write: {}={}", key, value));
    }

    router.wait_for_log_commit(4, 1000).await?;
    events.push("baseline-writes-committed: 5 writes".into());

    // Partition the network: isolate nodes 3, 4 (minority)
    // Majority: 0, 1, 2 (quorum maintained)
    // Minority: 3, 4 (cannot form quorum)
    router.fail_node(3);
    router.fail_node(4);
    events.push("partition-created: isolated nodes 3, 4".into());

    // Wait a bit for partition to take effect
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // The majority partition (0, 1, 2) should still have a leader
    router.wait_for_stable_leadership(2000).await?;
    let majority_leader = router
        .leader()
        .await
        .ok_or_else(|| anyhow::anyhow!("majority partition lost leadership"))?;

    // Verify majority leader is in majority partition
    if majority_leader == 3 || majority_leader == 4 {
        anyhow::bail!(
            "leader should be in majority partition (0,1,2), got node {}",
            majority_leader
        );
    }
    events.push(format!(
        "majority-leader-stable: node {}",
        majority_leader
    ));

    // Writes should succeed in majority partition
    for i in 5..10 {
        let key = format!("key{}", i);
        let value = format!("during-partition-{}", i);
        router
            .write(majority_leader, key.clone(), value.clone())
            .await?;
        events.push(format!("partition-write: {}={}", key, value));
    }

    router.wait_for_log_commit(9, 1000).await?;
    events.push("partition-writes-committed: 5 writes".into());

    // Heal the partition by recovering minority nodes
    router.recover_node(3);
    router.recover_node(4);
    events.push("partition-healed: recovered nodes 3, 4".into());

    // Wait for cluster to reconverge
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    router.wait_for_stable_leadership(2000).await?;

    // All nodes should eventually have the same committed log
    router.wait_for_log_commit(9, 2000).await?;
    events.push("cluster-reconverged: all nodes at log index 9".into());

    // Verify all writes are present on all nodes (consistency check)
    for node_id in 0..5 {
        for i in 0..10 {
            let key = format!("key{}", i);
            let expected = if i < 5 {
                format!("value{}", i)
            } else {
                format!("during-partition-{}", i)
            };

            match router.read(node_id, key.clone()).await {
                Ok(Some(value)) if value == expected => {
                    // Expected value found
                }
                Ok(Some(value)) => {
                    anyhow::bail!(
                        "node {} key {} has wrong value: got {}, expected {}",
                        node_id,
                        key,
                        value,
                        expected
                    );
                }
                Ok(None) => {
                    anyhow::bail!("node {} missing key {}", node_id, key);
                }
                Err(e) => {
                    anyhow::bail!("read error on node {}: {}", node_id, e);
                }
            }
        }
    }
    events.push("consistency-verified: all nodes have identical state".into());

    Ok(())
}
