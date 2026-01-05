//! Protocol handler adapters for bridging internal types with trait interfaces.
//!
//! This module provides adapter implementations that allow internal types
//! to work with the trait-based protocol handler interfaces.

use std::sync::Arc;

use aspen_core::AspenDocsTicket;
#[cfg(feature = "global-discovery")]
use aspen_core::ContentDiscovery;
use aspen_core::DocsSyncProvider;
use aspen_core::EndpointProvider;
use aspen_core::NetworkFactory;
use aspen_core::PeerInfo;
use aspen_core::PeerManager;
use aspen_core::StateMachineProvider;
use aspen_core::SyncStatus;
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
                // Use the get method to read value
                let key_str = std::str::from_utf8(key).ok()?;
                sm.get(key_str).ok()?.map(|entry| entry.value.into_bytes())
            }
        }
    }

    async fn contains_key(&self, key: &[u8]) -> bool {
        self.direct_read(key).await.is_some()
    }

    async fn direct_scan(&self, _prefix: &[u8], _limit: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
        // Direct scan not implemented for now
        Vec::new()
    }
}

/// Network factory adapter - stub for now.
pub struct NetworkFactoryAdapter;

impl NetworkFactoryAdapter {
    /// Create a new network factory adapter.
    pub fn new(_member_info: Arc<crate::raft::types::RaftMemberInfo>) -> Self {
        Self
    }
}

#[async_trait]
impl NetworkFactory for NetworkFactoryAdapter {
    async fn add_peer(&self, _node_id: u64, _address: String) -> Result<(), String> {
        // Network factory not fully implemented yet
        Ok(())
    }

    async fn remove_peer(&self, _node_id: u64) -> Result<(), String> {
        // Network factory not fully implemented yet
        Ok(())
    }
}

/// Stub peer manager for future implementation.
pub struct PeerManagerStub;

#[async_trait]
impl PeerManager for PeerManagerStub {
    async fn add_peer(&self, _ticket: AspenDocsTicket) -> Result<(), String> {
        Err("peer management not implemented".to_string())
    }

    async fn remove_peer(&self, _cluster_id: &str) -> Result<(), String> {
        Err("peer management not implemented".to_string())
    }

    async fn list_peers(&self) -> Vec<PeerInfo> {
        Vec::new()
    }

    async fn sync_status(&self, _cluster_id: &str) -> Option<SyncStatus> {
        None
    }

    fn importer(&self) -> &Arc<dyn aspen_core::PeerImporter> {
        // Return a stub - this would need proper implementation
        panic!("PeerManager stub does not support importer access")
    }
}

/// Stub docs sync provider for future implementation.
pub struct DocsSyncProviderStub;

#[async_trait]
impl DocsSyncProvider for DocsSyncProviderStub {
    async fn join_document(&self, _doc_id: &[u8]) -> Result<(), String> {
        Err("document sync not implemented".to_string())
    }

    async fn leave_document(&self, _doc_id: &[u8]) -> Result<(), String> {
        Err("document sync not implemented".to_string())
    }

    async fn get_document(&self, _doc_id: &[u8]) -> Result<Vec<u8>, String> {
        Err("document sync not implemented".to_string())
    }

    async fn set_entry(&self, _key: Vec<u8>, _value: Vec<u8>) -> Result<(), String> {
        Err("document sync not implemented".to_string())
    }

    async fn get_entry(&self, _key: &[u8]) -> Result<Option<(Vec<u8>, u64, String)>, String> {
        Err("document sync not implemented".to_string())
    }

    async fn delete_entry(&self, _key: Vec<u8>) -> Result<(), String> {
        Err("document sync not implemented".to_string())
    }

    async fn list_entries(
        &self,
        _prefix: Option<String>,
        _limit: Option<u32>,
    ) -> Result<Vec<aspen_core::DocsEntry>, String> {
        Err("document sync not implemented".to_string())
    }

    async fn get_status(&self) -> Result<aspen_core::DocsStatus, String> {
        Err("document sync not implemented".to_string())
    }

    fn namespace_id(&self) -> String {
        "not-implemented".to_string()
    }

    fn author_id(&self) -> String {
        "not-implemented".to_string()
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

        Ok(result.map(|addr| aspen_core::ContentNodeAddr {
            public_key: iroh::PublicKey::from_bytes(&addr.public_key).expect("valid public key"),
            relay_url: addr.relay_url,
            direct_addrs: addr.direct_addrs,
        }))
    }
}
