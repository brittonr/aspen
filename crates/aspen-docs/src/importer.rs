//! DocsImporter - Real-time import of iroh-docs entries to KV.
//!
//! Subscribes to iroh-docs sync events and imports remote entries
//! into the local KV store based on priority ordering.
//!
//! # Architecture
//!
//! The importer listens to sync events from subscribed peer clusters
//! and applies priority-based conflict resolution before writing
//! entries to the local KV store via Raft consensus.
//!
//! ```text
//! Remote Cluster A (iroh-docs namespace)
//!         |
//!         v
//! iroh-docs CRDT sync (range-based set reconciliation)
//!         |
//!         v
//! DocsImporter (receives RemoteInsert events)
//!         |
//!         v
//! Priority conflict check (using KeyOrigin)
//!         |
//!         v
//! Local KV Store (via KeyValueStore trait)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::constants::MAX_DOC_KEY_SIZE;
use super::constants::MAX_DOC_VALUE_SIZE;
use super::origin::KeyOrigin;
use aspen_client::ClusterSubscription;
use aspen_client::SubscriptionFilter;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;

/// Maximum number of peer subscriptions allowed.
/// Tiger Style: Bounded to prevent resource exhaustion.
pub const MAX_PEER_SUBSCRIPTIONS: usize = 32;

/// Imports iroh-docs entries from peer clusters into the local KV store.
///
/// Listens to sync events from subscribed peer clusters and imports
/// entries based on priority ordering. Lower priority numbers win conflicts.
pub struct DocsImporter {
    /// Local cluster ID for tracking origin.
    #[allow(dead_code)] // Reserved for future origin tracking
    local_cluster_id: String,
    /// Local KV store to write imported entries.
    kv_store: Arc<dyn KeyValueStore>,
    /// Active subscriptions to peer clusters.
    subscriptions: RwLock<HashMap<String, PeerSubscription>>,
    /// Cancellation token for shutdown.
    cancel: CancellationToken,
}

/// An active subscription to a peer cluster.
#[derive(Debug, Clone)]
pub struct PeerSubscription {
    /// The subscription configuration.
    pub config: ClusterSubscription,
    /// Whether sync is currently active.
    pub active: bool,
    /// Number of entries imported from this peer.
    pub entries_imported: u64,
    /// Number of entries skipped due to priority.
    pub entries_skipped: u64,
    /// Number of entries skipped due to filter.
    pub entries_filtered: u64,
}

impl PeerSubscription {
    /// Create a new peer subscription from a cluster subscription.
    pub fn new(config: ClusterSubscription) -> Self {
        Self {
            config,
            active: false,
            entries_imported: 0,
            entries_skipped: 0,
            entries_filtered: 0,
        }
    }
}

/// Status information for a peer subscription.
#[derive(Debug, Clone)]
pub struct PeerStatus {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Human-readable name.
    pub name: String,
    /// Priority for conflict resolution.
    pub priority: u32,
    /// Whether the subscription is enabled.
    pub enabled: bool,
    /// Whether sync is currently active.
    pub active: bool,
    /// Total entries imported.
    pub entries_imported: u64,
    /// Entries skipped due to priority.
    pub entries_skipped: u64,
    /// Entries skipped due to filter.
    pub entries_filtered: u64,
    /// Current filter configuration.
    pub filter: SubscriptionFilter,
}

impl DocsImporter {
    /// Create a new DocsImporter.
    ///
    /// # Arguments
    /// * `local_cluster_id` - ID of the local cluster
    /// * `kv_store` - Local KV store for writing imported entries
    pub fn new(local_cluster_id: impl Into<String>, kv_store: Arc<dyn KeyValueStore>) -> Self {
        Self {
            local_cluster_id: local_cluster_id.into(),
            kv_store,
            subscriptions: RwLock::new(HashMap::new()),
            cancel: CancellationToken::new(),
        }
    }

    /// Get the cancellation token for this importer.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Add a subscription to a peer cluster.
    ///
    /// Returns an error if max subscriptions exceeded or cluster already subscribed.
    pub async fn add_subscription(&self, subscription: ClusterSubscription) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        if subs.len() >= MAX_PEER_SUBSCRIPTIONS {
            anyhow::bail!("max peer subscriptions ({}) exceeded", MAX_PEER_SUBSCRIPTIONS);
        }

        if subs.contains_key(&subscription.cluster_id) {
            anyhow::bail!("already subscribed to cluster: {}", subscription.cluster_id);
        }

        info!(
            cluster_id = %subscription.cluster_id,
            name = %subscription.name,
            priority = subscription.priority,
            "adding peer subscription"
        );

