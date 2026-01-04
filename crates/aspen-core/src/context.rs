//! Context traits for protocol handlers.
//!
//! Defines abstract interfaces for accessing node resources without
//! depending on concrete implementations.

use std::sync::Arc;

use async_trait::async_trait;

// Forward declarations for iroh types
extern crate iroh;
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

/// Peer manager for cluster-to-cluster synchronization.
#[async_trait]
pub trait PeerManager: Send + Sync {
    /// Add a peer for synchronization using a docs ticket.
    async fn add_peer(&self, ticket: AspenDocsTicket) -> Result<(), String>;

    /// Remove a peer from synchronization.
    async fn remove_peer(&self, cluster_id: &str) -> Result<(), String>;

    /// List current synchronization peers.
    async fn list_peers(&self) -> Vec<PeerInfo>;

    /// Get sync status for a specific peer.
    async fn sync_status(&self, cluster_id: &str) -> Option<SyncStatus>;

    /// Get access to the underlying importer for advanced operations.
    fn importer(&self) -> &Arc<dyn PeerImporter>;
}

/// Information about a peer cluster connection.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Human-readable name of the peer.
    pub name: String,
    /// Current connection state.
    pub state: PeerConnectionState,
    /// Priority for conflict resolution.
    pub priority: u32,
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Number of sync sessions completed.
    pub sync_count: u64,
    /// Number of connection failures.
    pub failure_count: u64,
}

/// Connection state for a peer cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PeerConnectionState {
    /// Not connected to the peer.
    #[default]
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Connected and syncing.
    Connected,
    /// Connection failed with retriable error.
    Failed,
}

/// Sync status for a peer connection.
#[derive(Debug, Clone)]
pub struct SyncStatus {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Current connection state.
    pub state: PeerConnectionState,
    /// Whether sync is currently in progress.
    pub syncing: bool,
    /// Entries received in current/last sync.
    pub entries_received: u64,
    /// Entries imported in current/last sync.
    pub entries_imported: u64,
    /// Entries skipped due to priority.
    pub entries_skipped: u64,
    /// Entries skipped due to filter.
    pub entries_filtered: u64,
}

/// Docs ticket for connecting to a peer cluster.
#[derive(Debug, Clone)]
pub struct AspenDocsTicket {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Priority for conflict resolution.
    pub priority: u8,
}

/// Peer importer for managing subscriptions and filters.
#[async_trait]
pub trait PeerImporter: Send + Sync {
    /// Update filter for a peer cluster.
    async fn update_filter(&self, cluster_id: &str, filter: SubscriptionFilter) -> Result<(), String>;

    /// Update priority for a peer cluster.
    async fn update_priority(&self, cluster_id: &str, priority: u32) -> Result<(), String>;

    /// Set enabled state for a peer cluster.
    async fn set_enabled(&self, cluster_id: &str, enabled: bool) -> Result<(), String>;

    /// Get key origin information.
    async fn get_key_origin(&self, key: &str) -> Option<KeyOrigin>;
}

/// Subscription filter for peer clusters.
#[derive(Debug, Clone)]
pub enum SubscriptionFilter {
    FullReplication,
    PrefixFilter(Vec<String>),
    PrefixExclude(Vec<String>),
}

/// Key origin information.
#[derive(Debug, Clone)]
pub struct KeyOrigin {
    /// Cluster ID where the key originated.
    pub cluster_id: String,
    /// Priority of the origin cluster.
    pub priority: u32,
    /// Timestamp when the key was created.
    pub timestamp_secs: u64,
}

impl KeyOrigin {
    /// Check if the key originated locally.
    pub fn is_local(&self) -> bool {
        // This would be implemented based on local cluster ID comparison
        false
    }
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

    /// Set an entry in the docs namespace.
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String>;

    /// Get an entry from the docs namespace.
    async fn get_entry(&self, key: &[u8]) -> Result<Option<(Vec<u8>, u64, String)>, String>; // (value, size, hash)

    /// Delete an entry from the docs namespace.
    async fn delete_entry(&self, key: Vec<u8>) -> Result<(), String>;

    /// List entries with optional prefix filter.
    async fn list_entries(&self, prefix: Option<String>, limit: Option<u32>) -> Result<Vec<DocsEntry>, String>;

    /// Get docs namespace status.
    async fn get_status(&self) -> Result<DocsStatus, String>;

    /// Get namespace ID as string.
    fn namespace_id(&self) -> String;

    /// Get author ID as string.
    fn author_id(&self) -> String;
}

/// Entry in docs namespace.
#[derive(Debug, Clone)]
pub struct DocsEntry {
    pub key: String,
    pub size: u64,
    pub hash: String,
}

/// Status of docs namespace.
#[derive(Debug, Clone)]
pub struct DocsStatus {
    pub enabled: bool,
    pub namespace_id: Option<String>,
    pub author_id: Option<String>,
    pub entry_count: Option<u64>,
    pub replica_open: Option<bool>,
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

/// Content discovery service for DHT operations.
#[cfg(feature = "global-discovery")]
#[async_trait]
pub trait ContentDiscovery: Send + Sync {
    /// Announce content availability in DHT.
    async fn announce(&self, hash: &[u8]) -> Result<(), String>;

    /// Find providers for content hash.
    async fn find_providers(&self, hash: &[u8]) -> Result<Vec<String>, String>;
}
