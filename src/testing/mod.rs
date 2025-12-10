/// Testing infrastructure for Aspen distributed system tests.
///
/// This module provides deterministic testing primitives for multi-node Raft clusters,
/// enabling fast, reliable tests without real network I/O or timing dependencies.
///
/// ## Key Components
///
/// - `AspenRouter`: Manages multiple in-memory Raft nodes with simulated networking
/// - Wait helpers: Metrics-based assertions via OpenRaft's `Wait` API
/// - Network simulation: Configurable delays, failures, and partitions
/// - `create_test_aspen_node`: Helper for creating test node metadata
///
/// ## Usage Pattern
///
/// ```ignore
/// let config = Arc::new(Config::default().validate()?);
/// let mut router = AspenRouter::new(config);
///
/// router.new_raft_node(0).await;
/// router.new_raft_node(1).await;
/// router.new_raft_node(2).await;
///
/// let node0 = router.get_raft_handle(&0)?;
/// node0.initialize(btreeset! {0,1,2}).await?;
///
/// // Use wait helpers instead of sleep
/// router.wait(&0, timeout()).applied_index(Some(1), "initialized").await?;
/// router.wait(&0, timeout()).current_leader(Some(0), "leader elected").await?;
/// ```
pub mod router;

pub use router::AspenRouter;

use crate::raft::types::{AspenNode, NodeId};

/// Create a test `AspenNode` with a deterministic Iroh address derived from the node ID.
///
/// This is used in tests where we don't have real Iroh endpoints.
/// The address is deterministically generated from the node ID to ensure consistency.
///
/// # Example
///
/// ```ignore
/// use aspen::testing::create_test_aspen_node;
/// use std::collections::BTreeMap;
///
/// let mut nodes = BTreeMap::new();
/// nodes.insert(0, create_test_aspen_node(0));
/// nodes.insert(1, create_test_aspen_node(1));
/// raft.initialize(nodes).await?;
/// ```
pub fn create_test_aspen_node(node_id: NodeId) -> AspenNode {
    use iroh::{EndpointAddr, EndpointId, SecretKey};

    // Generate a deterministic secret key from the node ID
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&node_id.to_le_bytes());
    let secret_key = SecretKey::from(seed);
    let endpoint_id: EndpointId = secret_key.public();

    // Create an EndpointAddr with just the ID (no relay URLs or direct addresses for tests)
    let endpoint_addr = EndpointAddr::new(endpoint_id);

    AspenNode::new(endpoint_addr)
}
