//! Protocol handler adapters for bridging internal types with trait interfaces.
//!
//! This module provides adapter implementations that allow internal types
//! to work with the trait-based protocol handler interfaces.

use std::sync::Arc;

#[cfg(feature = "docs")]
use aspen_blob::prelude::*;
#[cfg(feature = "global-discovery")]
use aspen_core::ContentDiscovery;
use aspen_core::EndpointProvider;
use aspen_core::StateMachineProvider;
use async_trait::async_trait;

use crate::cluster::IrohEndpointManager;
use crate::raft::StateMachineVariant;

/// Adapter for IrohEndpointManager to implement EndpointProvider.
pub struct EndpointProviderAdapter {
    inner: Arc<IrohEndpointManager>,
}

impl EndpointProviderAdapter {
    /// Create a new endpoint provider adapter.
    pub fn new(manager: Arc<IrohEndpointManager>) -> Self {
        Self { inner: manager }
    }
}

#[async_trait]
impl EndpointProvider for EndpointProviderAdapter {
    async fn public_key(&self) -> Vec<u8> {
        // Iroh 0.95 uses secret_key() to get the node's key
        self.inner.endpoint().secret_key().public().as_bytes().to_vec()
    }

    async fn peer_id(&self) -> String {
        // Get node ID from secret key
        self.inner.endpoint().secret_key().public().to_string()
    }

    async fn addresses(&self) -> Vec<String> {
        // Get addresses from node_addr
        let node_addr = self.inner.node_addr();
        // Format the node address using Debug implementation
        vec![format!("{:?}", node_addr)]
    }

    fn node_addr(&self) -> &aspen_core::context::EndpointAddr {
        self.inner.node_addr()
    }

    fn endpoint(&self) -> &aspen_core::context::IrohEndpoint {
        self.inner.endpoint()
    }
}

/// Adapter for StateMachineVariant to implement StateMachineProvider.
pub struct StateMachineProviderAdapter {
    inner: StateMachineVariant,
}

impl StateMachineProviderAdapter {
    /// Create a new state machine provider adapter.
    pub fn new(state_machine: StateMachineVariant) -> Self {
        Self { inner: state_machine }
    }
}

#[async_trait]
impl StateMachineProvider for StateMachineProviderAdapter {
    async fn direct_read(&self, key: &[u8]) -> Option<Vec<u8>> {
        match &self.inner {
            StateMachineVariant::InMemory(_sm) => {
                // InMemory doesn't have direct access methods
                None
            }
            StateMachineVariant::Redb(sm) => {
                // Tiger Style: Log errors rather than silently discarding them
                let key_str = match std::str::from_utf8(key) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::debug!(error = %e, "direct_read: invalid UTF-8 key");
                        return None;
                    }
                };
                match sm.get(key_str) {
                    Ok(entry) => entry.map(|e| e.value.into_bytes()),
                    Err(e) => {
                        tracing::debug!(error = %e, key = ?key_str, "direct_read: get failed");
                        None
                    }
                }
            }
        }
    }

    async fn contains_key(&self, key: &[u8]) -> bool {
        self.direct_read(key).await.is_some()
    }

    async fn direct_scan(&self, prefix: &[u8], limit: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
        // Tiger Style: Log invalid UTF-8 rather than silently returning empty
        let prefix_str = match std::str::from_utf8(prefix) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(error = %e, "direct_scan: invalid UTF-8 prefix");
                return Vec::new();
            }
        };

        match &self.inner {
            StateMachineVariant::InMemory(sm) => {
                let results = sm.scan_kv_with_prefix_async(prefix_str).await;
                results.into_iter().take(limit).map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect()
            }
            StateMachineVariant::Redb(sm) => match sm.scan(prefix_str, None, Some(limit)) {
                Ok(results) => results.into_iter().map(|kv| (kv.key.into_bytes(), kv.value.into_bytes())).collect(),
                Err(e) => {
                    tracing::debug!(error = %e, prefix = ?prefix_str, limit, "direct_scan: scan failed");
                    Vec::new()
                }
            },
        }
    }
}

