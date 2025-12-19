//! PeerManager - Manages connections to peer Aspen clusters.
//!
//! Coordinates peer cluster connections and iroh-docs sync sessions.
//! Works with DocsImporter to handle incoming data and conflict resolution.
//!
//! # Architecture
//!
//! ```text
//! PeerManager
//!     |
//!     +-- Peer A Connection (iroh-docs sync session)
//!     |       |
//!     |       +-- DocsImporter (processes incoming entries)
//!     |
//!     +-- Peer B Connection (iroh-docs sync session)
//!             |
//!             +-- DocsImporter (processes incoming entries)
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::client::ClusterSubscription;
use crate::docs::ticket::AspenDocsTicket;

use super::importer::DocsImporter;

/// Maximum number of peer connections allowed.
/// Tiger Style: Bounded to prevent resource exhaustion.
pub const MAX_PEER_CONNECTIONS: usize = 32;

/// Connection state for a peer cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    /// Time of last successful sync.
    pub last_sync: Option<Instant>,
    /// Number of sync sessions completed.
    pub sync_count: u64,
    /// Number of connection failures.
    pub failure_count: u64,
}

/// Sync status for a peer connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Internal state for a peer connection.
struct PeerConnection {
    /// The ticket used to connect to this peer.
    ticket: AspenDocsTicket,
    /// Current connection state.
    state: PeerConnectionState,
    /// Time of last successful sync.
    last_sync: Option<Instant>,
    /// Number of sync sessions completed.
    sync_count: u64,
    /// Number of connection failures.
    failure_count: u64,
    /// Whether a sync is currently in progress.
    syncing: bool,
}

impl PeerConnection {
    fn new(ticket: AspenDocsTicket) -> Self {
        Self {
            ticket,
            state: PeerConnectionState::Disconnected,
            last_sync: None,
            sync_count: 0,
            failure_count: 0,
            syncing: false,
        }
    }
}

/// Manages connections to peer Aspen clusters.
///
/// Coordinates iroh-docs sync sessions with peer clusters and routes
/// incoming entries through the DocsImporter for conflict resolution.
pub struct PeerManager {
    /// Our cluster's identity.
    #[allow(dead_code)] // Reserved for future peer coordination
    local_cluster_id: String,
    /// DocsImporter for handling incoming entries.
    importer: Arc<DocsImporter>,
    /// Active peer connections.
    peers: RwLock<HashMap<String, PeerConnection>>,
    /// Cancellation token for shutdown.
    cancel: CancellationToken,
}

impl PeerManager {
    /// Create a new PeerManager.
    ///
    /// # Arguments
    /// * `local_cluster_id` - ID of the local cluster
    /// * `importer` - DocsImporter for handling incoming entries
    pub fn new(local_cluster_id: impl Into<String>, importer: Arc<DocsImporter>) -> Self {
        Self {
            local_cluster_id: local_cluster_id.into(),
            importer,
            peers: RwLock::new(HashMap::new()),
            cancel: CancellationToken::new(),
        }
    }

    /// Get the cancellation token for this manager.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Get access to the underlying DocsImporter.
    ///
    /// This is used to update filters, priorities, and enabled state.
    pub fn importer(&self) -> &Arc<DocsImporter> {
        &self.importer
    }

    /// Add a peer cluster to sync with.
    ///
    /// This adds the peer subscription to the DocsImporter and starts
    /// tracking the connection state.
    ///
    /// # Arguments
    /// * `ticket` - The docs ticket for connecting to the peer
    pub async fn add_peer(&self, ticket: AspenDocsTicket) -> Result<()> {
        let cluster_id = &ticket.cluster_id;

        // Check limits
        {
            let peers = self.peers.read().await;
            if peers.len() >= MAX_PEER_CONNECTIONS {
                anyhow::bail!("max peer connections ({}) exceeded", MAX_PEER_CONNECTIONS);
            }
            if peers.contains_key(cluster_id) {
                anyhow::bail!("already connected to peer: {}", cluster_id);
            }
        }

        // Create subscription for the importer
        let subscription = ClusterSubscription::new(
            format!("peer-{}", cluster_id),
            &ticket.cluster_id,
            cluster_id,
        )
        .with_priority(ticket.priority as u32);

        // Add to importer
        self.importer
            .add_subscription(subscription)
            .await
            .context("failed to add peer subscription")?;

        // Track connection
        {
            let mut peers = self.peers.write().await;
            peers.insert(cluster_id.clone(), PeerConnection::new(ticket.clone()));
        }

        info!(cluster_id, priority = ticket.priority, "added peer cluster");

        Ok(())
    }

    /// Remove a peer cluster.
    ///
    /// Disconnects from the peer and removes its subscription.
    pub async fn remove_peer(&self, cluster_id: &str) -> Result<()> {
        // Remove from connection tracking
        {
            let mut peers = self.peers.write().await;
            if peers.remove(cluster_id).is_none() {
                anyhow::bail!("peer not found: {}", cluster_id);
            }
        }

        // Remove from importer
        self.importer
            .remove_subscription(cluster_id)
            .await
            .context("failed to remove peer subscription")?;

        info!(cluster_id, "removed peer cluster");
        Ok(())
    }

    /// List all peer clusters.
    pub async fn list_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        let importer_status = self.importer.list_peers().await;

