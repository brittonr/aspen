/// Test automatic snapshot streaming when follower lacks purged logs
///
/// This test validates that when a follower needs logs that have been
/// purged on the leader, the leader automatically switches to snapshot
/// replication mode to bring the follower up to date.
///
/// Original: openraft/tests/tests/snapshot_streaming/t50_snapshot_when_lacking_log.rs

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::testing::AspenRouter;
use openraft::{BasicNode, Config, ServerState, SnapshotPolicy};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test leader switches to snapshot replication when follower needs purged logs
///
/// 1. Configure aggressive snapshot policy (snapshot every 20 logs)
/// 2. Write 20 entries to trigger snapshot creation
/// 3. Write 11 more entries (total 31)
/// 4. Add a new learner node
/// 5. Verify learner receives snapshot (not individual logs)
#[tokio::test]
async fn test_snapshot_when_lacking_log() -> Result<()> {
    let snapshot_threshold = 20u64;

    // Configure aggressive snapshot and purge policy
    let config = Arc::new(
        Config {
            snapshot_policy: SnapshotPolicy::LogsSinceLast(snapshot_threshold),
            max_in_snapshot_log_to_keep: 0,  // Purge all logs covered by snapshot
            purge_batch_size: 1,              // Purge aggressively
            enable_heartbeat: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: initialize single-node cluster");

    // Create initial node
    router.new_raft_node(0).await?;

    // Initialize as single-node cluster
    {
        let n0 = router.get_raft_handle(&0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(0, BasicNode::default());
        n0.initialize(nodes).await?;
    }

    // Wait for leadership
    router
        .wait(&0, timeout())
        .state(ServerState::Leader, "node-0 becomes leader")
        .await?;

    let mut log_index = 1u64; // Initialization log

    tracing::info!("--- section 2: write entries to trigger snapshot");

    // Write entries to reach snapshot threshold
    for i in 0..(snapshot_threshold - 1) {
        router.write(&0, format!("key{}", i), format!("value{}", i)).await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    // Wait for snapshot to be created
    tracing::info!("--- section 3: wait for snapshot creation at log index {}", log_index);

    let start = std::time::Instant::now();
    loop {
        let n0 = router.get_raft_handle(&0)?;
        let metrics = n0.metrics().borrow().clone();

        if let Some(snapshot) = metrics.snapshot {
            assert_eq!(
                snapshot.index, log_index - 1,
                "snapshot should be at index {} (last committed log)",
                log_index - 1
            );
            tracing::info!("snapshot created at index {}", snapshot.index);
            break;
        }

        if start.elapsed() > Duration::from_secs(10) {
            panic!("timeout waiting for snapshot creation");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!("--- section 4: write more entries after snapshot");

    // Write additional entries beyond snapshot
    for i in 0..11 {
        router.write(&0, format!("key_after_{}", i), format!("value_after_{}", i)).await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    // Verify these entries are applied
    router
        .wait(&0, timeout())
        .applied_index(Some(log_index), "additional entries applied")
        .await?;

    tracing::info!("--- section 5: add learner to receive snapshot");

    // Create and add a new learner
    router.new_raft_node(1).await?;
    router.add_learner(0, 1).await?;
    log_index += 1; // Add learner generates a log entry

    // Wait for learner to become ready
    router
        .wait(&1, timeout())
        .state(ServerState::Learner, "node-1 becomes learner")
        .await?;

    tracing::info!("--- section 6: verify learner received snapshot");

    // Wait for learner to receive and install snapshot
    // Since logs before the snapshot are purged, the learner must receive a snapshot
    let start = std::time::Instant::now();
    loop {
        let n1 = router.get_raft_handle(&1)?;
        let metrics = n1.metrics().borrow().clone();

        if let Some(snapshot) = metrics.snapshot {
            assert_eq!(
                snapshot.index,
                snapshot_threshold - 1,
                "learner should have received snapshot at index {}",
                snapshot_threshold - 1
            );
            tracing::info!("learner received snapshot at index {}", snapshot.index);
            break;
        }

        if start.elapsed() > Duration::from_secs(10) {
            panic!("timeout waiting for learner to receive snapshot");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Verify learner has caught up to latest log
    router
        .wait(&1, timeout())
        .applied_index(Some(log_index), "learner caught up to latest")
        .await?;

    // Verify data integrity - learner should have all data from snapshot + new entries
    let val = router.read(&1, "key0").await;
    assert_eq!(val, Some("value0".to_string()), "data from snapshot should be present");

    let val = router.read(&1, "key_after_0").await;
    assert_eq!(val, Some("value_after_0".to_string()), "data after snapshot should be present");

    Ok(())
}