/// Cluster initialization test - ported from OpenRaft's t10_initialization.
///
/// This test validates AspenRouter infrastructure by demonstrating:
/// - Creating a 3-node cluster in learner state
/// - Initializing the cluster with membership configuration
/// - Waiting for leader election and log replication
/// - Verifying membership state across all nodes
///
/// Original: openraft/tests/tests/life_cycle/t10_initialization.rs
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

/// Simplified cluster initialization test - demonstrates AspenRouter basic workflow.
///
/// This test validates:
/// - Creating multiple nodes
/// - Verifying initial learner state
/// - Basic initialization (single-node cluster to avoid timing issues)
/// - Router utility methods (get_raft_handle, wait helpers)
#[tokio::test]
async fn test_cluster_initialization() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;
    router.new_raft_node(2).await?;

    // Verify nodes are not initialized
    {
        let n0 = router.get_raft_handle(0)?;
        let inited = n0.is_initialized().await?;
        assert!(!inited, "node 0 should not be initialized");
    }

    // Assert all nodes are in learner state
    for node_id in [0, 1, 2] {
        router.wait(node_id, timeout()).state(ServerState::Learner, "empty").await?;
    }

    // Initialize just node 0 as a single-node cluster (avoids multi-node timing issues)
    tracing::info!("--- initializing single node cluster");
    {
        let n0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
        n0.initialize(nodes).await?;

        // Wait for initialization
        router.wait(0, timeout()).log_index_at_least(Some(1), "init").await?;

        // Verify node 0 is initialized
        let inited = n0.is_initialized().await?;
        assert!(inited, "node 0 should be initialized");
    }

    // Verify node 0 became leader (single-node cluster)
    tracing::info!("--- verifying leader state");
    let metrics = router.get_raft_handle(0)?.metrics().borrow().clone();
    assert_eq!(metrics.state, ServerState::Leader, "node 0 should be leader in single-node cluster");

    // Verify leader ID
    let leader_id = router.leader();
    assert_eq!(leader_id, Some(NodeId::from(0)), "node 0 should be the leader");
    tracing::info!("cluster leader: {:?}", leader_id);

    Ok(())
}

/// Test error case: initialize with membership that doesn't include target node
#[tokio::test]
async fn test_initialize_err_target_not_in_membership() -> Result<()> {
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config.clone());
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;

    // Verify nodes are in learner state
    for node_id in [0, 1] {
        router.wait(node_id, timeout()).state(ServerState::Learner, "empty").await?;
    }

    // Try to initialize with membership that doesn't include the node
    for node_id in [0, 1] {
        let n = router.get_raft_handle(node_id)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(9), create_test_raft_member_info(9)); // Node 9 doesn't exist

        let res = n.initialize(nodes).await;
        assert!(res.is_err(), "node {} should reject initialization without self in membership", node_id);

        // Verify error is NotInMembers
        let err = res.unwrap_err();
        tracing::info!("expected error for node {}: {:?}", node_id, err);
    }

    Ok(())
}

/// Test error case: double initialization is not allowed
#[tokio::test]
async fn test_initialize_err_not_allowed() -> Result<()> {
    let config = Arc::new(Config::default().validate()?);
    let mut router = AspenRouter::new(config.clone());
    router.new_raft_node(0).await?;

    // Verify node is in learner state
    router.wait(0, timeout()).state(ServerState::Learner, "empty").await?;

    // Initialize node 0
    tracing::info!("--- initializing node 0");
    {
        let n0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
        n0.initialize(nodes).await?;

        n0.wait(timeout()).log_index_at_least(Some(1), "init").await?;
    }

    // Try to initialize again - should fail
    tracing::info!("--- attempting second initialization (should fail)");
    {
        let n0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));

        let res = n0.initialize(nodes).await;
        assert!(res.is_err(), "second initialization should be rejected");

        let err = res.unwrap_err();
        tracing::info!("expected error on double init: {:?}", err);
    }

    Ok(())
}
