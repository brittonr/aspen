//! Content discovery via DHT.
//!
//! Provides traits and types for discovering content providers
//! via the distributed hash table.
//!
//! This module is feature-gated with `global-discovery`.

use async_trait::async_trait;

/// Information about a content provider discovered via DHT.
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
