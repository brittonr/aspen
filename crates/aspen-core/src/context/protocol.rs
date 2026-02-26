//! Core protocol handler traits.
//!
//! These traits define the interfaces for accessing node resources
//! from protocol handlers without depending on concrete implementations.

use async_trait::async_trait;
// Re-export iroh types
pub use iroh::Endpoint as IrohEndpoint;
pub use iroh::EndpointAddr;

/// Provides access to endpoint information for a node.
#[async_trait]
pub trait EndpointProvider: Send + Sync {
    /// Get the node's public key.
    async fn public_key(&self) -> Vec<u8>;

    /// Get the node's peer ID.
    async fn peer_id(&self) -> String;

    /// Get the node's addresses for connectivity.
    async fn addresses(&self) -> Vec<String>;

    /// Get the node address for peer discovery.
    fn node_addr(&self) -> &EndpointAddr;

    /// Get a reference to the underlying Iroh endpoint.
    fn endpoint(&self) -> &IrohEndpoint;
}

/// Provides access to state machine for direct reads.
#[async_trait]
pub trait StateMachineProvider: Send + Sync {
    /// Read a value directly from the state machine.
    async fn direct_read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Check if a key exists in the state machine.
    async fn contains_key(&self, key: &[u8]) -> bool;

    /// Scan keys with a prefix directly from state machine.
    async fn direct_scan(&self, prefix: &[u8], limit: usize) -> Vec<(Vec<u8>, Vec<u8>)>;
}

/// Network factory for dynamic peer registration.
#[async_trait]
pub trait NetworkFactory: Send + Sync {
    /// Add a new peer in the network.
    async fn add_peer(&self, node_id: u64, address: String) -> Result<(), String>;

    /// Remove a peer from the network.
    async fn remove_peer(&self, node_id: u64) -> Result<(), String>;
}

/// Shard topology information.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ShardTopology {
    /// Shard ID this node belongs to.
    pub shard_id: u32,
    /// Total number of shards.
    pub total_shards: u32,
    /// Nodes in this shard.
    pub shard_nodes: Vec<u64>,
    /// Leader node for this shard.
    pub shard_leader: Option<u64>,
    /// Version of the topology configuration.
    pub version: u64,
}

impl ShardTopology {
    /// Get the total number of shards.
    pub fn shard_count(&self) -> u32 {
        self.total_shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ShardTopology Tests
    // ========================================================================

    #[test]
    fn shard_topology_construction() {
        let topo = ShardTopology {
            shard_id: 0,
            total_shards: 4,
            shard_nodes: vec![1, 2, 3],
            shard_leader: Some(1),
            version: 1,
        };
        assert_eq!(topo.shard_id, 0);
        assert_eq!(topo.total_shards, 4);
        assert_eq!(topo.shard_nodes, vec![1, 2, 3]);
        assert_eq!(topo.shard_leader, Some(1));
        assert_eq!(topo.version, 1);
    }

    #[test]
    fn shard_topology_shard_count() {
        let topo = ShardTopology {
            shard_id: 2,
            total_shards: 8,
            shard_nodes: vec![10, 11],
            shard_leader: Some(10),
            version: 5,
        };
        assert_eq!(topo.shard_count(), 8);
    }

    #[test]
    fn shard_topology_no_leader() {
        let topo = ShardTopology {
            shard_id: 1,
            total_shards: 2,
            shard_nodes: vec![5, 6, 7],
            shard_leader: None,
            version: 3,
        };
        assert!(topo.shard_leader.is_none());
    }

    #[test]
    fn shard_topology_empty_nodes() {
        let topo = ShardTopology {
            shard_id: 0,
            total_shards: 1,
            shard_nodes: vec![],
            shard_leader: None,
            version: 0,
        };
        assert!(topo.shard_nodes.is_empty());
    }

    #[test]
    fn shard_topology_debug() {
        let topo = ShardTopology {
            shard_id: 3,
            total_shards: 16,
            shard_nodes: vec![100, 101, 102],
            shard_leader: Some(100),
            version: 42,
        };
        let debug = format!("{:?}", topo);
        assert!(debug.contains("ShardTopology"));
        assert!(debug.contains("shard_id: 3"));
        assert!(debug.contains("16"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn shard_topology_clone() {
        let topo = ShardTopology {
            shard_id: 5,
            total_shards: 10,
            shard_nodes: vec![20, 21, 22, 23],
            shard_leader: Some(21),
            version: 99,
        };
        let cloned = topo.clone();
        assert_eq!(topo.shard_id, cloned.shard_id);
        assert_eq!(topo.total_shards, cloned.total_shards);
        assert_eq!(topo.shard_nodes, cloned.shard_nodes);
        assert_eq!(topo.shard_leader, cloned.shard_leader);
        assert_eq!(topo.version, cloned.version);
    }

    #[test]
    fn shard_topology_serialize() {
        let topo = ShardTopology {
            shard_id: 1,
            total_shards: 4,
            shard_nodes: vec![1, 2],
            shard_leader: Some(1),
            version: 10,
        };
        let json = serde_json::to_string(&topo).expect("should serialize");
        assert!(json.contains("\"shard_id\":1"));
        assert!(json.contains("\"total_shards\":4"));
        assert!(json.contains("\"version\":10"));
    }

    #[test]
    fn shard_topology_single_shard() {
        let topo = ShardTopology {
            shard_id: 0,
            total_shards: 1,
            shard_nodes: vec![1],
            shard_leader: Some(1),
            version: 1,
        };
        assert_eq!(topo.shard_count(), 1);
        assert_eq!(topo.shard_id, 0);
    }

    #[test]
    fn shard_topology_many_shards() {
        let topo = ShardTopology {
            shard_id: 127,
            total_shards: 256,
            shard_nodes: vec![1000, 1001, 1002],
            shard_leader: Some(1001),
            version: u64::MAX,
        };
        assert_eq!(topo.shard_count(), 256);
        assert_eq!(topo.shard_id, 127);
        assert_eq!(topo.version, u64::MAX);
    }

    #[test]
    fn shard_topology_leader_in_nodes() {
        let topo = ShardTopology {
            shard_id: 0,
            total_shards: 2,
            shard_nodes: vec![10, 20, 30],
            shard_leader: Some(20),
            version: 5,
        };
        // Leader should typically be in the nodes list
        assert!(topo.shard_nodes.contains(&topo.shard_leader.unwrap()));
    }
}
