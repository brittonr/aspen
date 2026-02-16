//! Global content discovery for Aspen using the BitTorrent Mainline DHT.
//!
//! This module re-exports from `aspen_dht_discovery` and provides the bridge
//! between gossip-based blob announcements and global DHT discovery.

// Re-export all public types from the extracted crate
// ============================================================================
// Bridge: BlobAnnouncement -> ContentDiscovery
// ============================================================================
use anyhow::Result;
pub use aspen_dht_discovery::ContentDiscoveryConfig;
pub use aspen_dht_discovery::ContentDiscoveryService;
pub use aspen_dht_discovery::DhtNodeAddr;
pub use aspen_dht_discovery::ProviderInfo;
pub use aspen_dht_discovery::to_dht_infohash;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;

use super::gossip_discovery::BlobAnnouncement;

/// Convert a BlobAnnouncement to a content discovery tuple.
pub fn blob_announcement_to_content_info(ann: &BlobAnnouncement) -> (Hash, u64, BlobFormat) {
    (ann.blob_hash, ann.blob_size, ann.blob_format)
}

/// Extension trait for bridging gossip announcements to global discovery.
pub trait ContentDiscoveryExt {
    /// Announce this blob to the global DHT (if configured).
    fn announce_to_dht(
        &self,
        service: &ContentDiscoveryService,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl ContentDiscoveryExt for BlobAnnouncement {
    async fn announce_to_dht(&self, service: &ContentDiscoveryService) -> Result<()> {
        service.announce(self.blob_hash, self.blob_size, self.blob_format).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use iroh::SecretKey;
    use tokio_util::sync::CancellationToken;

    use super::*;

    #[test]
    fn test_dht_infohash_deterministic() {
        let hash = Hash::from_bytes([0x42; 32]);
        let infohash1 = to_dht_infohash(&hash, BlobFormat::Raw);
        let infohash2 = to_dht_infohash(&hash, BlobFormat::Raw);
        assert_eq!(infohash1, infohash2);
    }

    #[test]
    fn test_dht_infohash_format_differs() {
        let hash = Hash::from_bytes([0x42; 32]);
        let raw_infohash = to_dht_infohash(&hash, BlobFormat::Raw);
        let seq_infohash = to_dht_infohash(&hash, BlobFormat::HashSeq);
        assert_ne!(raw_infohash, seq_infohash);
    }

    #[test]
    fn test_content_discovery_config_default() {
        let config = ContentDiscoveryConfig::default();
        assert!(!config.is_enabled); // Opt-in by default
        assert!(!config.server_mode);
        assert!(config.bootstrap_nodes.is_empty());
        assert_eq!(config.dht_port, 0); // Random port
        assert!(!config.auto_announce);
        assert_eq!(config.max_concurrent_queries, 8);
    }

    #[test]
    fn test_content_discovery_config_custom() {
        let config = ContentDiscoveryConfig {
            is_enabled: true,
            server_mode: true,
            bootstrap_nodes: vec!["1.2.3.4:6881".to_string()],
            dht_port: 12345,
            auto_announce: true,
            max_concurrent_queries: 16,
        };

        assert!(config.is_enabled);
        assert!(config.server_mode);
        assert_eq!(config.bootstrap_nodes.len(), 1);
        assert_eq!(config.dht_port, 12345);
        assert!(config.auto_announce);
        assert_eq!(config.max_concurrent_queries, 16);
    }

    #[test]
    fn test_provider_info_fields() {
        let node_id = SecretKey::generate(&mut rand::rng()).public();
        let provider = ProviderInfo {
            node_id,
            blob_size: 4096,
            blob_format: BlobFormat::Raw,
            discovered_at: 1234567890,
            is_verified: false,
        };

        assert_eq!(provider.blob_size, 4096);
        assert_eq!(provider.blob_format, BlobFormat::Raw);
        assert_eq!(provider.discovered_at, 1234567890);
        assert!(!provider.is_verified);
    }

    #[test]
    fn test_provider_info_verified() {
        let node_id = SecretKey::generate(&mut rand::rng()).public();
        let provider = ProviderInfo {
            node_id,
            blob_size: 0,
            blob_format: BlobFormat::HashSeq,
            discovered_at: 0,
            is_verified: true,
        };

        assert!(provider.is_verified);
        assert_eq!(provider.blob_format, BlobFormat::HashSeq);
    }

    #[test]
    fn test_dht_node_addr_creation() {
        let public_key = SecretKey::generate(&mut rand::rng()).public();
        let relay_url = url::Url::parse("https://relay.example.com").unwrap();
        let direct_addrs = vec!["127.0.0.1:1234".parse().unwrap()];

        let addr = DhtNodeAddr::new(public_key, Some(&relay_url), direct_addrs, 2048, BlobFormat::Raw).unwrap();

        assert_eq!(addr.version, 1);
        assert_eq!(addr.blob_size, 2048);
        assert_eq!(addr.blob_format, BlobFormat::Raw);
        assert_eq!(addr.relay_url, Some("https://relay.example.com/".to_string()));
        assert_eq!(addr.direct_addrs.len(), 1);
    }

    #[test]
    fn test_dht_node_addr_no_relay() {
        let public_key = SecretKey::generate(&mut rand::rng()).public();

        let addr = DhtNodeAddr::new(public_key, None, Vec::new(), 1024, BlobFormat::HashSeq).unwrap();

        assert!(addr.relay_url.is_none());
        assert!(addr.direct_addrs.is_empty());
    }

    #[test]
    fn test_dht_node_addr_roundtrip() {
        let public_key = SecretKey::generate(&mut rand::rng()).public();

        let addr = DhtNodeAddr::new(public_key, None, vec!["192.168.1.1:5000".parse().unwrap()], 512, BlobFormat::Raw)
            .unwrap();

        let bytes = addr.to_bytes().unwrap();
        let decoded = DhtNodeAddr::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.version, addr.version);
        assert_eq!(decoded.blob_size, addr.blob_size);
        assert_eq!(decoded.direct_addrs.len(), 1);
    }

    #[test]
    fn test_dht_node_addr_iroh_public_key() {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let public_key = secret_key.public();

        let addr = DhtNodeAddr::new(public_key, None, Vec::new(), 0, BlobFormat::Raw).unwrap();

        let recovered = addr.iroh_public_key().unwrap();
        assert_eq!(recovered, public_key);
    }

    #[test]
    fn test_dht_node_addr_from_invalid_bytes() {
        let result = DhtNodeAddr::from_bytes(b"invalid data");
        assert!(result.is_none());
    }

    #[test]
    fn test_to_dht_infohash_length() {
        let hash = Hash::from_bytes([0x55; 32]);
        let infohash = to_dht_infohash(&hash, BlobFormat::Raw);
        assert_eq!(infohash.len(), 20); // Standard infohash length
    }

    #[test]
    fn test_to_dht_infohash_different_hashes() {
        let hash1 = Hash::from_bytes([0x11; 32]);
        let hash2 = Hash::from_bytes([0x22; 32]);

        let infohash1 = to_dht_infohash(&hash1, BlobFormat::Raw);
        let infohash2 = to_dht_infohash(&hash2, BlobFormat::Raw);

        assert_ne!(infohash1, infohash2);
    }

    #[test]
    fn test_to_dht_infohash_same_hash_different_format() {
        let hash = Hash::from_bytes([0x33; 32]);

        let raw = to_dht_infohash(&hash, BlobFormat::Raw);
        let seq = to_dht_infohash(&hash, BlobFormat::HashSeq);

        // Same hash but different format should produce different infohash
        assert_ne!(raw, seq);
    }

    #[tokio::test]
    async fn test_content_discovery_service_lifecycle() {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder().bind().await.unwrap();

        let config = ContentDiscoveryConfig {
            is_enabled: true,
            ..Default::default()
        };
        let cancel = CancellationToken::new();

        let (service, handle) =
            ContentDiscoveryService::spawn(Arc::new(endpoint), secret_key.clone(), config, cancel.clone())
                .await
                .unwrap();

        assert_eq!(service.public_key(), secret_key.public());

        // Shutdown
        cancel.cancel();
        handle.await.unwrap();
    }
}
