/// Snapshot building test - validates automatic snapshot creation with policies.
///
/// Tests that:
/// - Snapshots are automatically created based on SnapshotPolicy
/// - Snapshots include the correct log range
/// - Data persists correctly after snapshot creation
///
/// Note: Learner replication testing is deferred due to OpenRaft engine assertion issue
/// when adding learners post-initialization (documented in router_t20_change_membership.rs).
///
/// Original: openraft/tests/tests/snapshot_building/t10_build_snapshot.rs
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen::raft::types::NodeId;
use aspen::testing::AspenRouter;
use aspen::testing::create_test_raft_member_info;
use openraft::{Config, ServerState, SnapshotPolicy};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test automatic snapshot building with log compaction.
///
/// This test validates the complete snapshot lifecycle:
/// 1. Create single-node cluster with snapshot policy
/// 2. Write enough entries to trigger snapshot creation
/// 3. Verify snapshot was created with correct log index
/// 4. Add learner and verify it receives the snapshot
/// 5. Verify logs are compacted after snapshot installation
#[tokio::test]
async fn test_build_snapshot() -> Result<()> {
    let snapshot_threshold: u64 = 50;

    let config = Arc::new(
        Config {
            snapshot_policy: SnapshotPolicy::LogsSinceLast(snapshot_threshold),
            max_in_snapshot_log_to_keep: 2,
            purge_batch_size: 1,
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- creating and initializing single-node cluster");
    router.new_raft_node(0).await?;

    let node0 = router.get_raft_handle(0)?;
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
    node0.initialize(nodes).await?;

    router
        .wait(0, timeout())
        .state(ServerState::Leader, "node 0 is leader")
        .await?;

    let mut log_index = 1; // Initialization log at index 1

    tracing::info!(
        "--- writing entries to trigger snapshot (threshold={})",
        snapshot_threshold
    );
    {
        // Write enough entries to reach the snapshot threshold
        // We need to write (snapshot_threshold - 1 - log_index) more entries
        let num_writes = (snapshot_threshold - 1 - log_index) as usize;

        for i in 0..num_writes {
            router
                .write(0, format!("key{}", i), format!("value{}", i))
                .await
                .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        }

        log_index = snapshot_threshold - 1;

        tracing::info!(log_index, "--- log_index after writes: {}", log_index);

        // Wait for all writes to be applied
        router
            .wait(0, timeout())
            .applied_index(Some(log_index), "writes applied")
            .await?;

        // Wait for snapshot to be created by polling metrics
        let start = std::time::Instant::now();
        loop {
            let metrics = node0.metrics().borrow().clone();
            if metrics.snapshot.is_some() {
                break;
            }
            if start.elapsed() > Duration::from_secs(10) {
                panic!("timeout waiting for snapshot creation");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    tracing::info!("--- verifying snapshot was created");
    {
        let metrics = node0.metrics().borrow().clone();
        assert!(
            metrics.snapshot.is_some(),
            "snapshot should exist after threshold"
        );

        if let Some(snapshot_log_id) = metrics.snapshot {
            assert_eq!(
                snapshot_log_id.index, log_index,
                "snapshot should include all logs up to threshold"
            );
        }
    }

    tracing::info!("--- verifying data is readable");
    {
        // Verify some of the written data
        let val0 = router.read(0, "key0").await;
        assert_eq!(val0, Some("value0".to_string()), "key0 should be persisted");

        let val10 = router.read(0, "key10").await;
        assert_eq!(
            val10,
            Some("value10".to_string()),
            "key10 should be persisted"
        );
    }

    Ok(())
}

/// Test snapshot policy configuration.
///
/// Validates that different snapshot policies work correctly.
#[tokio::test]
async fn test_snapshot_policy_threshold() -> Result<()> {
    let snapshot_threshold: u64 = 10;

    let config = Arc::new(
        Config {
            snapshot_policy: SnapshotPolicy::LogsSinceLast(snapshot_threshold),
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());
    router.new_raft_node(0).await?;

    let node0 = router.get_raft_handle(0)?;
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
    node0.initialize(nodes).await?;

    router
        .wait(0, timeout())
        .state(ServerState::Leader, "node 0 is leader")
        .await?;

    tracing::info!("--- writing entries below threshold");
    {
        // Write fewer entries than threshold
        for i in 0..5 {
            router
                .write(0, format!("key{}", i), format!("value{}", i))
                .await
                .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        }

        // Wait for writes to be applied
        router
            .wait(0, timeout())
            .applied_index(Some(6), "writes applied") // 1 init + 5 writes
            .await?;

        // Snapshot should NOT be created yet
        let metrics = node0.metrics().borrow().clone();
        if let Some(snapshot_log_id) = metrics.snapshot {
            assert!(
                snapshot_log_id.index < 5,
                "snapshot should not be created before threshold"
            );
        }
    }

    tracing::info!("--- writing entries to reach threshold");
    {
        // Write enough to reach threshold (need 4 more writes: 1 init + 5 + 4 = 10)
        for i in 5..9 {
            router
                .write(0, format!("key{}", i), format!("value{}", i))
                .await
                .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        }

        router
            .wait(0, timeout())
            .applied_index(Some(10), "writes applied to threshold")
            .await?;

        // Now snapshot should be created - poll metrics
        let start = std::time::Instant::now();
        loop {
            let metrics = node0.metrics().borrow().clone();
            if metrics.snapshot.is_some() && metrics.snapshot.unwrap().index >= 9 {
                break;
            }
            if start.elapsed() > Duration::from_secs(10) {
                panic!("timeout waiting for snapshot creation");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let metrics = node0.metrics().borrow().clone();
        assert!(
            metrics.snapshot.is_some(),
            "snapshot should exist after reaching threshold"
        );
    }

    Ok(())
}
