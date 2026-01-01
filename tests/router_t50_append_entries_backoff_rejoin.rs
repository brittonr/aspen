/// Test replication recovery when previously unreachable node rejoins
///
/// This test validates that when a node that was temporarily partitioned
/// (unreachable) rejoins the cluster, it properly catches up to the
/// current log state through append-entries replication.
///
/// Original: openraft/tests/tests/replication/t50_append_entries_backoff_rejoin.rs
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen_testing::AspenRouter;
use openraft::Config;
use openraft::ServerState;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test node recovery after network partition heals
///
/// 1. Create 3-node cluster {0, 1, 2}
/// 2. Isolate node-0 (make it unreachable)
/// 3. Elect node-1 as new leader
/// 4. Write entries to new leader
/// 5. Restore node-0 connectivity
/// 6. Verify node-0 catches up via replication
#[tokio::test]
async fn test_append_entries_backoff_rejoin() -> Result<()> {
    // Configure with manual election control
    let config = Arc::new(
        Config {
            election_timeout_min: 100,
            election_timeout_max: 200,
            enable_elect: false, // Manual election control
            enable_heartbeat: true,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: setup 3-node cluster");

    // Create cluster with nodes {0, 1, 2}
    let log_index = router.new_cluster(BTreeSet::from([NodeId(0), NodeId(1), NodeId(2)]), BTreeSet::new()).await?;

    // Verify initial leader
    router.wait(0, timeout()).state(ServerState::Leader, "node-0 is initial leader").await?;

    tracing::info!("--- section 2: isolate node-0 from network");

    // Make node-0 unreachable to simulate network partition
    router.fail_node(0);

    // Extract node-0's storage for later restoration
    let (n0, ls0, sm0) = router.remove_node(0).unwrap();
    n0.shutdown().await?;

    tracing::info!("--- section 3: elect node-1 as new leader");

    // Wait for leader lease to expire on node-2
    // Otherwise node-2 will reject vote requests while it still has
    // an active lease for the old leader (node-0)
    tokio::time::sleep(Duration::from_millis(1_000)).await;

    // Manually trigger election on node-1
    {
        let n1 = router.get_raft_handle(1)?;
        n1.trigger().elect().await?;
    }

    // Wait for node-1 to become leader
    router.wait(1, timeout()).state(ServerState::Leader, "node-1 becomes new leader").await?;

    // Verify node-2 recognizes node-1 as leader
    router.wait(2, timeout()).current_leader(NodeId(1), "node-2 sees node-1 as leader").await?;

    tracing::info!("--- section 4: write entries while node-0 is partitioned");

    // Write multiple entries to the new leader
    // Account for blank leader log when node-1 was elected
    let mut new_log_index = log_index + 1;
    for i in 0..10 {
        router
            .write(1, format!("key{}", i), format!("value{}", i))
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        new_log_index += 1;
    }

    // Verify entries are replicated to node-2
    router.wait(2, timeout()).applied_index(Some(new_log_index), "node-2 replicated entries").await?;

    tracing::info!("--- section 5: restore node-0 and verify catch-up");

    // Restore node-0 with its original storage
    router.new_raft_node_with_storage(0, ls0, sm0).await?;

    // Restore network connectivity
    router.recover_node(0);

    // Wait for node-0 to catch up via replication
    router
        .wait(0, timeout())
        .applied_index(Some(new_log_index), "node-0 catches up via replication")
        .await?;

    // Verify node-0 recognizes node-1 as leader
    router.wait(0, timeout()).current_leader(NodeId(1), "node-0 sees node-1 as leader").await?;

    tracing::info!("--- section 6: verify data consistency");

    // Verify all nodes have the same data
    for i in 0..10 {
        let key = format!("key{}", i);
        let expected_value = format!("value{}", i);

        for node_id in [NodeId(0), NodeId(1), NodeId(2)] {
            let val = router.read(node_id, &key).await;
            assert_eq!(val, Some(expected_value.clone()), "node {} should have correct value for {}", node_id, key);
        }
    }

    // Verify all nodes have same applied index
    for node_id in [NodeId(0), NodeId(1), NodeId(2)] {
        router
            .wait(node_id, timeout())
            .applied_index(
                Some(new_log_index),
                &format!("node {} should have applied index {}", node_id, new_log_index),
            )
            .await?;
    }

    Ok(())
}
