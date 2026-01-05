//! Protocol handler adapters for bridging internal types with trait interfaces.
//!
//! This module provides adapter implementations that allow internal types
//! to work with the trait-based protocol handler interfaces.

use std::sync::Arc;

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
                // Use the get method to read value
                let key_str = std::str::from_utf8(key).ok()?;
                sm.get(key_str).ok()?.map(|entry| entry.value.into_bytes())
            }
        }
    }

    async fn contains_key(&self, key: &[u8]) -> bool {
        self.direct_read(key).await.is_some()
    }

    async fn direct_scan(&self, prefix: &[u8], limit: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
        let prefix_str = match std::str::from_utf8(prefix) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        match &self.inner {
            StateMachineVariant::InMemory(sm) => {
                let results = sm.scan_kv_with_prefix_async(prefix_str).await;
                results.into_iter().take(limit).map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect()
            }
            StateMachineVariant::Redb(sm) => match sm.scan(prefix_str, None, Some(limit)) {
                Ok(results) => results.into_iter().map(|kv| (kv.key.into_bytes(), kv.value.into_bytes())).collect(),
                Err(_) => Vec::new(),
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

        // Use and_then to gracefully filter out invalid public keys from untrusted DHT data
        Ok(result.and_then(|addr| {
            let parsed_key = iroh::PublicKey::from_bytes(&addr.public_key).ok()?;
            Some(aspen_core::ContentNodeAddr {
                public_key: parsed_key,
                relay_url: addr.relay_url,
                direct_addrs: addr.direct_addrs,
            })
        }))
    }
}