        subs.insert(subscription.cluster_id.clone(), PeerSubscription::new(subscription));

        Ok(())
    }

    /// Remove a subscription to a peer cluster.
    pub async fn remove_subscription(&self, cluster_id: &str) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        if subs.remove(cluster_id).is_none() {
            anyhow::bail!("subscription not found: {}", cluster_id);
        }

        info!(cluster_id, "removed peer subscription");
        Ok(())
    }

    /// Update the filter for a peer subscription.
    pub async fn update_filter(&self, cluster_id: &str, filter: SubscriptionFilter) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        let sub = subs.get_mut(cluster_id).context("subscription not found")?;

        sub.config.filter = filter;
        info!(cluster_id, "updated peer filter");
        Ok(())
    }

    /// Update the priority for a peer subscription.
    pub async fn update_priority(&self, cluster_id: &str, priority: u32) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        let sub = subs.get_mut(cluster_id).context("subscription not found")?;

        sub.config.priority = priority;
        info!(cluster_id, priority, "updated peer priority");
        Ok(())
    }

    /// Enable or disable a peer subscription.
    pub async fn set_enabled(&self, cluster_id: &str, enabled: bool) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        let sub = subs.get_mut(cluster_id).context("subscription not found")?;

        sub.config.enabled = enabled;
        info!(cluster_id, enabled, "updated peer enabled status");
        Ok(())
    }

    /// List all peer subscriptions.
    pub async fn list_peers(&self) -> Vec<PeerStatus> {
        let subs = self.subscriptions.read().await;

        subs.values()
            .map(|sub| PeerStatus {
                cluster_id: sub.config.cluster_id.clone(),
                name: sub.config.name.clone(),
                priority: sub.config.priority,
                enabled: sub.config.enabled,
                active: sub.active,
                entries_imported: sub.entries_imported,
                entries_skipped: sub.entries_skipped,
                entries_filtered: sub.entries_filtered,
                filter: sub.config.filter.clone(),
            })
            .collect()
    }

    /// Get status for a specific peer.
    pub async fn get_peer_status(&self, cluster_id: &str) -> Option<PeerStatus> {
        let subs = self.subscriptions.read().await;

        subs.get(cluster_id).map(|sub| PeerStatus {
            cluster_id: sub.config.cluster_id.clone(),
            name: sub.config.name.clone(),
            priority: sub.config.priority,
            enabled: sub.config.enabled,
            active: sub.active,
            entries_imported: sub.entries_imported,
            entries_skipped: sub.entries_skipped,
            entries_filtered: sub.entries_filtered,
            filter: sub.config.filter.clone(),
        })
    }

    /// Process a remote entry from a peer cluster.
    ///
    /// This is called when a `RemoteInsert` event is received from iroh-docs sync.
    /// The entry is imported if:
    /// 1. The subscription is enabled
    /// 2. The key passes the subscription filter
    /// 3. The peer's priority is higher (lower number) than any existing origin
    ///
    /// # Arguments
    /// * `source_cluster_id` - ID of the cluster that sent the entry
    /// * `key` - The key being imported
    /// * `value` - The value being imported
    pub async fn process_remote_entry(
        &self,
        source_cluster_id: &str,
        key: &[u8],
        value: &[u8],
    ) -> Result<ImportResult> {
        // Validate sizes
        if key.len() > MAX_DOC_KEY_SIZE {
            warn!(key_len = key.len(), max = MAX_DOC_KEY_SIZE, "key too large for import, skipping");
            return Ok(ImportResult::Skipped("key too large"));
        }
        if value.len() > MAX_DOC_VALUE_SIZE {
            warn!(value_len = value.len(), max = MAX_DOC_VALUE_SIZE, "value too large for import, skipping");
            return Ok(ImportResult::Skipped("value too large"));
        }

        // Get subscription info
        let (priority, filter_result) = {
            let mut subs = self.subscriptions.write().await;
            let sub = match subs.get_mut(source_cluster_id) {
                Some(s) => s,
                None => return Ok(ImportResult::Skipped("unknown cluster")),
            };

            if !sub.config.enabled {
                return Ok(ImportResult::Skipped("subscription disabled"));
            }

            // Convert key to string for filter check
            let key_str = String::from_utf8_lossy(key);
            if !sub.config.filter.should_include(&key_str) {
                sub.entries_filtered += 1;
                return Ok(ImportResult::Filtered);
            }

            (sub.config.priority, true)
        };

        if !filter_result {
            return Ok(ImportResult::Filtered);
        }

        // Check existing origin for this key
        let key_str = String::from_utf8_lossy(key).to_string();
        let origin_key = KeyOrigin::storage_key(&key_str);

        // Read existing origin to check priority
        let existing_origin = match self.kv_store.read(ReadRequest::new(origin_key.clone())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                KeyOrigin::from_bytes(value.as_bytes())
            }
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => None,
            Err(e) => {
                warn!(error = %e, "failed to read origin, proceeding with import");
                None
            }
        };

        // Check if we should import based on priority
        if let Some(existing) = &existing_origin
            && !existing.should_replace(priority)
        {
            // Update skipped count
            let mut subs = self.subscriptions.write().await;
            if let Some(sub) = subs.get_mut(source_cluster_id) {
                sub.entries_skipped += 1;
            }

            debug!(
                key = %key_str,
                existing_priority = existing.priority,
                new_priority = priority,
                "skipping import due to lower priority"
            );
            return Ok(ImportResult::PrioritySkipped {
                existing_priority: existing.priority,
                new_priority: priority,
            });
        }

        // Create new origin for this key
        let new_origin = KeyOrigin::remote(source_cluster_id, priority, 0);

        // Write the value and origin atomically using SetMulti
        let value_str = String::from_utf8_lossy(value).to_string();
        let origin_value = String::from_utf8_lossy(&new_origin.to_bytes()).to_string();

        // Write both the value and origin in a single batch
        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::SetMulti {
                    pairs: vec![(key_str.clone(), value_str), (origin_key, origin_value)],
                },
            })
            .await
            .map_err(|e| anyhow::anyhow!("failed to write imported data: {}", e))?;

        // Update imported count
        {
            let mut subs = self.subscriptions.write().await;
            if let Some(sub) = subs.get_mut(source_cluster_id) {
                sub.entries_imported += 1;
            }
        }

        debug!(
            key = %key_str,
            source = source_cluster_id,
            priority,
            "imported entry from peer"
        );

        Ok(ImportResult::Imported)
    }

    /// Shutdown the importer.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    /// Get the origin metadata for a key.
    ///
    /// Returns the origin information (cluster ID, priority, timestamp)
    /// for a key if it was imported from a peer cluster.
    ///
    /// # Arguments
    /// * `key` - The key to look up origin for
    ///
    /// # Returns
    /// * `Some(KeyOrigin)` - If the key has origin metadata
    /// * `None` - If the key has no origin (local write or doesn't exist)
    pub async fn get_key_origin(&self, key: &str) -> Option<KeyOrigin> {
        let origin_key = KeyOrigin::storage_key(key);

        match self.kv_store.read(ReadRequest::new(origin_key)).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value)?;
                KeyOrigin::from_bytes(value.as_bytes())
            }
            Err(_) => None,
        }
    }
}

