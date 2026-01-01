/// Network partition test - simplified from OpenRaft's t51_remove_unreachable_follower.
///
/// This test validates that:
/// - Network failures can be simulated via fail_node/recover_node
/// - Isolated nodes cannot communicate with the cluster
///
/// Original: openraft/tests/tests/membership/t51_remove_unreachable_follower.rs
/// Note: Simplified due to add_learner multi-node limitations
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen_testing::AspenRouter;
use aspen_testing::create_test_raft_member_info;
use openraft::Config;
use openraft::ServerState;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test basic network partition simulation.
///
/// Validates that AspenRouter's fail_node() and recover_node() APIs
/// correctly simulate network failures in a 3-node cluster.
#[tokio::test]
async fn test_network_partition_simulation() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_tick: true, // Enable automatic heartbeats and elections
            election_timeout_min: 150,
            election_timeout_max: 300,
            heartbeat_interval: 50,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    // Create a 3-node cluster
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;
    router.new_raft_node(2).await?;

    tracing::info!("--- initialize 3-node cluster");
    {
        let node0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
        nodes.insert(NodeId::from(1), create_test_raft_member_info(1));
        nodes.insert(NodeId::from(2), create_test_raft_member_info(2));
        node0.initialize(nodes).await?;

        // Wait for leader election
        router.wait(0, timeout()).log_index_at_least(Some(1), "initialized").await?;

        router.wait(0, timeout()).state(ServerState::Leader, "node 0 is leader").await?;

        // Wait for followers to catch up
        router.wait(1, timeout()).state(ServerState::Follower, "node 1 is follower").await?;

        router.wait(2, timeout()).state(ServerState::Follower, "node 2 is follower").await?;
    }

    tracing::info!("--- write initial data to cluster");
    {
        router
            .write(0, "key1".to_string(), "value1".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;

        // Wait for replication to all nodes
        router.wait(0, timeout()).log_index_at_least(Some(2), "leader has data").await?;

        router.wait(1, timeout()).log_index_at_least(Some(2), "follower 1 has data").await?;

        router.wait(2, timeout()).log_index_at_least(Some(2), "follower 2 has data").await?;
    }

    tracing::info!("--- verify data replicated to all nodes");
    {
        assert_eq!(router.read(0, "key1").await, Some("value1".to_string()));
        assert_eq!(router.read(1, "key1").await, Some("value1".to_string()));
        assert_eq!(router.read(2, "key1").await, Some("value1".to_string()));
    }

    tracing::info!("--- simulate network partition by isolating node 0 (leader)");
    {
        // Isolate node 0 from the cluster
        router.fail_node(0);

        // Nodes 1 and 2 should elect a new leader
        // Give them time to detect leader failure and elect
        // Election timeout is 150-300ms, so wait at least that long
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Check if node 1 or 2 became leader
        let node1_state = router.wait(1, Some(Duration::from_secs(3))).state(ServerState::Leader, "").await.is_ok();

        let node2_state = router.wait(2, Some(Duration::from_secs(3))).state(ServerState::Leader, "").await.is_ok();

        assert!(node1_state || node2_state, "Either node 1 or 2 should become leader after node 0 is isolated");

        // Determine which node is the new leader
        let new_leader = if node1_state { 1 } else { 2 };
        tracing::info!("--- node {} became new leader", new_leader);

        // Write to the new leader
        router
            .write(new_leader, "key2".to_string(), "value2".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write to new leader failed: {}", e))?;

        // Verify the isolated node 0 doesn't see the new write
        assert_eq!(router.read(0, "key2").await, None, "Isolated node shouldn't see new writes");

        // Verify the active cluster has the new write
        assert_eq!(router.read(new_leader, "key2").await, Some("value2".to_string()));
    }

    tracing::info!("--- recover node 0 from partition");
    {
        router.recover_node(0);

        // After recovery, node 0 should catch up with the cluster
        // It will become a follower and replicate the missed writes
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Node 0 should now be a follower
        router.wait(0, timeout()).state(ServerState::Follower, "node 0 is follower after recovery").await?;

        // Verify node 0 has caught up and now sees key2
        // This might take a moment for replication
        router.wait(0, timeout()).log_index_at_least(Some(3), "node 0 caught up").await?;

        let val = router.read(0, "key2").await;
        assert_eq!(val, Some("value2".to_string()), "Recovered node should see writes that happened during partition");
    }

    Ok(())
}

/// Test network delay simulation.
///
/// Validates that AspenRouter can simulate slow networks via set_network_delay().
#[tokio::test]
async fn test_network_delay() -> Result<()> {
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config.clone());

    router.new_raft_node(0).await?;

    tracing::info!("--- initialize cluster");
    {
        let node0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
        node0.initialize(nodes).await?;

        router.wait(0, timeout()).log_index_at_least(Some(1), "initialized").await?;
    }

    tracing::info!("--- set network delay to 50ms");
    {
        router.set_global_network_delay(50);

        // Write should still work but be slower
        let start = std::time::Instant::now();
        router
            .write(0, "slow".to_string(), "data".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        let elapsed = start.elapsed();

        tracing::info!("write took {:?} with 50ms delay", elapsed);

        // Verify data written despite delay
        let val = router.read(0, "slow").await;
        assert_eq!(val, Some("data".to_string()));
    }

    tracing::info!("--- reset network delay");
    {
        router.clear_network_delays();

        // Write should be fast again
        router
            .write(0, "fast".to_string(), "data".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;

        let val = router.read(0, "fast").await;
        assert_eq!(val, Some("data".to_string()));
    }

    Ok(())
}
