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

use std::fmt;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint as IrohEndpoint;
use iroh::EndpointAddr;
use iroh::RelayMode;
use iroh::SecretKey;
use iroh::protocol::Router;
use iroh_gossip::net::GOSSIP_ALPN;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;

// Bootstrap module requires blob, docs, jobs, and hooks features for full functionality
#[cfg(all(feature = "blob", feature = "docs", feature = "jobs", feature = "hooks"))]
pub mod bootstrap;
pub mod config;
#[cfg(feature = "blob")]
pub mod content_discovery;
#[cfg(feature = "federation")]
pub mod federation;
pub mod gossip;
pub mod gossip_discovery;
pub mod memory_watcher;
pub mod metadata;
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

/// Configuration for Iroh endpoint creation.
///
/// Tiger Style: Fixed limits and explicit configuration.
/// Relay URLs are optional but bounded (max 4 relay servers).
#[derive(Debug, Clone)]
pub struct IrohEndpointConfig {
    /// Optional secret key for the endpoint. If None, a new key is generated.
    pub secret_key: Option<SecretKey>,
    /// Path to persist/load the secret key.
    ///
    /// When set, the endpoint manager will:
    /// 1. Load existing key from this path if the file exists
    /// 2. Save newly generated keys to this path
    ///
    /// This ensures stable node identity across restarts.
    /// If `secret_key` is explicitly provided, it takes priority over the file.
    ///
    /// Recommended: Set to `{data_dir}/iroh_secret_key` for production deployments.
    pub secret_key_path: Option<PathBuf>,
    /// Bind port for the QUIC socket (0 = random port).
    /// Applied to both IPv4 and IPv6 if enabled.
    pub bind_port: u16,
    /// Enable IPv6 binding in addition to IPv4 (default: true).
    ///
    /// When true, the endpoint will bind to both IPv4 (0.0.0.0) and IPv6 (::)
    /// on the specified port. This enables dual-stack operation for better
    /// connectivity in IPv6-enabled environments.
    ///
    /// Set to false if:
    /// - Running on IPv4-only infrastructure
    /// - IPv6 causes connectivity issues in your environment
    /// - You need to bind to a specific IPv4-only interface
    pub enable_ipv6: bool,
    /// Enable gossip-based peer discovery (default: true).
    pub enable_gossip: bool,
    /// Optional explicit gossip topic ID. If None, derived from cluster cookie.
    pub gossip_topic: Option<TopicId>,
    /// Enable mDNS discovery for local network (default: true).
    pub enable_mdns: bool,
    /// Enable DNS discovery for production (default: false).
    pub enable_dns_discovery: bool,
    /// Custom DNS discovery URL (None = use n0's service).
    pub dns_discovery_url: Option<String>,
    /// Enable Pkarr DHT discovery (default: false).
    ///
    /// When enabled, uses `DhtDiscovery` which provides both publishing AND resolution
    /// via the BitTorrent Mainline DHT and optional relay servers. This is more powerful
    /// than the previous `PkarrPublisher` which only supported publishing.
    ///
    /// Features:
    /// - Publishes node addresses to DHT (decentralized, no relay dependency)
    /// - Publishes to relay servers (optional, for fallback)
    /// - Resolves peer addresses from DHT (enables peer discovery)
    /// - Cryptographic authentication via Ed25519 signatures
    pub enable_pkarr: bool,
    /// Enable DHT publishing when Pkarr is enabled (default: true).
    ///
    /// When true, node addresses are published to the BitTorrent Mainline DHT.
    /// This provides decentralized discovery without relay server dependencies.
    ///
    /// Set to false to use relay-only mode (more centralized but potentially faster).
    pub enable_pkarr_dht: bool,
    /// Enable Pkarr relay publishing when Pkarr is enabled (default: true).
    ///
    /// When true, node addresses are also published to Number 0's relay server
    /// at `dns.iroh.link`. This provides a reliable fallback when DHT lookups are slow.
    pub enable_pkarr_relay: bool,
    /// Include direct IP addresses in Pkarr DNS records (default: true).
    ///
    /// When true, both relay URLs and direct addresses are published.
    /// When false, only relay URLs are published (for privacy/NAT scenarios).
    pub include_pkarr_direct_addresses: bool,
    /// Republish delay for Pkarr DHT in seconds (default: 600 = 10 minutes).
    ///
    /// How often to republish addresses to the DHT to maintain freshness.
    /// Lower values increase network traffic but improve discovery reliability.
    pub pkarr_republish_delay_secs: u64,
    /// Custom Pkarr relay URL for discovery (None = use n0's dns.iroh.link).
    ///
    /// For private infrastructure, run your own pkarr relay and set this URL.
    /// Only relevant when `enable_pkarr` and `enable_pkarr_relay` are true.
    pub pkarr_relay_url: Option<String>,
    /// Relay server mode for connection facilitation.
    ///
    /// Relays help establish connections when direct P2P isn't possible (NAT traversal).
    /// - `Default`: Use n0's public relay infrastructure
    /// - `Custom`: Use your own relay servers (requires `relay_urls`)
    /// - `Disabled`: No relays, direct connections only
    pub relay_mode: config::RelayMode,
    /// Custom relay server URLs (required when relay_mode is Custom).
    ///
    /// Tiger Style: Bounded to max 4 relay servers for resource limits.
    /// Recommended to configure 2+ relays in different regions for redundancy.
    pub relay_urls: Vec<String>,
    /// ALPNs (Application-Layer Protocol Negotiation) to accept on this endpoint.
    ///
    /// # When to Set ALPNs
    ///
    /// **Using Iroh Router (recommended, bootstrap.rs):**
    /// - ALPNs are set AUTOMATICALLY via `Router::builder().accept(ALPN, handler)` calls
    /// - DO NOT set `alpns` here when using Router - they will be configured by Router
    /// - The Router handles ALPN-based protocol dispatching for Raft RPC, Gossip, etc.
    ///
    /// **NOT using Router (legacy/testing):**
    /// - Set `alpns` here to the list of protocols this endpoint should accept
    /// - Required for `endpoint.accept()` to work with specific protocols
    /// - Example: `vec![RAFT_AUTH_ALPN.to_vec(), GOSSIP_ALPN.to_vec()]`
    ///
    /// # Common ALPNs in Aspen
    ///
    /// - `RAFT_AUTH_ALPN` ("raft-auth"): Authenticated Raft consensus RPC (recommended)
    /// - `RAFT_ALPN` ("raft-rpc"): Legacy unauthenticated Raft (deprecated)
    /// - `CLIENT_ALPN` ("aspen-client"): Client RPC connections
    /// - `GOSSIP_ALPN` ("iroh-gossip/0"): Peer discovery via iroh-gossip
    ///
    /// # Architecture Note
    ///
    /// In production (aspen-node), the Router is spawned AFTER bootstrap via
    /// `iroh_manager.spawn_router()`, which registers all protocol handlers and
    /// sets ALPNs automatically. The `alpns` field here is primarily for:
    /// - Testing scenarios without Router
    /// - Custom endpoint configurations
    pub alpns: Vec<Vec<u8>>,
}

