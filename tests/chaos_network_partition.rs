use aspen::simulation::SimulationArtifact;
use aspen::testing::AspenRouter;

use openraft::{Config, ServerState};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config);

    // Create all 5 nodes
    for i in 0..5 {
        router.new_raft_node(i).await?;
    }
    events.push("cluster-created: 5 nodes".into());

    // Initialize cluster with all 5 nodes as voters
    router.initialize(0).await?;
    events.push("cluster-initialized: node 0".into());

    // Wait for leader election
    router
        .wait(&0, Some(Duration::from_millis(2000)))
        .state(ServerState::Leader, "leader elected")
        .await?;
    let leader = router
        .leader()
        .ok_or_else(|| anyhow::anyhow!("no leader elected"))?;
    events.push(format!("leader-elected: node {}", leader));

    // Perform some writes to establish baseline
    for i in 0..5 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        router
            .write(&leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("write: {}={}", key, value));
    }

    // Wait for writes to be committed (log index starts at 1 after init, so 5 writes = index 6)
    router
        .wait(&leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(6), "baseline writes committed")
        .await?;
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
    // Check a node in the majority partition
    router
        .wait(&0, Some(Duration::from_millis(2000)))
        .current_leader(leader, "majority partition maintains leader")
        .await?;
    let majority_leader = router
        .leader()
        .ok_or_else(|| anyhow::anyhow!("majority partition lost leadership"))?;

    // Verify majority leader is in majority partition
    if majority_leader == 3 || majority_leader == 4 {
        anyhow::bail!(
            "leader should be in majority partition (0,1,2), got node {}",
            majority_leader
        );
    }
    events.push(format!("majority-leader-stable: node {}", majority_leader));

    // Writes should succeed in majority partition
    for i in 5..10 {
        let key = format!("key{}", i);
        let value = format!("during-partition-{}", i);
        router
            .write(&majority_leader, key.clone(), value.clone())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        events.push(format!("partition-write: {}={}", key, value));
    }

    // Wait for partition writes to be committed (5 more writes = index 11)
    router
        .wait(&majority_leader, Some(Duration::from_millis(1000)))
        .applied_index(Some(11), "partition writes committed")
        .await?;
    events.push("partition-writes-committed: 5 writes".into());

    // Heal the partition by recovering minority nodes
    router.recover_node(3);
    router.recover_node(4);
    events.push("partition-healed: recovered nodes 3, 4".into());

    // Wait for cluster to reconverge
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Check that we still have a leader after healing
    router
        .wait(&0, Some(Duration::from_millis(2000)))
        .current_leader(majority_leader, "leader stable after healing")
        .await?;

    // All nodes should eventually have the same committed log
    // Increased timeout to 3000ms for partition recovery and CI reliability
    for node_id in 0..5 {
        router
            .wait(&node_id, Some(Duration::from_millis(3000)))
            .applied_index(Some(11), "all nodes synchronized")
            .await?;
    }
    events.push("cluster-reconverged: all nodes at log index 11".into());

    // Verify all writes are present on all nodes (consistency check)
    for node_id in 0..5 {
        for i in 0..10 {
            let key = format!("key{}", i);
            let expected = if i < 5 {
                format!("value{}", i)
            } else {
                format!("during-partition-{}", i)
            };

            match router.read(&node_id, &key).await {
                Some(value) if value == expected => {
                    // Expected value found
                }
                Some(value) => {
                    anyhow::bail!(
                        "node {} key {} has wrong value: got {}, expected {}",
                        node_id,
                        key,
                        value,
                        expected
                    );
                }
                None => {
                    anyhow::bail!("node {} missing key {}", node_id, key);
                }
            }
        }
    }
    events.push("consistency-verified: all nodes have identical state".into());

    Ok(())
}
