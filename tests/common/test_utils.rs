/// Common test utilities and helper functions
///
/// This module provides shared test setup and helper functions to reduce
/// duplication across integration tests.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::cluster::IrohEndpointManager;
use aspen::node::{Node, NodeBuilder};
use aspen::raft::types::NodeId;
use aspen_testing::create_test_raft_member_info;
use iroh::Endpoint;
use tempfile::TempDir;
use tokio::time::timeout;

/// Standard test timeout duration
pub const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Create a temporary directory for test data
pub fn test_temp_dir(test_name: &str) -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Setup a single node with default configuration for testing
pub async fn setup_test_node(node_id: u64) -> Result<(Node, TempDir)> {
    let temp_dir = test_temp_dir(&format!("node_{}", node_id));
    let node = setup_test_node_with_dir(node_id, &temp_dir).await?;
    Ok((node, temp_dir))
}

/// Setup a single node with specified temp directory
pub async fn setup_test_node_with_dir(node_id: u64, temp_dir: &TempDir) -> Result<Node> {
    let node = NodeBuilder::new()
        .node_id(NodeId::from(node_id))
        .data_dir(temp_dir.path())
        .storage_backend("redb")
        .build()
        .await?;

    Ok(node)
}

/// Setup a multi-node cluster for testing
pub async fn setup_test_cluster(node_count: usize) -> Result<Vec<(Node, TempDir)>> {
    let mut nodes = Vec::new();

    for i in 0..node_count {
        let (node, temp_dir) = setup_test_node(i as u64).await?;
        nodes.push((node, temp_dir));
    }

    // Initialize the first node as the cluster leader
    if !nodes.is_empty() {
        let mut members = BTreeMap::new();
        for i in 0..node_count {
            members.insert(NodeId::from(i as u64), create_test_raft_member_info(i as u64));
        }

        nodes[0].0.controller().init(members).await?;
    }

    Ok(nodes)
}

/// Create a test Iroh endpoint with default settings
pub async fn create_test_iroh_endpoint() -> Result<Endpoint> {
    let secret_key = iroh::SecretKey::generate();
    Endpoint::builder()
        .secret_key(secret_key)
        .bind_addr_v4("127.0.0.1:0".parse()?)
        .bind_addr_v6("[::1]:0".parse()?)
        .discovery_n0()
        .build()
        .await
}

/// Create a test IrohEndpointManager
pub async fn create_test_endpoint_manager(node_id: NodeId) -> Result<Arc<IrohEndpointManager>> {
    let secret_key = iroh::SecretKey::generate();
    let manager = IrohEndpointManager::new(
        secret_key,
        node_id,
        None, // No explicit bind addresses
    ).await?;

    Ok(Arc::new(manager))
}

/// Wait for a condition with timeout
pub async fn wait_for<F, Fut>(duration: Duration, mut condition: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    timeout(duration, async {
        loop {
            if condition().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for condition"))?
}

/// Wait for a node to reach a specific state
pub async fn wait_for_leader(node: &Node) -> Result<()> {
    wait_for(TEST_TIMEOUT, || async {
        matches!(
            node.controller().current_state().await,
            Ok(state) if state.server_state == openraft::ServerState::Leader
        )
    })
    .await
}

/// Wait for all nodes in a cluster to have consistent state
pub async fn wait_for_cluster_stability(nodes: &[(Node, TempDir)]) -> Result<()> {
    // Wait for leader election
    let mut leader_found = false;
    for (node, _) in nodes {
        if matches!(
            node.controller().current_state().await,
            Ok(state) if state.server_state == openraft::ServerState::Leader
        ) {
            leader_found = true;
            break;
        }
    }

    if !leader_found {
        return Err(anyhow::anyhow!("No leader found in cluster"));
    }

    // Wait for all nodes to agree on membership
    wait_for(TEST_TIMEOUT, || async {
        let mut memberships = Vec::new();
        for (node, _) in nodes {
            if let Ok(state) = node.controller().current_state().await {
                memberships.push(state.membership_config);
            }
        }

        if memberships.is_empty() {
            return false;
        }

        // Check all memberships are identical
        let first = &memberships[0];
        memberships.iter().all(|m| m == first)
    })
    .await?;

    Ok(())
}

/// Create test data for key-value operations
pub fn test_kv_data(prefix: &str, count: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..count)
        .map(|i| {
            let key = format!("{}_key_{}", prefix, i).into_bytes();
            let value = format!("{}_value_{}", prefix, i).into_bytes();
            (key, value)
        })
        .collect()
}

/// Helper to shutdown a node cleanly
pub async fn shutdown_node(node: Node) -> Result<()> {
    node.shutdown().await
}

/// Helper to shutdown all nodes in a cluster
pub async fn shutdown_cluster(nodes: Vec<(Node, TempDir)>) -> Result<()> {
    for (node, _temp_dir) in nodes {
        shutdown_node(node).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_node_setup() {
        let (node, _temp_dir) = setup_test_node(1).await.unwrap();
        assert_eq!(node.node_id(), NodeId::from(1));
        shutdown_node(node).await.unwrap();
    }

    #[tokio::test]
    async fn test_cluster_setup() {
        let nodes = setup_test_cluster(3).await.unwrap();
        assert_eq!(nodes.len(), 3);

        // Verify node IDs
        for (i, (node, _)) in nodes.iter().enumerate() {
            assert_eq!(node.node_id(), NodeId::from(i as u64));
        }

        shutdown_cluster(nodes).await.unwrap();
    }

    #[tokio::test]
    async fn test_kv_data_generation() {
        let data = test_kv_data("test", 5);
        assert_eq!(data.len(), 5);
        assert_eq!(data[0].0, b"test_key_0");
        assert_eq!(data[0].1, b"test_value_0");
    }
}