impl Default for IrohEndpointConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            secret_key_path: None,
            bind_port: 0,
            enable_ipv6: true, // Enable dual-stack by default for better connectivity
            enable_gossip: true,
            gossip_topic: None,
            enable_mdns: true,
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
            enable_pkarr_dht: true,               // DHT enabled by default when pkarr is on
            enable_pkarr_relay: true,             // Relay enabled by default for fallback
            include_pkarr_direct_addresses: true, // Include direct IPs by default
            pkarr_republish_delay_secs: 600,      // 10 minutes default republish
            pkarr_relay_url: None,
            relay_mode: config::RelayMode::Default,
            relay_urls: Vec::new(),
            alpns: Vec::new(),
        }
    }
}

impl IrohEndpointConfig {
    /// Create a new endpoint configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the secret key for deterministic endpoint identity.
    pub fn with_secret_key(mut self, key: SecretKey) -> Self {
        self.secret_key = Some(key);
        self
    }

    /// Set the path for persisting/loading the secret key.
    ///
    /// When set, the endpoint manager will:
    /// - Load the key from this file if it exists
    /// - Save newly generated keys to this file
    ///
    /// This ensures stable node identity across restarts.
    pub fn with_secret_key_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.secret_key_path = Some(path.into());
        self
    }

    /// Set the bind port for the QUIC socket.
    pub fn with_bind_port(mut self, port: u16) -> Self {
        self.bind_port = port;
        self
    }

    /// Enable or disable IPv6 dual-stack binding.
    ///
    /// When enabled (default), the endpoint binds to both IPv4 and IPv6.
    /// Disable for IPv4-only environments or when IPv6 causes issues.
    pub fn with_ipv6(mut self, enable: bool) -> Self {
        self.enable_ipv6 = enable;
        self
    }

    /// Enable or disable gossip-based peer discovery.
    pub fn with_gossip(mut self, enable: bool) -> Self {
        self.enable_gossip = enable;
        self
    }

    /// Set an explicit gossip topic ID.
    pub fn with_gossip_topic(mut self, topic: TopicId) -> Self {
        self.gossip_topic = Some(topic);
        self
    }

    /// Enable or disable mDNS discovery.
    pub fn with_mdns(mut self, enable: bool) -> Self {
        self.enable_mdns = enable;
        self
    }

    /// Enable or disable DNS discovery.
    pub fn with_dns_discovery(mut self, enable: bool) -> Self {
        self.enable_dns_discovery = enable;
        self
    }

    /// Set custom DNS discovery URL.
    pub fn with_dns_discovery_url(mut self, url: String) -> Self {
        self.dns_discovery_url = Some(url);
        self
    }

    /// Enable or disable Pkarr DHT discovery.
    ///
    /// When enabled, uses `DhtDiscovery` which provides both publishing AND resolution
    /// via the BitTorrent Mainline DHT and optional relay servers.
    pub fn with_pkarr(mut self, enable: bool) -> Self {
        self.enable_pkarr = enable;
        self
    }

    /// Enable or disable DHT publishing when Pkarr is enabled.
    ///
    /// When true, node addresses are published to the BitTorrent Mainline DHT.
    /// This provides decentralized discovery without relay server dependencies.
    pub fn with_pkarr_dht(mut self, enable: bool) -> Self {
        self.enable_pkarr_dht = enable;
        self
    }

    /// Enable or disable relay publishing when Pkarr is enabled.
    ///
    /// When true, node addresses are published to Number 0's relay server.
    /// This provides a reliable fallback when DHT lookups are slow.
    pub fn with_pkarr_relay(mut self, enable: bool) -> Self {
        self.enable_pkarr_relay = enable;
        self
    }

    /// Include or exclude direct IP addresses in Pkarr DNS records.
    ///
    /// When true, both relay URLs and direct addresses are published.
    /// When false, only relay URLs are published (for privacy/NAT scenarios).
    pub fn with_pkarr_direct_addresses(mut self, include: bool) -> Self {
        self.include_pkarr_direct_addresses = include;
        self
    }

    /// Set the republish delay for Pkarr DHT in seconds.
    ///
    /// How often to republish addresses to the DHT to maintain freshness.
    /// Lower values increase network traffic but improve discovery reliability.
    pub fn with_pkarr_republish_delay_secs(mut self, secs: u64) -> Self {
        self.pkarr_republish_delay_secs = secs;
        self
    }

    /// Set custom Pkarr relay URL for discovery.
    ///
    /// For private infrastructure, run your own pkarr relay and set this URL.
    /// Only relevant when `enable_pkarr` and `enable_pkarr_relay` are true.
    pub fn with_pkarr_relay_url(mut self, url: String) -> Self {
        self.pkarr_relay_url = Some(url);
        self
    }

    /// Set the relay server mode.
    ///
    /// - `Default`: Use n0's public relay infrastructure
    /// - `Custom`: Use your own relay servers (requires `relay_urls`)
    /// - `Disabled`: No relays, direct connections only
    pub fn with_relay_mode(mut self, mode: config::RelayMode) -> Self {
        self.relay_mode = mode;
        self
    }

    /// Set custom relay server URLs.
    ///
    /// Required when relay_mode is Custom. Recommended to configure 2+ relays
    /// in different regions for redundancy.
    ///
    /// Tiger Style: Bounded to max 4 relay servers.
    pub fn with_relay_urls(mut self, urls: Vec<String>) -> Self {
        // Tiger Style: Bound the number of relay URLs
        self.relay_urls = urls.into_iter().take(4).collect();
        self
    }

    /// Set ALPNs to accept on this endpoint.
    ///
    /// Required when NOT using an Iroh Router for ALPN dispatching.
    /// The endpoint must have at least one ALPN configured to accept incoming connections.
    pub fn with_alpns(mut self, alpns: Vec<Vec<u8>>) -> Self {
        self.alpns = alpns;
        self
    }

    /// Add a single ALPN to accept on this endpoint.
    pub fn with_alpn(mut self, alpn: Vec<u8>) -> Self {
        self.alpns.push(alpn);
        self
    }
}

