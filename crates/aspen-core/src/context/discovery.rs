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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_public_key() -> iroh::PublicKey {
        // Generate a deterministic public key for testing
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    // =========================================================================
    // ContentProviderInfo Tests
    // =========================================================================

    #[test]
    fn content_provider_info_construction() {
        let info = ContentProviderInfo {
            node_id: test_public_key(),
            blob_size: 1024,
            blob_format: iroh_blobs::BlobFormat::Raw,
            discovered_at: 1704067200000000,
            verified: false,
        };

        assert_eq!(info.blob_size, 1024);
        assert!(!info.verified);
    }

    #[test]
    fn content_provider_info_verified() {
        let info = ContentProviderInfo {
            node_id: test_public_key(),
            blob_size: 2048,
            blob_format: iroh_blobs::BlobFormat::HashSeq,
            discovered_at: 1704067200000000,
            verified: true,
        };

        assert!(info.verified);
        assert_eq!(info.blob_size, 2048);
    }

    #[test]
    fn content_provider_info_clone() {
        let info = ContentProviderInfo {
            node_id: test_public_key(),
            blob_size: 512,
            blob_format: iroh_blobs::BlobFormat::Raw,
            discovered_at: 1000000,
            verified: true,
        };

        let cloned = info.clone();

        assert_eq!(info.node_id, cloned.node_id);
        assert_eq!(info.blob_size, cloned.blob_size);
        assert_eq!(info.discovered_at, cloned.discovered_at);
        assert_eq!(info.verified, cloned.verified);
    }

    #[test]
    fn content_provider_info_debug() {
        let info = ContentProviderInfo {
            node_id: test_public_key(),
            blob_size: 100,
            blob_format: iroh_blobs::BlobFormat::Raw,
            discovered_at: 999,
            verified: false,
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("ContentProviderInfo"));
        assert!(debug_str.contains("blob_size"));
        assert!(debug_str.contains("100"));
    }

    #[test]
    fn content_provider_info_zero_size() {
        let info = ContentProviderInfo {
            node_id: test_public_key(),
            blob_size: 0,
            blob_format: iroh_blobs::BlobFormat::Raw,
            discovered_at: 0,
            verified: false,
        };

        assert_eq!(info.blob_size, 0);
        assert_eq!(info.discovered_at, 0);
    }

    #[test]
    fn content_provider_info_large_blob() {
        let info = ContentProviderInfo {
            node_id: test_public_key(),
            blob_size: u64::MAX,
            blob_format: iroh_blobs::BlobFormat::HashSeq,
            discovered_at: u64::MAX,
            verified: true,
        };

        assert_eq!(info.blob_size, u64::MAX);
        assert_eq!(info.discovered_at, u64::MAX);
    }

    // =========================================================================
    // ContentNodeAddr Tests
    // =========================================================================

    #[test]
    fn content_node_addr_construction() {
        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: None,
            direct_addrs: vec![],
        };

        assert!(addr.relay_url.is_none());
        assert!(addr.direct_addrs.is_empty());
    }

    #[test]
    fn content_node_addr_with_relay() {
        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: Some("https://relay.example.com".to_string()),
            direct_addrs: vec![],
        };

        assert_eq!(addr.relay_url, Some("https://relay.example.com".to_string()));
    }

    #[test]
    fn content_node_addr_with_direct_addrs() {
        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: None,
            direct_addrs: vec!["192.168.1.1:4433".to_string(), "[::1]:4433".to_string()],
        };

        assert_eq!(addr.direct_addrs.len(), 2);
        assert!(addr.direct_addrs.contains(&"192.168.1.1:4433".to_string()));
        assert!(addr.direct_addrs.contains(&"[::1]:4433".to_string()));
    }

    #[test]
    fn content_node_addr_full() {
        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: Some("https://relay.example.com".to_string()),
            direct_addrs: vec!["10.0.0.1:4433".to_string(), "10.0.0.2:4433".to_string()],
        };

        assert!(addr.relay_url.is_some());
        assert_eq!(addr.direct_addrs.len(), 2);
    }

    #[test]
    fn content_node_addr_clone() {
        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: Some("https://relay.test".to_string()),
            direct_addrs: vec!["1.2.3.4:5678".to_string()],
        };

        let cloned = addr.clone();

        assert_eq!(addr.public_key, cloned.public_key);
        assert_eq!(addr.relay_url, cloned.relay_url);
        assert_eq!(addr.direct_addrs, cloned.direct_addrs);
    }

    #[test]
    fn content_node_addr_debug() {
        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: None,
            direct_addrs: vec!["127.0.0.1:9999".to_string()],
        };

        let debug_str = format!("{:?}", addr);
        assert!(debug_str.contains("ContentNodeAddr"));
        assert!(debug_str.contains("direct_addrs"));
    }

    #[test]
    fn content_node_addr_many_addrs() {
        let addrs: Vec<String> = (0..100).map(|i| format!("10.0.0.{}:4433", i)).collect();

        let addr = ContentNodeAddr {
            public_key: test_public_key(),
            relay_url: None,
            direct_addrs: addrs.clone(),
        };

        assert_eq!(addr.direct_addrs.len(), 100);
    }
}
