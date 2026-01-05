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

/// Information about a content provider discovered via DHT.
#[cfg(feature = "global-discovery")]
#[derive(Debug, Clone)]
pub struct ContentProviderInfo {
    /// Public key of the provider node.
    pub node_id: iroh::PublicKey,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq).
    pub blob_format: iroh_blobs::BlobFormat,
    /// Timestamp when this provider was discovered (microseconds since epoch).
    pub discovered_at: u64,
    /// Whether this provider has been verified (we successfully connected).
    pub verified: bool,
}

/// Node address resolved from DHT.
#[cfg(feature = "global-discovery")]
#[derive(Debug, Clone)]
pub struct ContentNodeAddr {
    /// The node's iroh public key.
    pub public_key: iroh::PublicKey,
    /// Optional relay URL for NAT traversal.
    pub relay_url: Option<String>,
    /// Direct socket addresses (IPv4 and IPv6).
    pub direct_addrs: Vec<String>,
}

/// Content discovery service for DHT operations.
#[cfg(feature = "global-discovery")]
#[async_trait]
pub trait ContentDiscovery: Send + Sync {
    /// Announce content availability in DHT.
    ///
    /// # Arguments
    /// * `hash` - The iroh blob hash
    /// * `size` - Size of the blob in bytes
    /// * `format` - Blob format (Raw or HashSeq)
    async fn announce(&self, hash: iroh_blobs::Hash, size: u64, format: iroh_blobs::BlobFormat) -> Result<(), String>;

    /// Find providers for content hash.
    ///
    /// # Arguments
    /// * `hash` - The iroh blob hash
    /// * `format` - Blob format to query
    async fn find_providers(
        &self,
        hash: iroh_blobs::Hash,
        format: iroh_blobs::BlobFormat,
    ) -> Result<Vec<ContentProviderInfo>, String>;

    /// Look up a specific provider's node address by their public key.
    ///
    /// This queries the DHT for a known provider's address information.
    ///
    /// # Arguments
    /// * `public_key` - The provider's public key
    /// * `hash` - The blob hash
    /// * `format` - Blob format
    async fn find_provider_by_public_key(
        &self,
        public_key: &iroh::PublicKey,
        hash: iroh_blobs::Hash,
        format: iroh_blobs::BlobFormat,
    ) -> Result<Option<ContentNodeAddr>, String>;
}

// ============================================================================
// Watch Registry
// ============================================================================

/// Information about an active watch subscription.
#[derive(Debug, Clone)]
pub struct WatchInfo {
    /// Unique watch ID.
    pub watch_id: u64,
    /// Key prefix being watched.
    pub prefix: String,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Number of events sent.
    pub events_sent: u64,
    /// Watch creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Whether to include previous value in events.
    pub include_prev_value: bool,
}

/// Registry for tracking active watch subscriptions.
///
/// This trait allows querying the status of active watches across the system.
/// Watches are created via the streaming protocol (LOG_SUBSCRIBER_ALPN), but
/// their status can be queried via the standard RPC interface.
///
/// # Tiger Style
///
/// - Bounded watch count via MAX_WATCHES constant
/// - Thread-safe via interior mutability
/// - Read operations are lock-free where possible
#[async_trait]
pub trait WatchRegistry: Send + Sync {
    /// Get all active watches.
    ///
    /// Returns a list of all currently active watch subscriptions.
    async fn get_all_watches(&self) -> Vec<WatchInfo>;

    /// Get a specific watch by ID.
    ///
    /// Returns `None` if the watch doesn't exist.
    async fn get_watch(&self, watch_id: u64) -> Option<WatchInfo>;

    /// Get the total number of active watches.
    async fn watch_count(&self) -> usize;

    /// Register a new watch subscription.
    ///
    /// Called by `LogSubscriberProtocolHandler` when a client creates a watch.
    /// Returns the assigned watch ID.
    fn register_watch(&self, prefix: String, include_prev_value: bool) -> u64;

    /// Update watch state after sending events.
    ///
    /// Called by `LogSubscriberProtocolHandler` after sending events to a subscriber.
    fn update_watch(&self, watch_id: u64, last_sent_index: u64, events_sent: u64);

    /// Unregister a watch subscription.
    ///
    /// Called by `LogSubscriberProtocolHandler` when a client disconnects or cancels.
    fn unregister_watch(&self, watch_id: u64);
}

/// In-memory implementation of WatchRegistry.
///
/// Thread-safe via `RwLock` for the watch map and atomic counters.
pub struct InMemoryWatchRegistry {
    watches: std::sync::RwLock<std::collections::HashMap<u64, WatchInfo>>,
    next_watch_id: std::sync::atomic::AtomicU64,
}