// ============================================================================
// Router Builder
// ============================================================================

/// Fluent builder for configuring Iroh Router protocol handlers.
///
/// This builder eliminates duplication across `spawn_router`, `spawn_router_extended`,
/// and `spawn_router_full` by providing a unified, fluent API for registering
/// protocol handlers.
///
/// # Example
///
/// ```ignore
/// manager.spawn_router_with(|b| b
///     .raft(raft_handler)
///     .auth_raft(auth_handler)
///     .client(client_handler));
/// ```
///
/// Gossip is automatically registered if enabled on the endpoint manager.
pub struct RouterBuilder {
    builder: iroh::protocol::RouterBuilder,
    gossip: Option<Arc<Gossip>>,
}

impl RouterBuilder {
    /// Create a new router builder.
    pub(crate) fn new(builder: iroh::protocol::RouterBuilder, gossip: Option<Arc<Gossip>>) -> Self {
        Self { builder, gossip }
    }

    /// Register the legacy unauthenticated Raft RPC protocol handler.
    ///
    /// ALPN: `raft-rpc`
    ///
    /// # Security Warning
    ///
    /// This method registers an **unauthenticated** Raft handler. Any node that knows
    /// the endpoint address can connect. For production deployments, use `auth_raft()`
    /// which uses the `raft-auth` ALPN with HMAC-SHA256 authentication.
    ///
    /// # Deprecation
    ///
    /// This method is deprecated. Use `auth_raft()` for new deployments.
    #[deprecated(
        since = "0.2.0",
        note = "Use auth_raft() for production deployments. raft() provides no authentication."
    )]
    #[allow(deprecated)]
    pub fn raft<R: iroh::protocol::ProtocolHandler>(mut self, handler: R) -> Self {
        use aspen_transport::RAFT_ALPN;
        self.builder = self.builder.accept(RAFT_ALPN, handler);
        tracing::warn!(
            "registered LEGACY unauthenticated Raft RPC handler (ALPN: raft-rpc) - use auth_raft() for production"
        );
        self
    }

    /// Register the authenticated Raft RPC protocol handler (optional).
    ///
    /// ALPN: `raft-auth`
    pub fn auth_raft<A: iroh::protocol::ProtocolHandler>(mut self, handler: A) -> Self {
        use aspen_transport::RAFT_AUTH_ALPN;
        self.builder = self.builder.accept(RAFT_AUTH_ALPN, handler);
        tracing::info!("registered authenticated Raft RPC protocol handler (ALPN: raft-auth)");
        self
    }

    /// Register the log subscriber protocol handler (optional).
    ///
    /// ALPN: `aspen-logs`
    pub fn log_subscriber<L: iroh::protocol::ProtocolHandler>(mut self, handler: L) -> Self {
        use aspen_transport::LOG_SUBSCRIBER_ALPN;
        self.builder = self.builder.accept(LOG_SUBSCRIBER_ALPN, handler);
        tracing::info!("registered log subscriber protocol handler (ALPN: aspen-logs)");
        self
    }

    /// Register the client RPC protocol handler (optional).
    ///
    /// ALPN: `aspen-tui`
    pub fn client<C: iroh::protocol::ProtocolHandler>(mut self, handler: C) -> Self {
        use aspen_transport::CLIENT_ALPN;
        self.builder = self.builder.accept(CLIENT_ALPN, handler);
        tracing::info!("registered Client RPC protocol handler (ALPN: aspen-tui)");
        self
    }

    /// Register the blobs protocol handler (optional).
    ///
    /// ALPN: `iroh-blobs/0`
    #[cfg(feature = "blob")]
    pub fn blobs<B: iroh::protocol::ProtocolHandler>(mut self, handler: B) -> Self {
        self.builder = self.builder.accept(iroh_blobs::ALPN, handler);
        tracing::info!("registered Blobs protocol handler (ALPN: iroh-blobs/0)");
        self
    }

    /// Register the Nix cache HTTP/3 gateway protocol handler (optional).
    ///
    /// This serves a Nix binary cache over HTTP/3 using h3-iroh.
    ///
    /// ALPN: `iroh+h3`
    pub fn nix_cache<N: iroh::protocol::ProtocolHandler>(mut self, handler: N) -> Self {
        use aspen_transport::NIX_CACHE_H3_ALPN;
        self.builder = self.builder.accept(NIX_CACHE_H3_ALPN, handler);
        tracing::info!("registered Nix cache HTTP/3 gateway (ALPN: iroh+h3)");
        self
    }

    /// Finalize the router configuration and spawn it.
    ///
    /// Automatically registers gossip if enabled on the endpoint.
    fn spawn_internal(mut self) -> Router {
        // Auto-register gossip if enabled
        if let Some(gossip) = self.gossip {
            self.builder = self.builder.accept(GOSSIP_ALPN, gossip);
            tracing::info!("registered Gossip protocol handler (ALPN: gossip)");
        }
        self.builder.spawn()
    }
}

