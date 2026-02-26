//! Cluster coordination and peer discovery for Aspen.
//!
//! This module provides the infrastructure for distributed cluster coordination,
//! including:

#![allow(
    dead_code,
    unused_imports,
    clippy::useless_conversion,
    clippy::await_holding_lock,
    clippy::collapsible_if
)]
//!
//! - **Iroh P2P Transport**: QUIC-based peer-to-peer networking with NAT traversal (exclusive
//!   inter-node transport)
//! - **Gossip-based Peer Discovery**: Automatic node discovery via iroh-gossip (default)
//! - **Cluster Tickets**: Compact bootstrap information for joining clusters
//! - **Manual Peer Configuration**: Explicit peer list as fallback when gossip is disabled
//!
//! # Test Coverage
//!
//! Unit tests for IrohEndpointManager cover:
//! - IrohEndpointConfig default values and builder pattern
//! - Relay URL bounding (Tiger Style: max 4)
//! - Gossip topic configuration
//! - Endpoint manager creation with various configs
//! - Secret key generation and persistence
//! - NetworkTransport trait implementation
//! - Graceful shutdown sequence
//!
//! # Peer Discovery
//!
//! Aspen provides multiple automatic discovery mechanisms (all can work simultaneously):
//!
//! ## Iroh Discovery Services (Establish Connectivity)
//!
//! 1. **mDNS** (enabled by default): Discovers peers on the same LAN
//!    - Works automatically for multi-machine testing on same network
//!    - Does NOT work on localhost/127.0.0.1 (multicast limitation)
//!
//! 2. **DNS Discovery** (opt-in): Production peer discovery via DNS service
//!    - Query DNS service for initial bootstrap peers
//!    - Recommended for cloud/multi-region deployments
//!
//! 3. **Pkarr DHT Discovery** (opt-in): Full DHT-based distributed discovery
//!    - Uses `DhtDiscovery` for both publishing AND resolution
//!    - Publishes node addresses to BitTorrent Mainline DHT (decentralized)
//!    - Publishes to relay servers as fallback (configurable)
//!    - Resolves peer addresses from DHT (enables true peer discovery)
//!    - Cryptographic authentication via Ed25519 signatures
//!    - Configuration options: DHT on/off, relay on/off, republish interval
//!
//! ## Gossip (Broadcasts Raft Metadata - Default)
//!
//! Once Iroh connectivity is established (via mDNS, DNS, Pkarr, or manual):
//! 1. Each node subscribes to a gossip topic (derived from cluster cookie)
//! 2. Nodes broadcast their `node_id` + `EndpointAddr` every 10 seconds
//! 3. Received announcements are automatically added to the Raft network factory
//! 4. Raft RPCs can then flow to discovered peers
//!
//! ## Manual Peers (Fallback)
//!
//! When all discovery is disabled, nodes must be configured with explicit peer addresses:
//! - Via CLI: `--peers "node_id@endpoint_id"`
//! - Via config file: `peers = ["node_id@endpoint_id"]`
//! - Use for: single-host testing, airgapped deployments, custom discovery logic
//!
//! # Cluster Tickets
//!
//! Tickets provide a convenient way to join clusters:
//! - First node starts with default gossip (topic from cookie)
//! - HTTP GET `/cluster-ticket` returns a serialized ticket
//! - New nodes use `--ticket "aspen{...}"` to join automatically
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  RaftNode       │  Direct async API for Raft consensus
//! │  (openraft)     │
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │ IrohEndpoint    │  P2P QUIC transport
//! │ (iroh)          │
//! └────────┬────────┘
//!          │
//!          ├─────────► Gossip (peer discovery)
//!          ├─────────► IRPC (Raft RPC)
//!          └─────────► HTTP Control Plane
//! ```

// Bootstrap module requires blob, docs, jobs, and hooks features for full functionality
#[cfg(all(feature = "blob", feature = "docs", feature = "jobs", feature = "hooks"))]
pub mod bootstrap;
pub mod config;
#[cfg(feature = "blob")]
pub mod content_discovery;
pub mod endpoint_config;
pub mod endpoint_manager;
#[cfg(feature = "federation")]
pub mod federation;
pub mod gossip;
pub mod gossip_discovery;
pub mod memory_watcher;
pub mod metadata;
pub mod router_builder;
pub mod ticket;
pub mod transport;
pub mod validation;
pub mod verified;

