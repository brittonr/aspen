//! Iroh P2P networking configuration.
//!
//! Contains configuration for Iroh endpoint, relay servers, discovery mechanisms,
//! and gossip-based peer discovery.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Relay server mode for Iroh connections.
///
/// Relays facilitate connections when direct peer-to-peer isn't possible (NAT traversal)
/// and help with hole-punching. They are stateless connection facilitators.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RelayMode {
    /// Use n0's public relay infrastructure (default).
    ///
    /// Good for development and testing. For production, consider using
    /// dedicated relays for better performance and control.
    #[default]
    Default,

    /// Use custom relay servers.
    ///
    /// Requires `relay_urls` to be configured with at least one relay URL.
    /// Recommended to have 2+ relays in different regions for redundancy.
    Custom,

    /// Disable relay servers entirely.
    ///
    /// Only direct peer-to-peer connections will work. This is only suitable
    /// for networks where all nodes can directly reach each other (no NAT).
    Disabled,
}

impl std::str::FromStr for RelayMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "default" => Ok(RelayMode::Default),
            "custom" => Ok(RelayMode::Custom),
            "disabled" => Ok(RelayMode::Disabled),
            _ => Err(format!("invalid relay mode: {s}, expected: default, custom, disabled")),
        }
    }
}

/// Iroh networking configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IrohConfig {
    /// Hex-encoded Iroh secret key (64 hex characters = 32 bytes).
    /// If not provided, a new key is generated.
    pub secret_key: Option<String>,

    /// Enable iroh-gossip for automatic peer discovery.
    ///
    /// When enabled, nodes broadcast their presence and discover peers automatically
    /// via a shared gossip topic. The topic ID is derived from the cluster cookie or
    /// provided via a cluster ticket.
    ///
    /// When disabled, only manual peers (from --peers CLI flag) are used for connections.
    ///
    /// Default: true (gossip enabled).
    #[serde(default = "default_enable_gossip")]
    pub enable_gossip: bool,

    /// Aspen cluster ticket for gossip-based bootstrap.
    /// Contains the gossip topic ID and bootstrap peer endpoints.
    /// Format: "aspen{base32-encoded-data}"
    /// If provided, overrides manual peers for gossip bootstrap.
    pub gossip_ticket: Option<String>,

    /// Enable mDNS discovery for local network peer discovery.
    ///
    /// When enabled, nodes automatically discover peers on the same local network
    /// without requiring relay servers or manual configuration. Perfect for development
    /// and testing environments.
    ///
    /// Default: true (mDNS enabled).
    #[serde(default = "default_enable_mdns")]
    pub enable_mdns: bool,

    /// Enable DNS discovery for production peer discovery.
    ///
    /// When enabled, nodes can discover each other via DNS lookups.
    /// Uses n0's public DNS service by default, or a custom URL if dns_discovery_url is provided.
    ///
    /// Default: false (DNS discovery disabled).
    #[serde(default)]
    pub enable_dns_discovery: bool,

    /// Custom DNS discovery service URL.
    ///
    /// If not provided, uses n0's public DNS service (<https://dns.iroh.link>).
    /// Only relevant when enable_dns_discovery is true.
    pub dns_discovery_url: Option<String>,

    /// Enable Pkarr DHT discovery for distributed peer discovery.
    ///
    /// When enabled, uses `DhtDiscovery` which provides both publishing AND resolution
    /// via the BitTorrent Mainline DHT and optional relay servers. This is a significant
    /// upgrade from the previous `PkarrPublisher` which only supported publishing.
    ///
    /// Features:
    /// - Publishes node addresses to DHT (decentralized, no relay dependency)
    /// - Publishes to relay servers (optional, for fallback)
    /// - Resolves peer addresses from DHT (enables peer discovery)
    /// - Cryptographic authentication via Ed25519 signatures
    ///
    /// **Security**: When this is enabled, `enable_raft_auth` is automatically set to
    /// true (unless explicitly disabled). This prevents unauthorized nodes from joining
    /// the cluster after discovering the node's address via the public DHT.
    ///
    /// Default: false (Pkarr disabled).
    #[serde(default)]
    pub enable_pkarr: bool,

    /// Enable DHT publishing when Pkarr is enabled (default: true).
    ///
    /// When true, node addresses are published to the BitTorrent Mainline DHT.
    /// This provides decentralized discovery without relay server dependencies.
    ///
    /// Set to false to use relay-only mode (more centralized but potentially faster).
    #[serde(default = "default_enable_pkarr_dht")]
    pub enable_pkarr_dht: bool,

    /// Enable Pkarr relay publishing when Pkarr is enabled (default: true).
    ///
    /// When true, node addresses are also published to Number 0's relay server
    /// at `dns.iroh.link`. This provides a reliable fallback when DHT lookups are slow.
    #[serde(default = "default_enable_pkarr_relay")]
    pub enable_pkarr_relay: bool,

    /// Include direct IP addresses in Pkarr DNS records (default: true).
    ///
    /// When true, both relay URLs and direct addresses are published.
    /// When false, only relay URLs are published (for privacy/NAT scenarios).
    #[serde(default = "default_include_pkarr_direct_addresses")]
    pub include_pkarr_direct_addresses: bool,

    /// Republish delay for Pkarr DHT in seconds (default: 600 = 10 minutes).
    ///
    /// How often to republish addresses to the DHT to maintain freshness.
    /// Lower values increase network traffic but improve discovery reliability.
    #[serde(default = "default_pkarr_republish_delay_secs")]
    pub pkarr_republish_delay_secs: u64,

    /// Custom Pkarr relay URL for discovery.
    ///
    /// If not provided, uses n0's public Pkarr relay (`dns.iroh.link`).
    /// For private infrastructure, run your own pkarr relay and set this URL.
    ///
    /// Only relevant when `enable_pkarr` and `enable_pkarr_relay` are true.
    pub pkarr_relay_url: Option<String>,

    /// Relay server mode for connection facilitation (default: default).
    ///
    /// Relays help establish connections when direct P2P isn't possible (NAT traversal).
    /// Options:
    /// - `default`: Use n0's public relay infrastructure
    /// - `custom`: Use your own relay servers (requires `relay_urls`)
    /// - `disabled`: No relays, direct connections only
    ///
    /// For production, dedicated relays are recommended for better performance and control.
    #[serde(default)]
    pub relay_mode: RelayMode,

    /// Custom relay server URLs (required when relay_mode is "custom").
    ///
    /// Recommended to configure 2+ relays in different regions for redundancy.
    /// Example: `["https://relay-us.company.com", "https://relay-eu.company.com"]`
    ///
    /// Each URL should point to an iroh-relay server.
    /// See: <https://github.com/n0-computer/iroh/tree/main/iroh-relay>
    #[serde(default)]
    pub relay_urls: Vec<String>,

    /// Enable HMAC-SHA256 authentication for Raft RPC.
    ///
    /// When enabled, nodes perform mutual authentication using the cluster
    /// cookie before accepting Raft RPC requests. Prevents unauthorized nodes
    /// from participating in consensus.
    ///
    /// Default: false (for backwards compatibility).
    #[serde(default)]
    pub enable_raft_auth: bool,

    /// Port to bind for QUIC connections.
    ///
    /// - 0: Use random port (default)
    /// - Other: Use specific port (e.g., 7777 for VM deployments)
    ///
    /// Default: 0 (random port).
    #[serde(default)]
    pub bind_port: u16,
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            enable_gossip: default_enable_gossip(),
            gossip_ticket: None,
            enable_mdns: default_enable_mdns(),
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
            enable_pkarr_dht: default_enable_pkarr_dht(),
            enable_pkarr_relay: default_enable_pkarr_relay(),
            include_pkarr_direct_addresses: default_include_pkarr_direct_addresses(),
            pkarr_republish_delay_secs: default_pkarr_republish_delay_secs(),
            pkarr_relay_url: None,
            relay_mode: RelayMode::default(),
            relay_urls: Vec::new(),
            enable_raft_auth: false,
            bind_port: 0,
        }
    }
}