// ============================================================================
// Iroh Endpoint Manager
// ============================================================================

/// Manages the lifecycle of an Iroh endpoint for P2P transport.
///
/// Uses Iroh Router for proper ALPN-based protocol dispatching.
/// Protocol handlers are registered with the Router during initialization.
///
/// Tiger Style:
/// - Explicit error handling for endpoint creation and connection
/// - Resource cleanup via shutdown method
/// - EndpointAddr exposed for peer discovery via HTTP control-plane
pub struct IrohEndpointManager {
    endpoint: IrohEndpoint,
    node_addr: EndpointAddr,
    secret_key: SecretKey,
    gossip: Option<Arc<Gossip>>,
    /// The Iroh Router for ALPN-based protocol dispatching.
    /// If None, protocol handlers must be registered separately.
    router: Option<Router>,
}

impl IrohEndpointManager {
    /// Create and bind a new Iroh endpoint.
    ///
    /// Tiger Style: Fail fast if endpoint creation fails.
    ///
    /// # Key Persistence
    ///
    /// The secret key is resolved in the following order:
    /// 1. If `config.secret_key` is provided, use it (explicit config takes priority)
    /// 2. If `config.secret_key_path` is set and the file exists, load from file
    /// 3. Otherwise, generate a new key and save it to `secret_key_path` if set
    ///
    /// This ensures stable node identity across restarts when `secret_key_path` is configured.
    pub async fn new(config: IrohEndpointConfig) -> Result<Self> {
        // Resolve secret key with persistence support
        let secret_key = Self::resolve_secret_key(&config)?;

        // Configure transport to allow larger messages for git bridge operations
        // Default QUIC stream receive window is ~1MB, increase for large git objects
        use iroh::endpoint::TransportConfig;
        use iroh::endpoint::VarInt;
        let mut transport_config = TransportConfig::default();
        // Set stream receive window to 64MB to handle large git objects
        transport_config.stream_receive_window(VarInt::from_u32(64 * 1024 * 1024));
        // Set connection receive window to 256MB
        transport_config.receive_window(VarInt::from_u32(256 * 1024 * 1024));
        // Set idle timeout high enough for server-side processing of large git batches.
        // Processing 5000+ tree objects sequentially can take 30-60 seconds.
        // Default QUIC idle timeout is often 30-60s, which is too short.
        transport_config.max_idle_timeout(Some(
            std::time::Duration::from_secs(600).try_into().expect("600 seconds is a valid idle timeout"),
        ));

        // Build endpoint with explicit configuration
        let mut builder = IrohEndpoint::builder();
        builder = builder.secret_key(secret_key.clone());
        builder = builder.transport_config(transport_config);

        // Configure bind addresses if port is specified
        // Tiger Style: Support both IPv4 and IPv6 for dual-stack connectivity
        if config.bind_port > 0 {
            // Always bind IPv4
            let bind_addr_v4 = std::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.bind_port);
            builder = builder.bind_addr_v4(bind_addr_v4);
            tracing::info!(port = config.bind_port, "bound IPv4 address");

            // Optionally bind IPv6 for dual-stack
            if config.enable_ipv6 {
                let bind_addr_v6 = std::net::SocketAddrV6::new(std::net::Ipv6Addr::UNSPECIFIED, config.bind_port, 0, 0);
                builder = builder.bind_addr_v6(bind_addr_v6);
                tracing::info!(port = config.bind_port, "bound IPv6 address (dual-stack)");
            }
        }

