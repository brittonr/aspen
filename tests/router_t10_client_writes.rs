/// Client write/read tests - simplified from OpenRaft's client_api tests.
///
/// Validates basic client write and read operations against a Raft cluster.
///
/// Original: openraft/tests/tests/client_api/t10_client_writes.rs
/// Note: Simplified to single-node cluster due to add_learner limitations
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen::testing::AspenRouter;
use aspen::testing::create_test_raft_member_info;
use openraft::Config;
use openraft::ServerState;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test basic client write operations.
///
/// Validates that:
/// - Writes are applied to the state machine
/// - Log index increases with each write
/// - Data persists across reads
#[tokio::test]
async fn test_client_writes() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());
    router.new_raft_node(0).await?;

    tracing::info!("--- initialize single-node cluster");
    {
        let node0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
        node0.initialize(nodes).await?;

        router.wait(0, timeout()).log_index_at_least(Some(1), "initialized").await?;

        router.wait(0, timeout()).state(ServerState::Leader, "node 0 is leader").await?;
    }

    tracing::info!("--- write multiple entries");
    {
        for i in 0..10 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            router.write(0, key, value).await.map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        }

        // Wait for all writes to be applied
        router.wait(0, timeout()).log_index_at_least(Some(11), "10 writes applied").await?;
    }

    tracing::info!("--- verify all data persisted");
    {
        for i in 0..10 {
            let key = format!("key{}", i);
            let expected = format!("value{}", i);
            let val = router.read(0, &key).await;
            assert_eq!(val, Some(expected), "key{} should be persisted", i);
        }
    }

    tracing::info!("--- overwrite existing keys");
    {
        router
            .write(0, "key0".to_string(), "updated0".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;
        router
            .write(0, "key5".to_string(), "updated5".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;

        router.wait(0, timeout()).log_index_at_least(Some(13), "2 updates applied").await?;
    }

    tracing::info!("--- verify updates took effect");
    {
        let val0 = router.read(0, "key0").await;
        assert_eq!(val0, Some("updated0".to_string()));

        let val5 = router.read(0, "key5").await;
        assert_eq!(val5, Some("updated5".to_string()));

        // Other keys should remain unchanged
        let val1 = router.read(0, "key1").await;
        assert_eq!(val1, Some("value1".to_string()));
    }

    Ok(())
}

/// Test writing to non-leader (should fail or forward).
///
/// In a single-node cluster, node 0 is always the leader.
/// This test verifies the write path works correctly.
#[tokio::test]
async fn test_leader_write_path() -> Result<()> {
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

    tracing::info!("--- verify node 0 is leader");
    {
        let leader_id = router.leader();
        assert_eq!(leader_id, Some(NodeId::from(0)), "node 0 should be leader");
    }

    tracing::info!("--- write via leader");
    {
        router
            .write(0, "test".to_string(), "data".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;

        let val = router.read(0, "test").await;
        assert_eq!(val, Some("data".to_string()));
    }

    Ok(())
}

/// Test sequential writes with increasing log indices.
///
/// Validates that log indices increase monotonically with each write.
#[tokio::test]
async fn test_sequential_log_indices() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

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

    tracing::info!("--- write and verify log indices increase");
    {
        let mut expected_log_index = 1; // Start after initialization

        for i in 0..5 {
            expected_log_index += 1;

            router
                .write(0, format!("k{}", i), format!("v{}", i))
                .await
                .map_err(|e| anyhow::anyhow!("write failed: {}", e))?;

            router
                .wait(0, timeout())
                .log_index_at_least(Some(expected_log_index), &format!("write {} applied", i))
                .await?;

            let metrics = router.get_raft_handle(0)?.metrics().borrow().clone();
            assert_eq!(metrics.last_log_index, Some(expected_log_index), "log index should be {}", expected_log_index);
        }
    }

    Ok(())
}
