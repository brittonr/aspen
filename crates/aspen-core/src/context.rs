//! Context traits for protocol handlers.
//!
//! Defines abstract interfaces for accessing node resources without
//! depending on concrete implementations.

use async_trait::async_trait;
use std::sync::Arc;

/// Provides access to endpoint information for a node.
#[async_trait]
pub trait EndpointProvider: Send + Sync {
    /// Get the node's public key.
    async fn public_key(&self) -> Vec<u8>;

    /// Get the node's peer ID.
    async fn peer_id(&self) -> String;

    /// Get the node's addresses for connectivity.
    async fn addresses(&self) -> Vec<String>;
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
    /// Register a new peer in the network.
    async fn register_peer(&self, node_id: u64, address: String) -> Result<(), String>;

    /// Unregister a peer from the network.
    async fn unregister_peer(&self, node_id: u64) -> Result<(), String>;
}

/// Peer manager for cluster-to-cluster synchronization.
#[async_trait]
pub trait PeerManager: Send + Sync {
    /// Add a peer for synchronization.
    async fn add_peer(&self, peer_id: String, addresses: Vec<String>) -> Result<(), String>;

    /// Remove a peer from synchronization.
    async fn remove_peer(&self, peer_id: String) -> Result<(), String>;

    /// List current synchronization peers.
    async fn list_peers(&self) -> Vec<String>;
}

/// Document synchronization provider.
#[async_trait]
pub trait DocsSyncProvider: Send + Sync {
    /// Join a document for synchronization.
    async fn join_document(&self, doc_id: &[u8]) -> Result<(), String>;

    /// Leave a document synchronization.
    async fn leave_document(&self, doc_id: &[u8]) -> Result<(), String>;

    /// Get document content.
    async fn get_document(&self, doc_id: &[u8]) -> Result<Vec<u8>, String>;
}

/// Shard topology information.
#[derive(Clone, Debug)]
pub struct ShardTopology {
    /// Shard ID this node belongs to.
    pub shard_id: u32,
    /// Total number of shards.
    pub total_shards: u32,
    /// Nodes in this shard.
    pub shard_nodes: Vec<u64>,
    /// Leader node for this shard.
    pub shard_leader: Option<u64>,
}

/// Content discovery service for DHT operations.
#[cfg(feature = "global-discovery")]
#[async_trait]
pub trait ContentDiscovery: Send + Sync {
    /// Announce content availability in DHT.
    async fn announce(&self, hash: &[u8]) -> Result<(), String>;

    /// Find providers for content hash.
    async fn find_providers(&self, hash: &[u8]) -> Result<Vec<String>, String>;
}