        // Configure relay mode based on configuration
        // Relays facilitate connections when direct P2P isn't possible (NAT traversal)
        let relay_mode = match config.relay_mode {
            config::RelayMode::Default => {
                tracing::info!("using default n0 relay infrastructure");
                RelayMode::Default
            }
            config::RelayMode::Disabled => {
                tracing::info!("relay servers disabled - direct connections only");
                RelayMode::Disabled
            }
            config::RelayMode::Custom => {
                if config.relay_urls.is_empty() {
                    // Fall back to default if no custom URLs provided
                    tracing::warn!("relay_mode is Custom but no relay_urls configured, falling back to default");
                    RelayMode::Default
                } else {
                    // Parse relay URLs and construct custom relay map
                    use iroh::RelayUrl;

                    let mut relay_nodes = Vec::new();
                    for url_str in &config.relay_urls {
                        match url_str.parse::<RelayUrl>() {
                            Ok(url) => {
                                relay_nodes.push(url);
                                tracing::info!(url = %url_str, "added custom relay server");
                            }
                            Err(e) => {
                                tracing::warn!(
                                    url = %url_str,
                                    error = %e,
                                    "failed to parse relay URL, skipping"
                                );
                            }
                        }
                    }

                    if relay_nodes.is_empty() {
                        tracing::warn!("no valid relay URLs parsed, falling back to default relays");
                        RelayMode::Default
                    } else {
                        let relay_map: iroh::RelayMap = relay_nodes.into_iter().collect();
                        tracing::info!(relay_count = relay_map.len(), "configured custom relay infrastructure");
                        RelayMode::Custom(relay_map)
                    }
                }
            }
        };
        builder = builder.relay_mode(relay_mode);

        // Configure ALPNs if provided.
        // When using Router (spawn_router()), ALPNs are set automatically via .accept() calls.
        // When NOT using Router (use_router=false), ALPNs must be set here for the endpoint
        // to accept incoming connections.
        if !config.alpns.is_empty() {
            builder = builder.alpns(config.alpns.clone());
            tracing::info!(
                alpns = ?config.alpns.iter().map(|a| String::from_utf8_lossy(a).to_string()).collect::<Vec<_>>(),
                "configured endpoint to accept ALPNs"
            );
        }

        // Configure discovery services for bootstrapping network connectivity
        // mDNS: Local network discovery (default enabled for dev/testing)
        if config.enable_mdns {
            builder = builder.discovery(iroh::discovery::mdns::MdnsDiscovery::builder());
            tracing::info!("mDNS discovery enabled for local network peer discovery");
        }

        // DNS Discovery: Production peer discovery via DNS lookups
        if config.enable_dns_discovery {
            let dns_discovery_builder = if let Some(ref url) = config.dns_discovery_url {
                iroh::discovery::dns::DnsDiscovery::builder(url.clone())
            } else {
                iroh::discovery::dns::DnsDiscovery::n0_dns()
            };
            builder = builder.discovery(dns_discovery_builder);
            tracing::info!(
                "DNS discovery enabled with URL: {}",
                config.dns_discovery_url.as_deref().unwrap_or("n0 DNS service (iroh.link)")
            );
        }

        // Pkarr DHT Discovery: Full publish + resolve via DHT and/or relay
        //
        // DhtDiscovery provides:
        // - Publishing to BitTorrent Mainline DHT (decentralized)
        // - Publishing to relay servers (fallback)
        // - Resolution from both DHT and relay (enabling peer discovery)
        // - Cryptographic authentication via Ed25519 signatures
        //
        // This is a significant upgrade from PkarrPublisher which was publish-only.
        if config.enable_pkarr {
            use iroh::discovery::pkarr::dht::DhtDiscovery;

            // Build DhtDiscovery with configurable options
            let mut dht_builder = DhtDiscovery::builder()
                .dht(config.enable_pkarr_dht)
                .include_direct_addresses(config.include_pkarr_direct_addresses)
                .republish_delay(std::time::Duration::from_secs(config.pkarr_republish_delay_secs));

            // Add relay if enabled
            if config.enable_pkarr_relay {
                if let Some(ref relay_url) = config.pkarr_relay_url {
                    // Use custom pkarr relay URL (private infrastructure)
                    match relay_url.parse::<url::Url>() {
                        Ok(url) => {
                            dht_builder = dht_builder.pkarr_relay(url);
                            tracing::info!(
                                relay_url = %relay_url,
                                "Pkarr using custom relay server (private infrastructure)"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                relay_url = %relay_url,
                                error = %e,
                                "failed to parse custom pkarr relay URL, falling back to n0 relay"
                            );
                            dht_builder = dht_builder.n0_dns_pkarr_relay();
                        }
                    }
                } else {
                    // Use n0's default relay at dns.iroh.link
                    dht_builder = dht_builder.n0_dns_pkarr_relay();
                    tracing::debug!("Pkarr using n0 default relay (dns.iroh.link)");
                }
            }

            builder = builder.discovery(dht_builder);
            tracing::info!(
                dht_enabled = config.enable_pkarr_dht,
                relay_enabled = config.enable_pkarr_relay,
                custom_relay = config.pkarr_relay_url.is_some(),
                include_direct_addrs = config.include_pkarr_direct_addresses,
                republish_delay_secs = config.pkarr_republish_delay_secs,
                "Pkarr DHT discovery enabled (publish + resolve)"
            );
        }