// Feature-gated bridge modules (re-exported from aspen-cluster-bridges crate)
#[cfg(all(feature = "blob", feature = "hooks"))]
pub use aspen_cluster_bridges::blob_bridge;
#[cfg(all(feature = "docs", feature = "hooks"))]
pub use aspen_cluster_bridges::docs_bridge;
#[cfg(feature = "hooks")]
pub use aspen_cluster_bridges::hooks_bridge;
#[cfg(feature = "hooks")]
pub use aspen_cluster_bridges::snapshot_events_bridge;
#[cfg(feature = "hooks")]
pub use aspen_cluster_bridges::system_events_bridge;
#[cfg(feature = "hooks")]
pub use aspen_cluster_bridges::ttl_events_bridge;

#[cfg(feature = "jobs")]
pub mod worker_service;

// Re-export transport traits and types for convenient access
// Re-export extracted types to preserve the public API
pub use endpoint_config::IrohEndpointConfig;
pub use endpoint_manager::IrohEndpointManager;
pub use router_builder::RouterBuilder;
pub use transport::DiscoveredPeer;
pub use transport::DiscoveryHandle;
pub use transport::IrohTransportExt;
pub use transport::NetworkTransport;
pub use transport::PeerDiscovery;

// Type aliases for concrete transport implementations.
// These provide the specific types used with IrohEndpointManager.
/// Raft network factory using IrohEndpointManager as the transport.
pub type IrpcRaftNetworkFactory = aspen_raft::network::IrpcRaftNetworkFactory<IrohEndpointManager>;
/// Raft connection pool using IrohEndpointManager as the transport.
pub type RaftConnectionPool = aspen_raft::connection_pool::RaftConnectionPool<IrohEndpointManager>;
/// Raft network client using IrohEndpointManager as the transport.
pub type IrpcRaftNetwork = aspen_raft::network::IrpcRaftNetwork<IrohEndpointManager>;

/// Controls how the node server should behave while running in deterministic
/// simulations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeterministicClusterConfig {
    /// Optional seed for deterministic behavior in simulations.
    pub simulation_seed: Option<u64>,
}

// ============================================================================
// Unit Tests for IrohEndpointManager
// ============================================================================

#[cfg(test)]
mod tests {
    use iroh::SecretKey;
    use iroh_gossip::proto::TopicId;

    use super::*;

    /// Test IrohEndpointConfig default values.
    #[test]
    fn test_endpoint_config_defaults() {
        let config = IrohEndpointConfig::default();

        assert!(config.secret_key.is_none());
        assert_eq!(config.bind_port, 0);
        assert!(config.enable_ipv6);
        assert!(config.enable_gossip);
        assert!(config.gossip_topic.is_none());
        assert!(config.enable_mdns);
        assert!(!config.enable_dns_discovery);
        assert!(config.dns_discovery_url.is_none());
        assert!(!config.enable_pkarr);
        assert!(config.enable_pkarr_dht);
        assert!(config.enable_pkarr_relay);
        assert!(config.include_pkarr_direct_addresses);
        assert_eq!(config.pkarr_republish_delay_secs, 600);
        assert!(config.pkarr_relay_url.is_none());
        assert!(matches!(config.relay_mode, config::RelayMode::Default));
        assert!(config.relay_urls.is_empty());
        assert!(config.alpns.is_empty());
    }

    /// Test IrohEndpointConfig builder pattern.
    #[test]
    fn test_endpoint_config_builder() {
        let secret_key = {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            SecretKey::from(bytes)
        };
        let config = IrohEndpointConfig::new()
            .with_secret_key(secret_key.clone())
            .with_bind_port(8080)
            .with_ipv6(false)
            .with_gossip(false)
            .with_mdns(false)
            .with_dns_discovery(true)
            .with_dns_discovery_url("https://dns.example.com".to_string())
            .with_pkarr(true)
            .with_pkarr_dht(false)
            .with_pkarr_relay(true)
            .with_pkarr_direct_addresses(false)
            .with_pkarr_republish_delay_secs(300)
            .with_pkarr_relay_url("https://relay.example.com".to_string())
            .with_relay_mode(config::RelayMode::Disabled)
            .with_alpn(b"test-alpn".to_vec());

        assert!(config.secret_key.is_some());
        assert_eq!(config.bind_port, 8080);
        assert!(!config.enable_ipv6);
        assert!(!config.enable_gossip);
        assert!(!config.enable_mdns);
        assert!(config.enable_dns_discovery);
        assert_eq!(config.dns_discovery_url, Some("https://dns.example.com".to_string()));
        assert!(config.enable_pkarr);
        assert!(!config.enable_pkarr_dht);
        assert!(config.enable_pkarr_relay);
        assert!(!config.include_pkarr_direct_addresses);
        assert_eq!(config.pkarr_republish_delay_secs, 300);
        assert_eq!(config.pkarr_relay_url, Some("https://relay.example.com".to_string()));
        assert!(matches!(config.relay_mode, config::RelayMode::Disabled));
        assert_eq!(config.alpns.len(), 1);
    }