impl IrohConfig {
    /// Apply security defaults based on configuration.
    ///
    /// This method enforces security-by-default policies:
    /// - When Pkarr DHT discovery is enabled, Raft authentication is automatically enabled to
    ///   prevent unauthorized nodes from joining the cluster.
    ///
    /// Returns true if any settings were changed.
    pub fn apply_security_defaults(&mut self) -> bool {
        let mut changed = false;

        // Auto-enable Raft auth when Pkarr is enabled
        // Rationale: Pkarr publishes node addresses to a public DHT, making nodes
        // discoverable by anyone. Without Raft auth, any node that discovers the
        // address could potentially join the cluster and disrupt consensus.
        if self.enable_pkarr && !self.enable_raft_auth {
            self.enable_raft_auth = true;
            changed = true;
            tracing::info!(
                "Raft authentication auto-enabled because Pkarr DHT discovery is enabled. \
                 This prevents unauthorized nodes from joining the cluster."
            );
        }

        changed
    }
}

// Default value functions

pub(crate) fn default_enable_gossip() -> bool {
    true
}

pub(crate) fn default_enable_mdns() -> bool {
    true
}

pub(crate) fn default_enable_pkarr_dht() -> bool {
    true // DHT enabled by default when pkarr is on
}

pub(crate) fn default_enable_pkarr_relay() -> bool {
    true // Relay enabled by default for fallback
}

pub(crate) fn default_include_pkarr_direct_addresses() -> bool {
    true // Include direct IPs by default
}

pub(crate) fn default_pkarr_republish_delay_secs() -> u64 {
    600 // 10 minutes default republish
}
