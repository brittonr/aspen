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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PeerConnectionState Tests
    // ========================================================================

    #[test]
    fn peer_connection_state_default() {
        let state: PeerConnectionState = Default::default();
        assert_eq!(state, PeerConnectionState::Disconnected);
    }

    #[test]
    fn peer_connection_state_equality() {
        assert_eq!(PeerConnectionState::Disconnected, PeerConnectionState::Disconnected);
        assert_eq!(PeerConnectionState::Connecting, PeerConnectionState::Connecting);
        assert_eq!(PeerConnectionState::Connected, PeerConnectionState::Connected);
        assert_eq!(PeerConnectionState::Failed, PeerConnectionState::Failed);
    }

    #[test]
    fn peer_connection_state_inequality() {
        assert_ne!(PeerConnectionState::Disconnected, PeerConnectionState::Connected);
        assert_ne!(PeerConnectionState::Connecting, PeerConnectionState::Failed);
    }

    #[test]
    fn peer_connection_state_debug() {
        assert_eq!(format!("{:?}", PeerConnectionState::Disconnected), "Disconnected");
        assert_eq!(format!("{:?}", PeerConnectionState::Connecting), "Connecting");
        assert_eq!(format!("{:?}", PeerConnectionState::Connected), "Connected");
        assert_eq!(format!("{:?}", PeerConnectionState::Failed), "Failed");
    }

    #[test]
    fn peer_connection_state_clone() {
        let state = PeerConnectionState::Connected;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn peer_connection_state_copy() {
        let state = PeerConnectionState::Connecting;
        let copied: PeerConnectionState = state; // Copy, not move
        assert_eq!(state, copied); // Original still valid
    }

    // ========================================================================
    // PeerInfo Tests
    // ========================================================================

    #[test]
    fn peer_info_construction() {
        let info = PeerInfo {
            cluster_id: "cluster-1".to_string(),
            name: "Test Cluster".to_string(),
            state: PeerConnectionState::Connected,
            priority: 10,
            enabled: true,
            sync_count: 100,
            failure_count: 5,
        };
        assert_eq!(info.cluster_id, "cluster-1");
        assert_eq!(info.name, "Test Cluster");
        assert_eq!(info.state, PeerConnectionState::Connected);
        assert_eq!(info.priority, 10);
        assert!(info.enabled);
        assert_eq!(info.sync_count, 100);
        assert_eq!(info.failure_count, 5);
    }

    #[test]
    fn peer_info_debug() {
        let info = PeerInfo {
            cluster_id: "test-id".to_string(),
            name: "Test".to_string(),
            state: PeerConnectionState::Disconnected,
            priority: 1,
            enabled: false,
            sync_count: 0,
            failure_count: 0,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("PeerInfo"));
        assert!(debug.contains("test-id"));
        assert!(debug.contains("Disconnected"));
    }

    #[test]
    fn peer_info_clone() {
        let info = PeerInfo {
            cluster_id: "original".to_string(),
            name: "Original".to_string(),
            state: PeerConnectionState::Failed,
            priority: 5,
            enabled: true,
            sync_count: 50,
            failure_count: 10,
        };
        let cloned = info.clone();
        assert_eq!(info.cluster_id, cloned.cluster_id);
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.state, cloned.state);
        assert_eq!(info.priority, cloned.priority);
        assert_eq!(info.enabled, cloned.enabled);
        assert_eq!(info.sync_count, cloned.sync_count);
        assert_eq!(info.failure_count, cloned.failure_count);
    }

    // ========================================================================
    // SyncStatus Tests
    // ========================================================================

    #[test]
    fn sync_status_construction() {
        let status = SyncStatus {
            cluster_id: "cluster-sync".to_string(),
            state: PeerConnectionState::Connected,
            syncing: true,
            entries_received: 1000,
            entries_imported: 950,
            entries_skipped: 30,
            entries_filtered: 20,
        };
        assert_eq!(status.cluster_id, "cluster-sync");
        assert_eq!(status.state, PeerConnectionState::Connected);
        assert!(status.syncing);
        assert_eq!(status.entries_received, 1000);
        assert_eq!(status.entries_imported, 950);
        assert_eq!(status.entries_skipped, 30);
        assert_eq!(status.entries_filtered, 20);
    }

    #[test]
    fn sync_status_entries_total() {
        let status = SyncStatus {
            cluster_id: "test".to_string(),
            state: PeerConnectionState::Connected,
            syncing: false,
            entries_received: 100,
            entries_imported: 80,
            entries_skipped: 10,
            entries_filtered: 10,
        };
        // Verify: imported + skipped + filtered = received
        assert_eq!(status.entries_imported + status.entries_skipped + status.entries_filtered, status.entries_received);
    }

    #[test]
    fn sync_status_debug() {
        let status = SyncStatus {
            cluster_id: "debug-test".to_string(),
            state: PeerConnectionState::Connecting,
            syncing: true,
            entries_received: 500,
            entries_imported: 400,
            entries_skipped: 50,
            entries_filtered: 50,
        };
        let debug = format!("{:?}", status);
        assert!(debug.contains("SyncStatus"));
        assert!(debug.contains("debug-test"));
        assert!(debug.contains("500"));
    }

    #[test]
    fn sync_status_clone() {
        let status = SyncStatus {
            cluster_id: "clone-test".to_string(),
            state: PeerConnectionState::Failed,
            syncing: false,
            entries_received: 0,
            entries_imported: 0,
            entries_skipped: 0,
            entries_filtered: 0,
        };
        let cloned = status.clone();
        assert_eq!(status.cluster_id, cloned.cluster_id);
        assert_eq!(status.state, cloned.state);
        assert_eq!(status.syncing, cloned.syncing);
    }

    // ========================================================================
    // SubscriptionFilter Tests
    // ========================================================================

    #[test]
    fn subscription_filter_full_replication() {
        let filter = SubscriptionFilter::FullReplication;
        let debug = format!("{:?}", filter);
        assert!(debug.contains("FullReplication"));
    }

    #[test]
    fn subscription_filter_prefix_filter() {
        let prefixes = vec!["users/".to_string(), "data/".to_string()];
        let filter = SubscriptionFilter::PrefixFilter(prefixes.clone());

        if let SubscriptionFilter::PrefixFilter(p) = filter {
            assert_eq!(p, prefixes);
        } else {
            panic!("Expected PrefixFilter variant");
        }
    }

    #[test]
    fn subscription_filter_prefix_exclude() {
        let excludes = vec!["internal/".to_string(), "temp/".to_string()];
        let filter = SubscriptionFilter::PrefixExclude(excludes.clone());

        if let SubscriptionFilter::PrefixExclude(e) = filter {
            assert_eq!(e, excludes);
        } else {
            panic!("Expected PrefixExclude variant");
        }
    }

    #[test]
    fn subscription_filter_empty_prefixes() {
        let filter = SubscriptionFilter::PrefixFilter(vec![]);
        if let SubscriptionFilter::PrefixFilter(p) = filter {
            assert!(p.is_empty());
        } else {
            panic!("Expected PrefixFilter variant");
        }
    }

    #[test]
    fn subscription_filter_debug() {
        let filter = SubscriptionFilter::PrefixFilter(vec!["test/".to_string()]);
        let debug = format!("{:?}", filter);
        assert!(debug.contains("PrefixFilter"));
        assert!(debug.contains("test/"));
    }

    #[test]
    fn subscription_filter_clone() {
        let filter = SubscriptionFilter::PrefixExclude(vec!["exclude/".to_string()]);
        let cloned = filter.clone();

        if let (SubscriptionFilter::PrefixExclude(orig), SubscriptionFilter::PrefixExclude(clone)) = (&filter, &cloned)
        {
            assert_eq!(orig, clone);
        } else {
            panic!("Clone should preserve variant");
        }
    }
}