    /// Test relay URL bounding (Tiger Style: max 4 relay servers).
    #[test]
    fn test_relay_urls_bounded() {
        let urls = vec![
            "https://relay1.example.com".to_string(),
            "https://relay2.example.com".to_string(),
            "https://relay3.example.com".to_string(),
            "https://relay4.example.com".to_string(),
            "https://relay5.example.com".to_string(), // Should be dropped
            "https://relay6.example.com".to_string(), // Should be dropped
        ];

        let config = IrohEndpointConfig::new().with_relay_urls(urls);

        // Tiger Style: Should be bounded to 4
        assert_eq!(config.relay_urls.len(), 4);
    }

    /// Test gossip topic configuration.
    #[test]
    fn test_gossip_topic_config() {
        let topic = TopicId::from([1u8; 32]);
        let config = IrohEndpointConfig::new().with_gossip_topic(topic);

        assert!(config.gossip_topic.is_some());
        assert_eq!(config.gossip_topic.unwrap(), TopicId::from([1u8; 32]));
    }

    /// Test IrohEndpointManager creation with minimal config.
    #[tokio::test]
    async fn test_endpoint_manager_creation() {
        // Use minimal config: disable all discovery to avoid network dependencies
        let config = IrohEndpointConfig::new()
            .with_gossip(false)
            .with_mdns(false)
            .with_dns_discovery(false)
            .with_pkarr(false);

        let manager = IrohEndpointManager::new(config).await;
        assert!(manager.is_ok(), "Failed to create endpoint manager: {:?}", manager.err());

        let manager = manager.unwrap();
        assert!(manager.router().is_none()); // Router not spawned yet
        assert!(manager.gossip().is_none()); // Gossip disabled
    }

    /// Test IrohEndpointManager creation with gossip enabled.
    #[tokio::test]
    async fn test_endpoint_manager_with_gossip() {
        let config = IrohEndpointConfig::new()
            .with_gossip(true)
            .with_mdns(false)
            .with_dns_discovery(false)
            .with_pkarr(false);

        let manager = IrohEndpointManager::new(config).await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(manager.gossip().is_some()); // Gossip should be enabled
    }

    /// Test endpoint address is available after creation.
    #[tokio::test]
    async fn test_endpoint_addr_available() {
        let config = IrohEndpointConfig::new().with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Node address should be available
        let addr = manager.node_addr();
        assert!(!addr.id.as_bytes().is_empty());
    }

    /// Test secret key generation when not provided.
    #[tokio::test]
    async fn test_secret_key_generation() {
        let config = IrohEndpointConfig::new().with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Secret key should be generated
        let key = manager.secret_key();
        assert!(!key.to_bytes().is_empty());
    }

    /// Test secret key persistence when provided.
    #[tokio::test]
    async fn test_secret_key_persistence() {
        let secret_key = {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            SecretKey::from(bytes)
        };
        let expected_bytes = secret_key.to_bytes();

        let config = IrohEndpointConfig::new().with_secret_key(secret_key).with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Secret key should match what we provided
        assert_eq!(manager.secret_key().to_bytes(), expected_bytes);
    }

    /// Test graceful shutdown.
    #[tokio::test]
    async fn test_endpoint_shutdown() {
        let config = IrohEndpointConfig::new().with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Shutdown should succeed
        let result = manager.shutdown().await;
        assert!(result.is_ok());
    }

    /// Test NetworkTransport trait implementation.
    #[tokio::test]
    async fn test_network_transport_trait() {
        let config = IrohEndpointConfig::new().with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Test trait methods
        let addr = <IrohEndpointManager as transport::NetworkTransport>::node_addr(&manager);
        assert!(!addr.id.as_bytes().is_empty());

        let node_id_str = <IrohEndpointManager as transport::NetworkTransport>::node_id_string(&manager);
        assert!(!node_id_str.is_empty());

        let _endpoint = <IrohEndpointManager as transport::NetworkTransport>::endpoint(&manager);
        let _secret_key = <IrohEndpointManager as transport::NetworkTransport>::secret_key(&manager);
    }