/// Result of attempting to import an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportResult {
    /// Entry was successfully imported.
    Imported,
    /// Entry was filtered out by subscription filter.
    Filtered,
    /// Entry was skipped due to priority conflict.
    PrioritySkipped {
        /// Priority of existing entry.
        existing_priority: u32,
        /// Priority of new entry that was rejected.
        new_priority: u32,
    },
    /// Entry was skipped for another reason.
    Skipped(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspen_core::DeterministicKeyValueStore;
    use aspen_core::KeyValueStoreError;

    fn create_test_importer() -> DocsImporter {
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        DocsImporter::new("local-cluster", kv_store)
    }

    /// Helper to read a value from the test KV store
    async fn read_value(kv_store: &Arc<DeterministicKeyValueStore>, key: &str) -> Option<String> {
        match kv_store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => result.kv.map(|kv| kv.value),
            Err(KeyValueStoreError::NotFound { .. }) => None,
            Err(e) => panic!("unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_add_subscription() {
        let importer = create_test_importer();

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a").with_priority(5);

        importer.add_subscription(sub).await.unwrap();

        let peers = importer.list_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].cluster_id, "cluster-a");
        assert_eq!(peers[0].priority, 5);
    }

    #[tokio::test]
    async fn test_duplicate_subscription_fails() {
        let importer = create_test_importer();

        let sub1 = ClusterSubscription::new("sub-1", "Peer A", "cluster-a");
        let sub2 = ClusterSubscription::new("sub-2", "Peer A Copy", "cluster-a");

        importer.add_subscription(sub1).await.unwrap();
        let result = importer.add_subscription(sub2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_subscription() {
        let importer = create_test_importer();

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a");
        importer.add_subscription(sub).await.unwrap();

        importer.remove_subscription("cluster-a").await.unwrap();

        let peers = importer.list_peers().await;
        assert!(peers.is_empty());
    }

    #[tokio::test]
    async fn test_update_priority() {
        let importer = create_test_importer();

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a").with_priority(5);
        importer.add_subscription(sub).await.unwrap();

        importer.update_priority("cluster-a", 10).await.unwrap();

        let status = importer.get_peer_status("cluster-a").await.unwrap();
        assert_eq!(status.priority, 10);
    }

    #[tokio::test]
    async fn test_update_filter() {
        let importer = create_test_importer();

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a");
        importer.add_subscription(sub).await.unwrap();

        let filter = SubscriptionFilter::include_prefixes(["config/"]);
        importer.update_filter("cluster-a", filter).await.unwrap();

        let status = importer.get_peer_status("cluster-a").await.unwrap();
        assert!(matches!(status.filter, SubscriptionFilter::PrefixFilter(_)));
    }

    #[tokio::test]
    async fn test_import_entry() {
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let importer = DocsImporter::new("local-cluster", kv_store.clone());

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a").with_priority(5);
        importer.add_subscription(sub).await.unwrap();

        let result = importer.process_remote_entry("cluster-a", b"test-key", b"test-value").await.unwrap();

        assert_eq!(result, ImportResult::Imported);

        // Verify the value was written
        let value = read_value(&kv_store, "test-key").await;
        assert_eq!(value, Some("test-value".to_string()));

        // Verify origin was recorded
        let origin_key = KeyOrigin::storage_key("test-key");
        let origin_bytes = read_value(&kv_store, &origin_key).await.unwrap();
        let origin = KeyOrigin::from_bytes(origin_bytes.as_bytes()).unwrap();
        assert_eq!(origin.cluster_id, "cluster-a");
        assert_eq!(origin.priority, 5);
    }

    #[tokio::test]
    async fn test_priority_conflict_resolution() {
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let importer = DocsImporter::new("local-cluster", kv_store.clone());

        // Add two subscriptions with different priorities
        let sub_high = ClusterSubscription::new("sub-1", "High Priority", "cluster-high").with_priority(1); // Higher priority (lower number)
        let sub_low = ClusterSubscription::new("sub-2", "Low Priority", "cluster-low").with_priority(10); // Lower priority (higher number)

        importer.add_subscription(sub_high).await.unwrap();
        importer.add_subscription(sub_low).await.unwrap();

        // First import from low priority cluster
        let result1 = importer.process_remote_entry("cluster-low", b"key", b"low-value").await.unwrap();
        assert_eq!(result1, ImportResult::Imported);

        // Then try to import from high priority cluster - should succeed
        let result2 = importer.process_remote_entry("cluster-high", b"key", b"high-value").await.unwrap();
        assert_eq!(result2, ImportResult::Imported);

        // Verify high priority value won
        let value = read_value(&kv_store, "key").await;
        assert_eq!(value, Some("high-value".to_string()));

        // Now try to import again from low priority - should be skipped
        let result3 = importer.process_remote_entry("cluster-low", b"key", b"new-low-value").await.unwrap();
        assert!(matches!(result3, ImportResult::PrioritySkipped { .. }));

        // Value should still be high priority
        let value = read_value(&kv_store, "key").await;
        assert_eq!(value, Some("high-value".to_string()));
    }

    #[tokio::test]
    async fn test_filter_excludes_keys() {
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let importer = DocsImporter::new("local-cluster", kv_store.clone());

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a")
            .with_filter(SubscriptionFilter::include_prefixes(["allowed/"]));
        importer.add_subscription(sub).await.unwrap();

        // Should be filtered
        let result1 = importer.process_remote_entry("cluster-a", b"blocked/key", b"value").await.unwrap();
        assert_eq!(result1, ImportResult::Filtered);

        // Should be imported
        let result2 = importer.process_remote_entry("cluster-a", b"allowed/key", b"value").await.unwrap();
        assert_eq!(result2, ImportResult::Imported);

        // Verify only allowed key exists
        assert!(read_value(&kv_store, "blocked/key").await.is_none());
        assert_eq!(read_value(&kv_store, "allowed/key").await, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_disabled_subscription() {
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let importer = DocsImporter::new("local-cluster", kv_store.clone());

        let sub = ClusterSubscription::new("sub-1", "Peer A", "cluster-a").with_enabled(false);
        importer.add_subscription(sub).await.unwrap();

        let result = importer.process_remote_entry("cluster-a", b"key", b"value").await.unwrap();

        assert_eq!(result, ImportResult::Skipped("subscription disabled"));
        assert!(read_value(&kv_store, "key").await.is_none());
    }
}
