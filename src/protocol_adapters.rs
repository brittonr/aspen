//! Protocol handler adapters for bridging internal types with trait interfaces.
//!
//! This module provides adapter implementations that allow internal types
//! to work with the trait-based protocol handler interfaces.

use async_trait::async_trait;
use std::sync::Arc;
use aspen_core::{
    AspenDocsTicket, DocsSyncProvider, EndpointProvider, NetworkFactory, PeerInfo,
    PeerManager, StateMachineProvider, SyncStatus,
};
#[cfg(feature = "global-discovery")]
use aspen_core::ContentDiscovery;

use crate::cluster::IrohEndpointManager;
use crate::raft::StateMachineVariant;

/// Adapter for IrohEndpointManager to implement EndpointProvider.
pub struct EndpointProviderAdapter {
    inner: Arc<IrohEndpointManager>,
}

impl EndpointProviderAdapter {
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

    async fn list_entries(&self, _prefix: Option<String>, _limit: Option<u32>) -> Result<Vec<aspen_core::DocsEntry>, String> {
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
    pub fn new(service: Arc<crate::cluster::content_discovery::ContentDiscoveryService>) -> Self {
        Self { inner: service }
    }
}

#[cfg(feature = "global-discovery")]
#[async_trait]
impl ContentDiscovery for ContentDiscoveryAdapter {
    async fn announce(&self, hash: &[u8]) -> Result<(), String> {
        self.inner
            .announce_blob(hash.try_into().map_err(|_| "invalid hash")?)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_providers(&self, hash: &[u8]) -> Result<Vec<String>, String> {
        let providers = self.inner
            .find_blob_providers(hash.try_into().map_err(|_| "invalid hash")?)
            .await
            .map_err(|e| e.to_string())?;

        Ok(providers
            .into_iter()
            .map(|p| p.to_string())
            .collect())
    }
}