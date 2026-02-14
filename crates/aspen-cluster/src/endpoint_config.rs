//! Configuration for Iroh endpoint creation.

use std::path::PathBuf;

use iroh::SecretKey;
use iroh_gossip::proto::TopicId;

use crate::config;

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
