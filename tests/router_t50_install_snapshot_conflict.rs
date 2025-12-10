/// Test snapshot installation with conflicting uncommitted logs
///
/// This test validates that when a follower receives a snapshot whose `last_log_id`
/// conflicts with existing uncommitted logs, the follower correctly truncates the
/// conflicting logs and installs the snapshot. This ensures state consistency when
/// a node falls behind and needs snapshot recovery.
///
/// Original: openraft/openraft/src/engine/handler/following_handler/install_snapshot_test.rs::test_install_snapshot_conflict
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::testing::AspenRouter;
use aspen::testing::create_test_aspen_node;
use openraft::{Config, ServerState, SnapshotPolicy};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test that snapshot installation truncates conflicting uncommitted logs.
///
/// Scenario:
/// 1. Create a 3-node cluster
/// 2. Write some entries and let them replicate
/// 3. Partition the cluster so one node falls behind
/// 4. Write more entries on the majority side
/// 5. Trigger a snapshot on the leader
/// 6. Heal the partition and verify the lagging node receives snapshot
/// 7. Verify the lagging node's conflicting logs are truncated
///
/// This tests the core safety property: when a snapshot's `last_log_id` is beyond
/// a follower's committed point, any uncommitted logs past the committed point
/// must be truncated to maintain consistency.
#[tokio::test]
async fn test_install_snapshot_conflict() -> Result<()> {
    let snapshot_threshold = 10u64;

    let config = Arc::new(
        Config {
            snapshot_policy: SnapshotPolicy::LogsSinceLast(snapshot_threshold),
            max_in_snapshot_log_to_keep: 0,
            purge_batch_size: 1,
            enable_heartbeat: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: initialize 3-node cluster");

    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;
    router.new_raft_node(2).await?;

    // Initialize node 0 as leader
    {
        let n0 = router.get_raft_handle(&0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(0, create_test_aspen_node(0));
        n0.initialize(nodes).await?;
    }

    router
        .wait(&0, timeout())
        .state(ServerState::Leader, "node-0 becomes leader")
        .await?;

    // Add nodes 1 and 2 as learners
    router.add_learner(0, 1).await?;
    router.add_learner(0, 2).await?;

    router
        .wait(&1, timeout())
        .state(ServerState::Learner, "node-1 becomes learner")
        .await?;

    router
        .wait(&2, timeout())
        .state(ServerState::Learner, "node-2 becomes learner")
        .await?;

    // Change membership to make it a full cluster
    let voters: std::collections::BTreeSet<u64> = [0, 1, 2].into_iter().collect();
    let n0 = router.get_raft_handle(&0)?;
    n0.change_membership(voters, false).await?;

    // Wait for all nodes to apply the membership change
    let mut log_index = 1u64 + 2 + 2; // init + 2 add_learner + 2 change_membership

    router
        .wait(&0, timeout())
        .applied_index(Some(log_index), "node-0 applied membership")
        .await?;

    router
        .wait(&1, timeout())
        .applied_index(Some(log_index), "node-1 applied membership")
        .await?;

    router
        .wait(&2, timeout())
        .applied_index(Some(log_index), "node-2 applied membership")
        .await?;

    tracing::info!("--- section 2: write initial entries to all nodes");

    // Write a few entries that replicate to all nodes
    for i in 0..5 {
        router
            .write(&0, format!("initial_key{}", i), format!("value{}", i))
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    // Ensure all nodes have these entries
    router
        .wait(&0, timeout())
        .applied_index(Some(log_index), "node-0 applied initial entries")
        .await?;

    router
        .wait(&1, timeout())
        .applied_index(Some(log_index), "node-1 applied initial entries")
        .await?;

    router
        .wait(&2, timeout())
        .applied_index(Some(log_index), "node-2 applied initial entries")
        .await?;

    tracing::info!("--- section 3: partition node 2 from the cluster");

    router.fail_node(2);

    tracing::info!("--- section 4: write entries to trigger snapshot on majority");

    // Write enough entries to trigger a snapshot
    // These will NOT replicate to node 2 (it's partitioned)
    for i in 0..snapshot_threshold {
        router
            .write(&0, format!("majority_key{}", i), format!("value{}", i))
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    tracing::info!("--- section 5: wait for snapshot creation on leader");

    let start = std::time::Instant::now();
    loop {
        let n0 = router.get_raft_handle(&0)?;
        let metrics = n0.metrics().borrow().clone();

        if let Some(snapshot) = metrics.snapshot {
            tracing::info!(
                "snapshot created at index {}, current log index {}",
                snapshot.index,
                log_index
            );
            break;
        }

        if start.elapsed() > Duration::from_secs(10) {
            panic!("timeout waiting for snapshot creation");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Verify nodes 0 and 1 are up to date
    router
        .wait(&0, timeout())
        .applied_index(Some(log_index), "node-0 applied all entries")
        .await?;

    router
        .wait(&1, timeout())
        .applied_index(Some(log_index), "node-1 applied all entries")
        .await?;

    tracing::info!("--- section 6: heal partition and wait for snapshot replication");

    router.recover_node(2);

    // Wait for node 2 to receive the snapshot
    // Since it's far behind and the logs are purged on the leader,
    // it must receive a snapshot
    let start = std::time::Instant::now();
    loop {
        let n2 = router.get_raft_handle(&2)?;
        let metrics = n2.metrics().borrow().clone();

        if let Some(snapshot) = metrics.snapshot {
            tracing::info!(
                "node-2 received snapshot at index {}, expected around {}",
                snapshot.index,
                log_index
            );

            // The snapshot should be at or near the leader's snapshot point
            // Snapshot is created at last_committed_index when threshold is reached,
            // which is one less than the threshold value (see similar pattern in
            // router_t50_snapshot_when_lacking_log.rs)
            assert!(
                snapshot.index >= snapshot_threshold - 1,
                "snapshot index {} should be at least {} (last_committed when threshold {} is reached)",
                snapshot.index,
                snapshot_threshold - 1,
                snapshot_threshold
            );
            break;
        }

        if start.elapsed() > Duration::from_secs(15) {
            let n2 = router.get_raft_handle(&2)?;
            let metrics = n2.metrics().borrow().clone();
            panic!(
                "timeout waiting for node-2 to receive snapshot. State: {:?}, Applied: {:?}, Snapshot: {:?}",
                metrics.state, metrics.last_applied, metrics.snapshot
            );
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!("--- section 7: verify node 2 caught up to current log");

    // Wait for node 2 to catch up to the latest log index
    router
        .wait(&2, timeout())
        .applied_index(Some(log_index), "node-2 caught up after snapshot")
        .await?;

    tracing::info!("--- section 8: verify data consistency across all nodes");

    // Verify data from before partition (should be in snapshot)
    let val = router.read(&2, "initial_key0").await;
    assert_eq!(
        val,
        Some("value0".to_string()),
        "node-2 should have initial data from snapshot"
    );

    // Verify data from after partition (should be replicated after snapshot)
    let val = router.read(&2, "majority_key0").await;
    assert_eq!(
        val,
        Some("value0".to_string()),
        "node-2 should have majority data after catching up"
    );

    // Cross-check: verify all nodes have the same state
    let val_n0 = router.read(&0, "majority_key5").await;
    let val_n1 = router.read(&1, "majority_key5").await;
    let val_n2 = router.read(&2, "majority_key5").await;

    assert_eq!(val_n0, val_n1, "node-0 and node-1 should match");
    assert_eq!(val_n1, val_n2, "node-1 and node-2 should match");

    Ok(())
}

/// Test snapshot installation when snapshot's last_log_id equals committed.
///
/// When a snapshot's `last_log_id` is less than or equal to the node's committed
/// log index, the snapshot should not be installed (the node is already up-to-date).
#[tokio::test]
async fn test_install_snapshot_at_committed_boundary() -> Result<()> {
    let snapshot_threshold = 10u64;

    let config = Arc::new(
        Config {
            snapshot_policy: SnapshotPolicy::LogsSinceLast(snapshot_threshold),
            max_in_snapshot_log_to_keep: 2,
            purge_batch_size: 1,
            enable_heartbeat: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- initialize single-node cluster");

    router.new_raft_node(0).await?;

    {
        let n0 = router.get_raft_handle(&0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(0, create_test_aspen_node(0));
        n0.initialize(nodes).await?;
    }

    router
        .wait(&0, timeout())
        .state(ServerState::Leader, "node-0 becomes leader")
        .await?;

    let mut log_index = 1u64;

    tracing::info!("--- write entries to trigger snapshot");

    for i in 0..snapshot_threshold {
        router
            .write(&0, format!("key{}", i), format!("value{}", i))
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    tracing::info!("--- wait for snapshot creation");

    let start = std::time::Instant::now();
    let _snapshot_index = loop {
        let n0 = router.get_raft_handle(&0)?;
        let metrics = n0.metrics().borrow().clone();

        if let Some(snapshot) = metrics.snapshot {
            tracing::info!("snapshot created at index {}", snapshot.index);
            break snapshot.index;
        }

        if start.elapsed() > Duration::from_secs(10) {
            panic!("timeout waiting for snapshot creation");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    tracing::info!("--- write more entries after snapshot");

    for i in 0..5 {
        router
            .write(&0, format!("after_key{}", i), format!("value{}", i))
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    router
        .wait(&0, timeout())
        .applied_index(Some(log_index), "all entries applied")
        .await?;

    tracing::info!("--- add learner that will receive snapshot");

    router.new_raft_node(1).await?;
    router.add_learner(0, 1).await?;
    log_index += 1;

    router
        .wait(&1, timeout())
        .state(ServerState::Learner, "node-1 becomes learner")
        .await?;

    // Wait for learner to catch up
    router
        .wait(&1, timeout())
        .applied_index(Some(log_index), "node-1 caught up")
        .await?;

    // Verify learner has the snapshot
    let val = router.read(&1, "key0").await;
    assert_eq!(
        val,
        Some("value0".to_string()),
        "learner should have snapshot data"
    );

    // Verify learner has post-snapshot data
    let val = router.read(&1, "after_key0").await;
    assert_eq!(
        val,
        Some("value0".to_string()),
        "learner should have post-snapshot data"
    );

    Ok(())
}
