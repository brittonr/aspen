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