    /// Test Debug implementation.
    #[tokio::test]
    async fn test_debug_impl() {
        let config = IrohEndpointConfig::new().with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Debug output should work without panic
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("IrohEndpointManager"));
        assert!(debug_str.contains("node_id"));
    }

    /// Test secret key auto-persistence to file.
    #[tokio::test]
    async fn test_secret_key_file_persistence() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let key_path = temp_dir.path().join("iroh_secret_key");

        // Create endpoint with persistence path - should generate and save key
        let config = IrohEndpointConfig::new().with_secret_key_path(&key_path).with_gossip(false).with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();
        let original_key = manager.secret_key().to_bytes();

        // File should exist with correct content
        assert!(key_path.exists(), "secret key file should be created");
        let file_contents = std::fs::read_to_string(&key_path).unwrap();
        let hex_key = file_contents.trim();
        assert_eq!(hex_key.len(), 64, "key file should contain 64 hex chars");

        // Decoded key should match
        let decoded_bytes = hex::decode(hex_key).unwrap();
        assert_eq!(decoded_bytes, original_key.to_vec());

        // Shutdown first manager
        manager.shutdown().await.unwrap();

        // Create new endpoint with same path - should load existing key
        let config2 = IrohEndpointConfig::new().with_secret_key_path(&key_path).with_gossip(false).with_mdns(false);

        let manager2 = IrohEndpointManager::new(config2).await.unwrap();

        // Key should be the same
        assert_eq!(manager2.secret_key().to_bytes(), original_key, "reloaded key should match original");
    }

    /// Test explicit secret_key takes priority over file.
    #[tokio::test]
    async fn test_explicit_key_priority_over_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let key_path = temp_dir.path().join("iroh_secret_key");

        // Create a key file with known content
        let file_key_bytes = [1u8; 32];
        std::fs::write(&key_path, format!("{}\n", hex::encode(file_key_bytes))).unwrap();

        // Create endpoint with explicit key AND path
        let explicit_key = {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            SecretKey::from(bytes)
        };
        let explicit_bytes = explicit_key.to_bytes();

        let config = IrohEndpointConfig::new()
            .with_secret_key(explicit_key)
            .with_secret_key_path(&key_path)
            .with_gossip(false)
            .with_mdns(false);

        let manager = IrohEndpointManager::new(config).await.unwrap();

        // Explicit key should win
        assert_eq!(manager.secret_key().to_bytes(), explicit_bytes, "explicit key should take priority over file");
    }

    /// Test file permissions on Unix.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_secret_key_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::TempDir::new().unwrap();
        let key_path = temp_dir.path().join("iroh_secret_key");

        let config = IrohEndpointConfig::new().with_secret_key_path(&key_path).with_gossip(false).with_mdns(false);

        let _manager = IrohEndpointManager::new(config).await.unwrap();

        // Check file permissions
        let metadata = std::fs::metadata(&key_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "secret key file should have mode 0600");
    }

    /// Test handling of invalid key file content.
    #[tokio::test]
    async fn test_invalid_key_file_generates_new() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let key_path = temp_dir.path().join("iroh_secret_key");

        // Write invalid content
        std::fs::write(&key_path, "invalid-not-hex-not-right-length\n").unwrap();

        let config = IrohEndpointConfig::new().with_secret_key_path(&key_path).with_gossip(false).with_mdns(false);

        // Should succeed by generating new key (and overwriting invalid file)
        let manager = IrohEndpointManager::new(config).await.unwrap();
        assert!(!manager.secret_key().to_bytes().is_empty());

        // File should now have valid content
        let file_contents = std::fs::read_to_string(&key_path).unwrap();
        assert_eq!(file_contents.trim().len(), 64);
    }

    /// Test secret_key_path builder method.
    #[test]
    fn test_secret_key_path_builder() {
        let config = IrohEndpointConfig::new().with_secret_key_path("/tmp/test_key");

        assert_eq!(config.secret_key_path, Some(std::path::PathBuf::from("/tmp/test_key")));
    }

    /// Test default config has no secret_key_path.
    #[test]
    fn test_default_no_secret_key_path() {
        let config = IrohEndpointConfig::default();
        assert!(config.secret_key_path.is_none());
    }

    /// Regression: LogsSinceLast(100) triggered a snapshot race condition
    /// that panicked the Raft core. The constant MIN_SNAPSHOT_LOG_THRESHOLD
    /// enforces a minimum at compile time; this test ensures the constant
    /// itself stays sane and documents the fix.
    #[test]
    fn test_snapshot_threshold_safety_floor() {
        assert!(
            aspen_raft_types::MIN_SNAPSHOT_LOG_THRESHOLD >= 1_000,
            "snapshot threshold safety floor must be >= 1000; \
             see napkin 2026-02-26 snapshot race"
        );
    }
}
