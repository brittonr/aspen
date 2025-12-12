/// Test follower recovery after complete state loss on restart
///
/// This test validates that a follower can recover when all its state
/// (logs and state machine) is lost during a restart. The leader should
/// detect the state loss and help the follower recover through log reversion.
///
/// Original: openraft/tests/tests/replication/t62_follower_clear_restart_recover.rs
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen::testing::AspenRouter;
use openraft::{Config, ServerState};

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test follower recovery from complete state loss
///
/// 1. Create 3-node cluster
/// 2. Write some data
/// 3. Stop node-1 and clear all its state
/// 4. Restart node-1 with empty storage
/// 5. Leader detects state loss via heartbeat
/// 6. Verify node-1 recovers all data
#[tokio::test]
async fn test_follower_clear_restart_recover() -> Result<()> {
    // Configure with log reversion allowed for recovery
    let config = Arc::new(
        Config {
            enable_heartbeat: false,         // Manual heartbeat control
            enable_elect: false,             // No auto elections
            allow_log_reversion: Some(true), // Critical: allow log recovery after state loss
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: setup 3-node cluster");

    // Create cluster with nodes {0, 1, 2}
    let mut log_index = router
        .new_cluster(
            BTreeSet::from([NodeId(0), NodeId(1), NodeId(2)]),
            BTreeSet::new(),
        )
        .await?;

    // Verify node-0 is leader
    router
        .wait(0, timeout())
        .state(ServerState::Leader, "node-0 is leader")
        .await?;

    tracing::info!("--- section 2: write initial data");

    // Write test data
    for i in 0..10 {
        router
            .write(0, format!("key{}", i), format!("value{}", i))
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        log_index += 1;
    }

    // Wait for replication to all nodes
    for node_id in [NodeId(0), NodeId(1), NodeId(2)] {
        router
            .wait(node_id, timeout())
            .applied_index(
                Some(log_index),
                &format!("node-{} has initial data", node_id),
            )
            .await?;
    }

    tracing::info!("--- section 3: simulate node-1 state loss");

    // Extract and shutdown node-1
    let (n1, _log1, _sm1) = router.remove_node(1).unwrap();
    n1.shutdown().await?;

    // Restart node-1 with completely empty state (simulating data loss)
    router.new_raft_node(1).await?;

    // At this point, node-1 has:
    // - Empty log
    // - Empty state machine
    // - No vote information
    // - No membership info

    tracing::info!("--- section 4: trigger leader heartbeat to detect state loss");

    // Manually trigger heartbeat from leader to detect follower's state loss
    {
        let n0 = router.get_raft_handle(0)?;
        n0.trigger().heartbeat().await?;
    }

    tracing::info!("--- section 5: wait for node-1 recovery");

    // The leader should detect that node-1 has lost state and needs recovery
    // With allow_log_reversion enabled, the leader will help node-1 rebuild its log

    // Wait for node-1 to recover its log
    router
        .wait(1, timeout())
        .log_index(Some(log_index), "node-1 log recovered")
        .await?;

    // Wait for node-1 to apply recovered entries
    router
        .wait(1, timeout())
        .applied_index(Some(log_index), "node-1 state machine recovered")
        .await?;

    tracing::info!("--- section 6: verify complete recovery");

    // Verify node-1 has recovered all data
    for i in 0..10 {
        let key = format!("key{}", i);
        let val = router.read(1, &key).await;
        assert_eq!(
            val,
            Some(format!("value{}", i)),
            "node-1 should have recovered data for {}",
            key
        );
    }

    // Verify node-1's metrics match other nodes
    // Note: applied_index is already validated via Wait API above (lines 109-112)
    let n1 = router.get_raft_handle(1)?;
    let metrics1 = n1.metrics().borrow().clone();

    let n0 = router.get_raft_handle(0)?;
    let metrics0 = n0.metrics().borrow().clone();

    assert_eq!(
        metrics1.last_log_index, metrics0.last_log_index,
        "node-1 log index should match leader"
    );

    tracing::info!("--- section 7: test continued operation after recovery");

    // Write more data to ensure cluster continues functioning
    router
        .write(
            0,
            "post_recovery_key".to_string(),
            "post_recovery_value".to_string(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
    log_index += 1;

    // Verify replication works normally after recovery
    for node_id in [NodeId(0), NodeId(1), NodeId(2)] {
        router
            .wait(node_id, timeout())
            .applied_index(
                Some(log_index),
                &format!("node-{} applied post-recovery write", node_id),
            )
            .await?;

        let val = router.read(node_id, "post_recovery_key").await;
        assert_eq!(
            val,
            Some("post_recovery_value".to_string()),
            "node-{} should have post-recovery data",
            node_id
        );
    }

    Ok(())
}