/// Content discovery adapter for cluster content discovery.
#[cfg(feature = "global-discovery")]
pub struct ContentDiscoveryAdapter {
    inner: Arc<crate::cluster::content_discovery::ContentDiscoveryService>,
}

#[cfg(feature = "global-discovery")]
impl ContentDiscoveryAdapter {
    /// Create a new content discovery adapter wrapping the cluster service.
    pub fn new(service: Arc<crate::cluster::content_discovery::ContentDiscoveryService>) -> Self {
        Self { inner: service }
    }
}

#[cfg(feature = "docs")]
use aspen_core::DocsEntry;
#[cfg(feature = "docs")]
use aspen_core::DocsStatus;
#[cfg(feature = "docs")]
use aspen_core::DocsSyncProvider;

/// Adapter for DocsSyncResources to implement DocsSyncProvider.
///
/// This bridges the iroh-docs based DocsSyncResources with the RPC-facing
/// DocsSyncProvider trait. Content is stored in iroh-blobs for P2P transfer.
#[cfg(feature = "docs")]
pub struct DocsSyncProviderAdapter {
    /// The underlying docs sync resources.
    inner: Arc<aspen_docs::DocsSyncResources>,
    /// Blob store for content storage and retrieval.
    blob_store: Option<Arc<crate::blob::IrohBlobStore>>,
}

#[cfg(feature = "docs")]
impl DocsSyncProviderAdapter {
    /// Create a new docs sync provider adapter.
    ///
    /// # Arguments
    /// * `resources` - The DocsSyncResources from cluster bootstrap
    /// * `blob_store` - Optional blob store for content storage/retrieval
    pub fn new(
        resources: Arc<aspen_docs::DocsSyncResources>,
        blob_store: Option<Arc<crate::blob::IrohBlobStore>>,
    ) -> Self {
        Self {
            inner: resources,
            blob_store,
        }
    }
}

#[cfg(feature = "docs")]
#[async_trait]
impl DocsSyncProvider for DocsSyncProviderAdapter {
    async fn join_document(&self, _doc_id: &[u8]) -> Result<(), String> {
        // Single-namespace mode - document joining not supported
        Err("join_document not supported in single-namespace mode".to_string())
    }

    async fn leave_document(&self, _doc_id: &[u8]) -> Result<(), String> {
        // Single-namespace mode - document leaving not supported
        Err("leave_document not supported in single-namespace mode".to_string())
    }

