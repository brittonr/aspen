//! Peer synchronization types for cluster-to-cluster sync.
//!
//! Provides traits and types for managing peer cluster connections
//! and subscription filters.

use std::sync::Arc;

use async_trait::async_trait;

use super::docs::AspenDocsTicket;
use super::watch::KeyOrigin;

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
    ///
    /// Returns `None` if the implementation does not support direct importer access
    /// (e.g., stub implementations when peer sync is disabled).
    fn importer(&self) -> Option<&Arc<dyn PeerImporter>>;
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