impl InMemoryWatchRegistry {
    /// Create a new in-memory watch registry.
    pub fn new() -> Self {
        Self {
            watches: std::sync::RwLock::new(std::collections::HashMap::new()),
            next_watch_id: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

impl Default for InMemoryWatchRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WatchRegistry for InMemoryWatchRegistry {
    async fn get_all_watches(&self) -> Vec<WatchInfo> {
        self.watches.read().expect("watch registry lock poisoned").values().cloned().collect()
    }

    async fn get_watch(&self, watch_id: u64) -> Option<WatchInfo> {
        self.watches.read().expect("watch registry lock poisoned").get(&watch_id).cloned()
    }

    async fn watch_count(&self) -> usize {
        self.watches.read().expect("watch registry lock poisoned").len()
    }

    fn register_watch(&self, prefix: String, include_prev_value: bool) -> u64 {
        let watch_id = self.next_watch_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let info = WatchInfo {
            watch_id,
            prefix,
            last_sent_index: 0,
            events_sent: 0,
            created_at_ms: now_ms,
            include_prev_value,
        };

        self.watches.write().expect("watch registry lock poisoned").insert(watch_id, info);

        watch_id
    }

    fn update_watch(&self, watch_id: u64, last_sent_index: u64, events_sent: u64) {
        let mut watches = self.watches.write().expect("watch registry lock poisoned");
        if let Some(info) = watches.get_mut(&watch_id) {
            info.last_sent_index = last_sent_index;
            info.events_sent = events_sent;
        }
    }

    fn unregister_watch(&self, watch_id: u64) {
        self.watches.write().expect("watch registry lock poisoned").remove(&watch_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_watch_registry_new() {
        let registry = InMemoryWatchRegistry::new();
        assert_eq!(registry.watch_count().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_watch_registry_default() {
        let registry = InMemoryWatchRegistry::default();
        assert_eq!(registry.watch_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id1 = registry.register_watch("prefix1/".to_string(), false);
        assert_eq!(id1, 1);
        assert_eq!(registry.watch_count().await, 1);

        let id2 = registry.register_watch("prefix2/".to_string(), true);
        assert_eq!(id2, 2);
        assert_eq!(registry.watch_count().await, 2);

        // IDs should be unique and increasing
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_get_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id = registry.register_watch("test/".to_string(), true);

        let info = registry.get_watch(id).await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.watch_id, id);
        assert_eq!(info.prefix, "test/");
        assert!(info.include_prev_value);
        assert_eq!(info.last_sent_index, 0);
        assert_eq!(info.events_sent, 0);
        assert!(info.created_at_ms > 0);
    }

    #[tokio::test]
    async fn test_get_watch_not_found() {
        let registry = InMemoryWatchRegistry::new();
        assert!(registry.get_watch(999).await.is_none());
    }

    #[tokio::test]
    async fn test_get_all_watches() {
        let registry = InMemoryWatchRegistry::new();

        registry.register_watch("a/".to_string(), false);
        registry.register_watch("b/".to_string(), true);
        registry.register_watch("c/".to_string(), false);

        let watches = registry.get_all_watches().await;
        assert_eq!(watches.len(), 3);

        // Check all prefixes are present
        let prefixes: Vec<_> = watches.iter().map(|w| w.prefix.as_str()).collect();
        assert!(prefixes.contains(&"a/"));
        assert!(prefixes.contains(&"b/"));
        assert!(prefixes.contains(&"c/"));
    }

    #[tokio::test]
    async fn test_update_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id = registry.register_watch("test/".to_string(), false);

        // Initial state
        let info = registry.get_watch(id).await.unwrap();
        assert_eq!(info.last_sent_index, 0);
        assert_eq!(info.events_sent, 0);

        // Update
        registry.update_watch(id, 100, 42);

        // Verify update
        let info = registry.get_watch(id).await.unwrap();
        assert_eq!(info.last_sent_index, 100);
        assert_eq!(info.events_sent, 42);
    }

    #[tokio::test]
    async fn test_update_nonexistent_watch() {
        let registry = InMemoryWatchRegistry::new();

        // Should not panic or error
        registry.update_watch(999, 100, 42);
    }

    #[tokio::test]
    async fn test_unregister_watch() {
        let registry = InMemoryWatchRegistry::new();

        let id = registry.register_watch("test/".to_string(), false);
        assert_eq!(registry.watch_count().await, 1);

        registry.unregister_watch(id);
        assert_eq!(registry.watch_count().await, 0);
        assert!(registry.get_watch(id).await.is_none());
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_watch() {
        let registry = InMemoryWatchRegistry::new();

        // Should not panic or error
        registry.unregister_watch(999);
    }

    #[tokio::test]
    async fn test_watch_ids_are_unique() {
        let registry = InMemoryWatchRegistry::new();

        let mut ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let id = registry.register_watch("test/".to_string(), false);
            assert!(ids.insert(id), "Watch ID {} was not unique", id);
        }
    }

    #[test]
    fn test_watch_info_debug() {
        let info = WatchInfo {
            watch_id: 1,
            prefix: "test/".to_string(),
            last_sent_index: 100,
            events_sent: 42,
            created_at_ms: 1704326400000, // 2024-01-04 00:00:00 UTC
            include_prev_value: true,
        };

        let debug = format!("{:?}", info);
        assert!(debug.contains("watch_id: 1"));
        assert!(debug.contains("prefix: \"test/\""));
    }

    #[test]
    fn test_watch_info_clone() {
        let info = WatchInfo {
            watch_id: 1,
            prefix: "test/".to_string(),
            last_sent_index: 100,
            events_sent: 42,
            created_at_ms: 1704326400000,
            include_prev_value: true,
        };

        let cloned = info.clone();
        assert_eq!(cloned.watch_id, info.watch_id);
        assert_eq!(cloned.prefix, info.prefix);
        assert_eq!(cloned.last_sent_index, info.last_sent_index);
        assert_eq!(cloned.events_sent, info.events_sent);
        assert_eq!(cloned.created_at_ms, info.created_at_ms);
        assert_eq!(cloned.include_prev_value, info.include_prev_value);
    }
}