        let endpoint = builder.bind().await.context("failed to bind Iroh endpoint")?;

        // Extract node address for discovery (synchronous in 0.95.1)
        let node_addr = endpoint.addr();

        // Optionally spawn gossip
        let gossip = if config.enable_gossip {
            let gossip = Gossip::builder().spawn(endpoint.clone());
            tracing::info!("gossip spawned for peer discovery");
            Some(Arc::new(gossip))
        } else {
            None
        };

        Ok(Self {
            endpoint,
            node_addr,
            secret_key,
            gossip,
            router: None, // Router is created later via spawn_router()
        })
    }

    /// Resolve the secret key from config, file, or generate a new one.
    ///
    /// Priority order:
    /// 1. Explicit config.secret_key (takes precedence)
    /// 2. Load from config.secret_key_path if file exists
    /// 3. Generate new key and save to secret_key_path if set
    fn resolve_secret_key(config: &IrohEndpointConfig) -> Result<SecretKey> {
        // 1. Explicit key in config takes priority
        if let Some(ref key) = config.secret_key {
            tracing::info!("using explicitly configured secret key");
            return Ok(key.clone());
        }

        // 2. Try to load from file if path is configured
        if let Some(ref path) = config.secret_key_path {
            if path.exists() {
                match Self::load_secret_key_from_file(path) {
                    Ok(key) => {
                        tracing::info!(
                            path = %path.display(),
                            endpoint_id = %key.public().to_string(),
                            "loaded existing secret key from file"
                        );
                        return Ok(key);
                    }
                    Err(e) => {
                        // Log warning but continue to generate new key
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to load secret key from file, generating new key"
                        );
                    }
                }
            }
        }

        // 3. Generate new key
        let secret_key = {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            SecretKey::from(bytes)
        };

        // Save to file if path is configured
        if let Some(ref path) = config.secret_key_path {
            match Self::save_secret_key_to_file(&secret_key, path) {
                Ok(()) => {
                    tracing::info!(
                        path = %path.display(),
                        endpoint_id = %secret_key.public().to_string(),
                        "generated and saved new secret key to file"
                    );
                }
                Err(e) => {
                    // Log error but don't fail - node can still run with ephemeral key
                    tracing::error!(
                        path = %path.display(),
                        error = %e,
                        "failed to save secret key to file - identity will not persist across restarts"
                    );
                }
            }
        } else {
            tracing::info!(
                endpoint_id = %secret_key.public().to_string(),
                "generated ephemeral secret key (no persistence path configured)"
            );
        }

        Ok(secret_key)
    }

    /// Load a secret key from a hex-encoded file.
    ///
    /// File format: 64 hex characters (32 bytes) with optional trailing newline.
    fn load_secret_key_from_file(path: &std::path::Path) -> Result<SecretKey> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read secret key file: {}", path.display()))?;

        let hex_str = contents.trim();

        // Validate length (64 hex chars = 32 bytes)
        if hex_str.len() != 64 {
            anyhow::bail!("invalid secret key file: expected 64 hex characters, got {}", hex_str.len());
        }

        let bytes = hex::decode(hex_str).with_context(|| "failed to decode secret key hex")?;

        let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key length"))?;

        Ok(SecretKey::from(bytes_array))
    }

    /// Save a secret key to a hex-encoded file with restrictive permissions.
    ///
    /// File format: 64 hex characters (32 bytes) with trailing newline.
    /// Permissions: 0600 (owner read/write only) on Unix.
    fn save_secret_key_to_file(key: &SecretKey, path: &std::path::Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        let hex_str = hex::encode(key.to_bytes());
        let contents = format!("{}\n", hex_str);

        // Write file with restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600) // Owner read/write only
                .open(path)
                .with_context(|| format!("failed to create secret key file: {}", path.display()))?;

            use std::io::Write;
            file.write_all(contents.as_bytes())
                .with_context(|| format!("failed to write secret key file: {}", path.display()))?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(path, contents)
                .with_context(|| format!("failed to write secret key file: {}", path.display()))?;
        }

        Ok(())
    }

    /// Get a reference to the underlying Iroh endpoint.
    pub fn endpoint(&self) -> &IrohEndpoint {
        &self.endpoint
    }

    /// Get the node address for peer discovery.
    ///
    /// This should be shared with other nodes via the HTTP control-plane
    /// so they can dial this endpoint.
    pub fn node_addr(&self) -> &EndpointAddr {
        &self.node_addr
    }

    /// Get the secret key used by this endpoint.
    ///
    /// Needed for signing gossip messages for peer discovery.
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Get a reference to the gossip instance, if enabled.
    pub fn gossip(&self) -> Option<&Arc<Gossip>> {
        self.gossip.as_ref()
    }

    /// Get a reference to the router, if spawned.
    pub fn router(&self) -> Option<&Router> {
        self.router.as_ref()
    }

    /// Create a router builder for fluent protocol handler configuration.
    ///
    /// This is the preferred way to spawn a router with custom handlers.
    /// The builder automatically registers gossip if enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// manager.spawn_router_with(|b| b
    ///     .raft(raft_handler)
    ///     .auth_raft(auth_handler)
    ///     .client(client_handler)
    ///     .blobs(blobs_handler));
    /// ```
    pub fn spawn_router_with<F>(&mut self, configure: F)
    where F: FnOnce(RouterBuilder) -> RouterBuilder {
        let builder = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        let configured = configure(builder);
        self.router = Some(configured.spawn_internal());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with protocol handlers.
    ///
    /// This method creates a Router that properly dispatches incoming connections
    /// based on their ALPN. This eliminates the race condition that occurred when
    /// multiple servers were accepting from the same endpoint.
    ///
    /// **Note**: Consider using `spawn_router_with()` for more flexible configuration.
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for Raft RPC (ALPN: `raft-rpc`)
    /// * `tui_handler` - Optional protocol handler for TUI RPC (ALPN: `aspen-tui`)
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    #[allow(deprecated)] // Legacy API for backward compatibility - use spawn_router_extended with auth_raft
    pub fn spawn_router<R, T>(&mut self, raft_handler: R, tui_handler: Option<T>)
    where
        R: iroh::protocol::ProtocolHandler,
        T: iroh::protocol::ProtocolHandler,
    {
        let mut rb = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        #[allow(deprecated)]
        {
            rb = rb.raft(raft_handler);
        }
        if let Some(handler) = tui_handler {
            rb = rb.client(handler);
        }
        self.router = Some(rb.spawn_internal());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with extended protocol handlers.
    ///
    /// This method creates a Router with support for:
    /// - Legacy unauthenticated Raft RPC (ALPN: `raft-rpc`) - required
    /// - Authenticated Raft RPC (ALPN: `raft-auth`) - optional
    /// - Log subscription (ALPN: `aspen-logs`) - optional
    /// - Client RPC (ALPN: `aspen-tui`) - optional
    /// - Gossip (ALPN: `iroh-gossip/0`) - automatic if enabled
    ///
    /// **Note**: Consider using `spawn_router_with()` for more flexible configuration.
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for unauthenticated Raft RPC
    /// * `auth_raft_handler` - Optional handler for authenticated Raft RPC
    /// * `log_subscriber_handler` - Optional handler for log subscription
    /// * `client_handler` - Optional handler for Client RPC
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    #[allow(deprecated)] // Legacy API supports both auth and non-auth raft handlers
    pub fn spawn_router_extended<R, A, L, C>(
        &mut self,
        raft_handler: R,
        auth_raft_handler: Option<A>,
        log_subscriber_handler: Option<L>,
        client_handler: Option<C>,
    ) where
        R: iroh::protocol::ProtocolHandler,
        A: iroh::protocol::ProtocolHandler,
        L: iroh::protocol::ProtocolHandler,
        C: iroh::protocol::ProtocolHandler,
    {
        let mut rb = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        #[allow(deprecated)]
        {
            rb = rb.raft(raft_handler);
        }
        if let Some(handler) = auth_raft_handler {
            rb = rb.auth_raft(handler);
        }
        if let Some(handler) = log_subscriber_handler {
            rb = rb.log_subscriber(handler);
        }
        if let Some(handler) = client_handler {
            rb = rb.client(handler);
        }
        self.router = Some(rb.spawn_internal());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with full protocol support including blobs.
    ///
    /// This method creates a Router with support for:
    /// - Legacy unauthenticated Raft RPC (ALPN: `raft-rpc`) - required
    /// - Authenticated Raft RPC (ALPN: `raft-auth`) - optional
    /// - Log subscription (ALPN: `aspen-logs`) - optional
    /// - Client RPC (ALPN: `aspen-tui`) - optional
    /// - Blobs (ALPN: `iroh-blobs/0`) - optional
    /// - Gossip (ALPN: `iroh-gossip/0`) - automatic if enabled
    ///
    /// **Note**: Consider using `spawn_router_with()` for more flexible configuration.
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for unauthenticated Raft RPC
    /// * `auth_raft_handler` - Optional handler for authenticated Raft RPC
    /// * `log_subscriber_handler` - Optional handler for log subscription
    /// * `client_handler` - Optional handler for Client RPC
    /// * `blobs_handler` - Optional handler for blob transfers
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    #[allow(deprecated)] // Legacy API supports both auth and non-auth raft handlers
    pub fn spawn_router_full<R, A, L, C, B>(
        &mut self,
        raft_handler: R,
        auth_raft_handler: Option<A>,
        log_subscriber_handler: Option<L>,
        client_handler: Option<C>,
        blobs_handler: Option<B>,
    ) where
        R: iroh::protocol::ProtocolHandler,
        A: iroh::protocol::ProtocolHandler,
        L: iroh::protocol::ProtocolHandler,
        C: iroh::protocol::ProtocolHandler,
        B: iroh::protocol::ProtocolHandler,
    {
        let mut rb = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        #[allow(deprecated)]
        {
            rb = rb.raft(raft_handler);
        }
        if let Some(handler) = auth_raft_handler {
            rb = rb.auth_raft(handler);
        }
        if let Some(handler) = log_subscriber_handler {
            rb = rb.log_subscriber(handler);
        }
        if let Some(handler) = client_handler {
            rb = rb.client(handler);
        }
        #[cfg(feature = "blob")]
        if let Some(handler) = blobs_handler {
            rb = rb.blobs(handler);
        }
        #[cfg(not(feature = "blob"))]
        let _ = blobs_handler;
        self.router = Some(rb.spawn_internal());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Add a known peer address to the endpoint for direct connections.
    ///
    /// # Iroh 0.95.1 Behavior
    ///
    /// In Iroh 0.95.1, peer addresses are added implicitly when calling `connect()`.
    /// The endpoint's discovery services (mDNS, DNS, Pkarr) handle address resolution.
    ///
    /// This method logs the address for debugging but does NOT store it directly.
    /// For Raft peer management, use `IrpcRaftNetworkFactory::add_peer()` instead,
    /// which stores addresses in the network factory's peer map.
    ///
    /// # Arguments
    /// * `addr` - The EndpointAddr to add (logged but not stored)
    ///
    /// # Returns
    /// Always returns `Ok(())`. Use `IrpcRaftNetworkFactory::add_peer()` for actual
    /// peer management in Raft clusters.
    pub fn add_peer(&self, addr: EndpointAddr) -> Result<()> {
        // In Iroh 0.95.1, peer addresses are added implicitly via connect() or
        // through discovery services. This method cannot directly store addresses.
        //
        // For Raft peer management, use IrpcRaftNetworkFactory::add_peer() instead.
        // That method stores addresses in the network factory's peer map, which
        // is then used when creating connections via new_client().
        tracing::debug!(
            peer_id = %addr.id,
            direct_addrs = addr.ip_addrs().count(),
            "add_peer called but Iroh 0.95.1 doesn't support direct address storage; \
             use IrpcRaftNetworkFactory::add_peer() for Raft peer management"
        );
        Ok(())
    }

    /// Shutdown the endpoint and close all connections.
    ///
    /// Tiger Style: Explicit cleanup with bounded wait time.
    pub async fn shutdown(&self) -> Result<()> {
        // Shutdown the Router first (this stops accepting new connections)
        if let Some(router) = &self.router {
            router.shutdown().await.context("failed to shutdown Iroh Router")?;
            tracing::info!("Iroh Router shutdown complete");
        }

        // Iroh endpoint shutdown is graceful with internal timeouts
        // In Iroh 0.95.1, close() returns () not Result
        self.endpoint.close().await;
        tracing::info!("Iroh endpoint closed");

        Ok(())
    }
}

impl fmt::Debug for IrohEndpointManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ip_addrs: Vec<_> = self.node_addr.ip_addrs().collect();
        f.debug_struct("IrohEndpointManager")
            .field("node_id", &self.endpoint.id())
            .field("local_endpoints", &ip_addrs)
            .finish()
    }
}

// ============================================================================
// NetworkTransport Implementation
// ============================================================================

#[async_trait::async_trait]
impl transport::NetworkTransport for IrohEndpointManager {
    type Endpoint = IrohEndpoint;
    type Address = EndpointAddr;
    type SecretKey = SecretKey;
    type Gossip = Gossip;

    fn node_addr(&self) -> &Self::Address {
        &self.node_addr
    }

    fn node_id_string(&self) -> String {
        self.endpoint.id().to_string()
    }

    fn secret_key(&self) -> &Self::SecretKey {
        &self.secret_key
    }

    fn endpoint(&self) -> &Self::Endpoint {
        &self.endpoint
    }

    fn gossip(&self) -> Option<&Arc<Self::Gossip>> {
        self.gossip.as_ref()
    }

    async fn shutdown(&self) -> Result<()> {
        // Delegate to the existing shutdown implementation
        IrohEndpointManager::shutdown(self).await
    }
}

impl transport::IrohTransportExt for IrohEndpointManager {
    fn router(&self) -> Option<&Router> {
        self.router.as_ref()
    }
}

// ============================================================================
// Unit Tests for IrohEndpointManager
// ============================================================================

#[cfg(test)]
mod tests {
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

        assert_eq!(config.secret_key_path, Some(PathBuf::from("/tmp/test_key")));
    }

    /// Test default config has no secret_key_path.
    #[test]
    fn test_default_no_secret_key_path() {
        let config = IrohEndpointConfig::default();
        assert!(config.secret_key_path.is_none());
    }
}