    async fn get_document(&self, _doc_id: &[u8]) -> Result<Vec<u8>, String> {
        // Single-namespace mode - document retrieval not supported
        Err("get_document not supported in single-namespace mode".to_string())
    }

    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String> {
        // Store content in blob store if available, otherwise compute hash only
        let (hash, len) = if let Some(ref blob_store) = self.blob_store {
            let result = blob_store.add_bytes(&value).await.map_err(|e| e.to_string())?;
            (result.blob_ref.hash, result.blob_ref.size_bytes)
        } else {
            (iroh_blobs::Hash::new(&value), value.len() as u64)
        };

        let author_id = self.inner.author.id();

        self.inner
            .sync_handle
            .insert_local(self.inner.namespace_id, author_id, bytes::Bytes::from(key), hash, len)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_entry(&self, key: &[u8]) -> Result<Option<(Vec<u8>, u64, String)>, String> {
        // Query the sync handle for the entry
        let author_id = self.inner.author.id();

        match self
            .inner
            .sync_handle
            .get_exact(self.inner.namespace_id, author_id, bytes::Bytes::from(key.to_vec()), false)
            .await
        {
            Ok(Some(entry)) => {
                let content_hash = entry.content_hash();
                let content_len = entry.content_len();
                let hash_str = content_hash.to_string();

                // Check for tombstone (single null byte)
                if content_len == 1 {
                    // Might be a tombstone, check the hash
                    const TOMBSTONE: &[u8] = b"\x00";
                    if content_hash == iroh_blobs::Hash::new(TOMBSTONE) {
                        return Ok(None);
                    }
                }

                // Fetch content from blob store if available
                let value = if let Some(ref blob_store) = self.blob_store {
                    match blob_store.get_bytes(&content_hash).await {
                        Ok(Some(bytes)) => bytes.to_vec(),
                        Ok(None) => {
                            // Content not in blob store, return empty with metadata
                            Vec::new()
                        }
                        Err(_) => Vec::new(),
                    }
                } else {
                    // No blob store, return empty with metadata
                    Vec::new()
                };

                Ok(Some((value, content_len, hash_str)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn delete_entry(&self, key: Vec<u8>) -> Result<(), String> {
        // iroh-docs doesn't support empty entries, use tombstone marker
        const TOMBSTONE: &[u8] = b"\x00";

        let (hash, len) = if let Some(ref blob_store) = self.blob_store {
            let result = blob_store.add_bytes(TOMBSTONE).await.map_err(|e| e.to_string())?;
            (result.blob_ref.hash, result.blob_ref.size_bytes)
        } else {
            (iroh_blobs::Hash::new(TOMBSTONE), TOMBSTONE.len() as u64)
        };

        let author_id = self.inner.author.id();

        self.inner
            .sync_handle
            .insert_local(self.inner.namespace_id, author_id, bytes::Bytes::from(key), hash, len)
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_entries(&self, prefix: Option<String>, limit: Option<u32>) -> Result<Vec<DocsEntry>, String> {
        use iroh_docs::api::RpcResult;
        use iroh_docs::store::Query;
        use iroh_docs::sync::SignedEntry;

        // Build query with optional prefix filter
        let query = match prefix {
            Some(p) => Query::single_latest_per_key().key_prefix(p.into_bytes()).build(),
            None => Query::single_latest_per_key().build(),
        };

        // Create irpc mpsc channel for receiving entries
        let (tx, mut rx) = irpc::channel::mpsc::channel::<RpcResult<SignedEntry>>(1000);

        // Start the query
        self.inner
            .sync_handle
            .get_many(self.inner.namespace_id, query, tx)
            .await
            .map_err(|e| e.to_string())?;

        // Collect results with optional limit
        let limit = limit.unwrap_or(10000) as usize;
        let mut entries = Vec::with_capacity(limit.min(1000));

        loop {
            match rx.recv().await {
                Ok(Some(result)) => match result {
                    Ok(entry) => {
                        let key = String::from_utf8_lossy(entry.key()).to_string();
                        let size_bytes = entry.content_len();
                        let hash = entry.content_hash().to_string();

                        // Skip tombstones
                        if size_bytes == 1 {
                            const TOMBSTONE: &[u8] = b"\x00";
                            if entry.content_hash() == iroh_blobs::Hash::new(TOMBSTONE) {
                                continue;
                            }
                        }

                        entries.push(DocsEntry { key, size_bytes, hash });

                        if entries.len() >= limit {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "error during list_entries");
                        break;
                    }
                },
                Ok(None) => break, // Channel closed normally
                Err(e) => {
                    // Tiger Style: Log at debug level since channel closure is expected behavior
                    tracing::debug!(error = %e, "recv error during list_entries - channel closed");
                    break;
                }
            }
        }

        Ok(entries)
    }

    async fn get_status(&self) -> Result<DocsStatus, String> {
        // Check if replica is open by getting its state
        // Tiger Style: Log why state check failed for debugging
        let replica_open = match self.inner.sync_handle.get_state(self.inner.namespace_id).await {
            Ok(_) => true,
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    namespace_id = %self.inner.namespace_id,
                    "get_status: replica state check failed"
                );
                false
            }
        };

        Ok(DocsStatus {
            is_enabled: true,
            namespace_id: Some(self.inner.namespace_id.to_string()),
            author_id: Some(self.inner.author.id().to_string()),
            entry_count: None, // Would require full scan to count
            replica_open: Some(replica_open),
        })
    }

    fn namespace_id(&self) -> String {
        self.inner.namespace_id.to_string()
    }

    fn author_id(&self) -> String {
        self.inner.author.id().to_string()
    }
}

#[cfg(feature = "global-discovery")]
#[async_trait]
impl ContentDiscovery for ContentDiscoveryAdapter {
    async fn announce(&self, hash: iroh_blobs::Hash, size: u64, format: iroh_blobs::BlobFormat) -> Result<(), String> {
        self.inner.announce(hash, size, format).await.map_err(|e| e.to_string())
    }

    async fn find_providers(
        &self,
        hash: iroh_blobs::Hash,
        format: iroh_blobs::BlobFormat,
    ) -> Result<Vec<aspen_core::ContentProviderInfo>, String> {
        let providers = self.inner.find_providers(hash, format).await.map_err(|e| e.to_string())?;

        Ok(providers
            .into_iter()
            .map(|p| aspen_core::ContentProviderInfo {
                node_id: p.node_id,
                blob_size: p.blob_size,
                blob_format: p.blob_format,
                discovered_at: p.discovered_at,
                verified: p.verified,
            })
            .collect())
    }

    async fn find_provider_by_public_key(
        &self,
        public_key: &iroh::PublicKey,
        hash: iroh_blobs::Hash,
        format: iroh_blobs::BlobFormat,
    ) -> Result<Option<aspen_core::ContentNodeAddr>, String> {
        let result =
            self.inner.find_provider_by_public_key(public_key, hash, format).await.map_err(|e| e.to_string())?;

        // Tiger Style: Log when DHT returns invalid public keys (indicates DHT pollution or corruption)
        Ok(result.and_then(|addr| match iroh::PublicKey::from_bytes(&addr.public_key) {
            Ok(parsed_key) => Some(aspen_core::ContentNodeAddr {
                public_key: parsed_key,
                relay_url: addr.relay_url,
                direct_addrs: addr.direct_addrs,
            }),
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    public_key_len = addr.public_key.len(),
                    "find_provider_by_public_key: invalid public key from DHT"
                );
                None
            }
        }))
    }
}

// Peer management adapters for cluster-to-cluster sync

#[cfg(feature = "docs")]
use aspen_core::AspenDocsTicket;
#[cfg(feature = "docs")]
use aspen_core::KeyOrigin;
#[cfg(feature = "docs")]
use aspen_core::PeerConnectionState;
#[cfg(feature = "docs")]
use aspen_core::PeerImporter;
#[cfg(feature = "docs")]
use aspen_core::PeerInfo;
#[cfg(feature = "docs")]
use aspen_core::PeerManager;
#[cfg(feature = "docs")]
use aspen_core::SubscriptionFilter;
#[cfg(feature = "docs")]
use aspen_core::SyncStatus;

/// Adapter for aspen_docs::PeerManager to implement aspen_core::PeerManager trait.
#[cfg(feature = "docs")]
pub struct PeerManagerAdapter {
    inner: Arc<aspen_docs::PeerManager>,
}

#[cfg(feature = "docs")]
impl PeerManagerAdapter {
    /// Create a new peer manager adapter.
    pub fn new(manager: Arc<aspen_docs::PeerManager>) -> Self {
        Self { inner: manager }
    }
}

#[cfg(feature = "docs")]
#[async_trait]
impl PeerManager for PeerManagerAdapter {
    async fn add_peer(&self, ticket: AspenDocsTicket) -> Result<(), String> {
        // Convert aspen_core::AspenDocsTicket to aspen_docs::ticket::AspenDocsTicket
        let docs_ticket = aspen_docs::AspenDocsTicket {
            cluster_id: ticket.cluster_id,
            priority: ticket.priority,
            namespace_id: String::new(), // Not used for peer add
            peers: vec![],               // Not used for peer add
            read_write: false,
        };

        self.inner.add_peer(docs_ticket).await.map_err(|e| e.to_string())
    }

    async fn remove_peer(&self, cluster_id: &str) -> Result<(), String> {
        self.inner.remove_peer(cluster_id).await.map_err(|e| e.to_string())
    }

    async fn list_peers(&self) -> Vec<PeerInfo> {
        self.inner
            .list_peers()
            .await
            .into_iter()
            .map(|p| PeerInfo {
                cluster_id: p.cluster_id,
                name: p.name,
                state: convert_peer_connection_state(p.state),
                priority: p.priority,
                is_enabled: p.is_enabled,
                sync_count: p.sync_count,
                failure_count: p.failure_count,
            })
            .collect()
    }

    async fn sync_status(&self, cluster_id: &str) -> Option<SyncStatus> {
        self.inner.sync_status(cluster_id).await.map(|s| SyncStatus {
            cluster_id: s.cluster_id,
            state: convert_peer_connection_state(s.state),
            syncing: s.syncing,
            entries_received: s.entries_received,
            entries_imported: s.entries_imported,
            entries_skipped: s.entries_skipped,
            entries_filtered: s.entries_filtered,
        })
    }

    fn importer(&self) -> Option<&Arc<dyn PeerImporter>> {
        // The PeerManagerAdapter wraps the concrete PeerManager which has direct
        // access to its importer. We'd need to store an adapter Arc separately.
        // For now, return None since the underlying importer access is via the
        // PeerImporterAdapter which should be constructed and passed separately.
        None
    }
}

/// Convert aspen_docs::PeerConnectionState to aspen_core::PeerConnectionState.
#[cfg(feature = "docs")]
fn convert_peer_connection_state(state: aspen_docs::PeerConnectionState) -> PeerConnectionState {
    match state {
        aspen_docs::PeerConnectionState::Disconnected => PeerConnectionState::Disconnected,
        aspen_docs::PeerConnectionState::Connecting => PeerConnectionState::Connecting,
        aspen_docs::PeerConnectionState::Connected => PeerConnectionState::Connected,
        aspen_docs::PeerConnectionState::Failed => PeerConnectionState::Failed,
    }
}

/// Adapter for aspen_docs::DocsImporter to implement aspen_core::PeerImporter trait.
#[cfg(feature = "docs")]
pub struct PeerImporterAdapter {
    inner: Arc<aspen_docs::DocsImporter>,
}

#[cfg(feature = "docs")]
impl PeerImporterAdapter {
    /// Create a new peer importer adapter.
    pub fn new(importer: Arc<aspen_docs::DocsImporter>) -> Self {
        Self { inner: importer }
    }
}

#[cfg(feature = "docs")]
#[async_trait]
impl PeerImporter for PeerImporterAdapter {
    async fn update_filter(&self, cluster_id: &str, filter: SubscriptionFilter) -> Result<(), String> {
        let docs_filter = convert_subscription_filter(filter);
        self.inner.update_filter(cluster_id, docs_filter).await.map_err(|e| e.to_string())
    }

    async fn update_priority(&self, cluster_id: &str, priority: u32) -> Result<(), String> {
        self.inner.update_priority(cluster_id, priority).await.map_err(|e| e.to_string())
    }

    async fn set_enabled(&self, cluster_id: &str, enabled: bool) -> Result<(), String> {
        self.inner.set_enabled(cluster_id, enabled).await.map_err(|e| e.to_string())
    }

    async fn get_key_origin(&self, key: &str) -> Option<KeyOrigin> {
        self.inner.get_key_origin(key).await.map(|o| KeyOrigin {
            cluster_id: o.cluster_id,
            priority: o.priority,
            // Convert HLC timestamp (milliseconds) to seconds
            timestamp_secs: o.hlc_timestamp.to_unix_ms() / 1000,
        })
    }
}

/// Convert aspen_core::SubscriptionFilter to aspen_client::SubscriptionFilter.
#[cfg(feature = "docs")]
fn convert_subscription_filter(filter: SubscriptionFilter) -> aspen_client::SubscriptionFilter {
    match filter {
        SubscriptionFilter::FullReplication => aspen_client::SubscriptionFilter::FullReplication,
        SubscriptionFilter::PrefixFilter(prefixes) => aspen_client::SubscriptionFilter::PrefixFilter(prefixes),
        SubscriptionFilter::PrefixExclude(prefixes) => aspen_client::SubscriptionFilter::PrefixExclude(prefixes),
    }
}