        peers
            .iter()
            .map(|(id, conn)| {
                let import_status = importer_status.iter().find(|s| &s.cluster_id == id);
                PeerInfo {
                    cluster_id: id.clone(),
                    name: conn.ticket.cluster_id.clone(),
                    state: conn.state,
                    priority: import_status.map(|s| s.priority).unwrap_or(0),
                    enabled: import_status.map(|s| s.enabled).unwrap_or(true),
                    last_sync: conn.last_sync,
                    sync_count: conn.sync_count,
                    failure_count: conn.failure_count,
                }
            })
            .collect()
    }

    /// Get sync status for a specific peer.
    pub async fn sync_status(&self, cluster_id: &str) -> Option<SyncStatus> {
        let peers = self.peers.read().await;
        let conn = peers.get(cluster_id)?;

        let import_status = self.importer.get_peer_status(cluster_id).await;

        Some(SyncStatus {
            cluster_id: cluster_id.to_string(),
            state: conn.state,
            syncing: conn.syncing,
            entries_received: import_status
                .as_ref()
                .map(|s| s.entries_imported + s.entries_skipped + s.entries_filtered)
                .unwrap_or(0),
            entries_imported: import_status
                .as_ref()
                .map(|s| s.entries_imported)
                .unwrap_or(0),
            entries_skipped: import_status
                .as_ref()
                .map(|s| s.entries_skipped)
                .unwrap_or(0),
            entries_filtered: import_status
                .as_ref()
                .map(|s| s.entries_filtered)
                .unwrap_or(0),
        })
    }

    /// Update the connection state for a peer.
    ///
    /// Called internally when connection state changes.
    pub async fn set_connection_state(&self, cluster_id: &str, state: PeerConnectionState) {
        let mut peers = self.peers.write().await;
        if let Some(conn) = peers.get_mut(cluster_id) {
            let old_state = conn.state;
            conn.state = state;

            match state {
                PeerConnectionState::Connected if old_state != PeerConnectionState::Connected => {
                    info!(cluster_id, "peer connected");
                }
                PeerConnectionState::Failed => {
                    conn.failure_count += 1;
                    warn!(
                        cluster_id,
                        failures = conn.failure_count,
                        "peer connection failed"
                    );
                }
                PeerConnectionState::Disconnected
                    if old_state == PeerConnectionState::Connected =>
                {
                    info!(cluster_id, "peer disconnected");
                }
                _ => {}
            }
        }
    }

    /// Mark a sync session as started for a peer.
    pub async fn mark_sync_started(&self, cluster_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(conn) = peers.get_mut(cluster_id) {
            conn.syncing = true;
            debug!(cluster_id, "sync started");
        }
    }

    /// Mark a sync session as completed for a peer.
    pub async fn mark_sync_completed(&self, cluster_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(conn) = peers.get_mut(cluster_id) {
            conn.syncing = false;
            conn.last_sync = Some(Instant::now());
            conn.sync_count += 1;
            debug!(cluster_id, sync_count = conn.sync_count, "sync completed");
        }
    }

    /// Shutdown the peer manager.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;

    fn create_test_manager() -> (Arc<DocsImporter>, PeerManager) {
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let importer = Arc::new(DocsImporter::new("local-cluster", kv_store));
        let manager = PeerManager::new("local-cluster", importer.clone());
        (importer, manager)
    }

    fn create_test_ticket(cluster_id: &str, priority: u8) -> AspenDocsTicket {
        AspenDocsTicket {
            cluster_id: cluster_id.to_string(),
            priority,
            namespace_id: "test-namespace".to_string(),
            peers: vec![],
            read_write: false,
        }
    }

    #[tokio::test]
    async fn test_add_peer() {
        let (_importer, manager) = create_test_manager();

        let ticket = create_test_ticket("cluster-a", 5);
        manager.add_peer(ticket).await.unwrap();

        let peers = manager.list_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].cluster_id, "cluster-a");
        assert_eq!(peers[0].priority, 5);
    }

    #[tokio::test]
    async fn test_duplicate_peer_fails() {
        let (_importer, manager) = create_test_manager();

        let ticket1 = create_test_ticket("cluster-a", 5);
        let ticket2 = create_test_ticket("cluster-a", 10);

        manager.add_peer(ticket1).await.unwrap();
        let result = manager.add_peer(ticket2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let (_importer, manager) = create_test_manager();

        let ticket = create_test_ticket("cluster-a", 5);
        manager.add_peer(ticket).await.unwrap();

        manager.remove_peer("cluster-a").await.unwrap();

        let peers = manager.list_peers().await;
        assert!(peers.is_empty());
    }

    #[tokio::test]
    async fn test_connection_state_tracking() {
        let (_importer, manager) = create_test_manager();

        let ticket = create_test_ticket("cluster-a", 5);
        manager.add_peer(ticket).await.unwrap();

        // Initial state should be disconnected
        let status = manager.sync_status("cluster-a").await.unwrap();
        assert_eq!(status.state, PeerConnectionState::Disconnected);

        // Update to connected
        manager
            .set_connection_state("cluster-a", PeerConnectionState::Connected)
            .await;
        let status = manager.sync_status("cluster-a").await.unwrap();
        assert_eq!(status.state, PeerConnectionState::Connected);

        // Track failure
        manager
            .set_connection_state("cluster-a", PeerConnectionState::Failed)
            .await;
        let peers = manager.list_peers().await;
        assert_eq!(peers[0].failure_count, 1);
    }

    #[tokio::test]
    async fn test_sync_tracking() {
        let (_importer, manager) = create_test_manager();

        let ticket = create_test_ticket("cluster-a", 5);
        manager.add_peer(ticket).await.unwrap();

        // Start sync
        manager.mark_sync_started("cluster-a").await;
        let status = manager.sync_status("cluster-a").await.unwrap();
        assert!(status.syncing);

        // Complete sync
        manager.mark_sync_completed("cluster-a").await;
        let status = manager.sync_status("cluster-a").await.unwrap();
        assert!(!status.syncing);

        let peers = manager.list_peers().await;
        assert_eq!(peers[0].sync_count, 1);
        assert!(peers[0].last_sync.is_some());
    }
}
