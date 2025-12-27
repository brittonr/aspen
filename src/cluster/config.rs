//! Cluster configuration types and validation.
//!
//! Defines configuration structures for Aspen cluster nodes, supporting multi-layer
//! configuration loading from environment variables, TOML files, and command-line
//! arguments. Configuration precedence is: environment < TOML < CLI args, ensuring
//! operator overrides always take priority.
//!
//! # Key Components
//!
//! - `NodeConfig`: Top-level node configuration (node_id, data_dir, addresses)
//! - `ControlBackend`: Control plane backend (Raft)
//! - `IrohConfig`: P2P networking configuration (endpoint addresses, tickets)
//! - `StorageBackend`: Log and state machine backend selection
//! - `SupervisionConfig`: Raft actor supervision and health check parameters
//! - Configuration builders: Programmatic API for testing and deployment tools
//!
//! # Configuration Sources
//!
//! 1. Environment variables: `ASPEN_NODE_ID`, `ASPEN_DATA_DIR`, `ASPEN_RAFT_ADDR`, etc.
//! 2. TOML configuration file: `--config /path/to/config.toml`
//! 3. Command-line arguments: `--node-id 1 --raft-addr 127.0.0.1:5301`
//!
//! Precedence: CLI > TOML > Environment (highest to lowest)
//!
//! # Tiger Style
//!
//! - Explicit types: u64 for node_id, SocketAddr for addresses (type-safe)
//! - Default values: Sensible defaults for optional fields (data_dir, timeouts)
//! - Validation: FromStr implementations fail fast on invalid input
//! - Serialization: TOML format for human-editable config files
//! - Fixed limits: Supervision config includes bounded retry counts and timeouts
//! - Path handling: Absolute paths preferred, relative paths resolved early
//!
//! # Example TOML
//!
//! ```toml
//! node_id = 1
//! data_dir = "./data/node-1"
//! raft_addr = "127.0.0.1:5301"
//! storage_backend = "redb"
//! control_backend = "Raft"
//!
//! [iroh]
//! bind_port = 4301
//!
//! [supervision]
//! max_restart_count = 5
//! health_check_interval_ms = 5000
//! ```
//!
//! # Example Usage
//!
//! ```ignore
//! use aspen::cluster::config::NodeConfig;
//!
//! // From TOML
//! let config: NodeConfig = toml::from_str(&toml_str)?;
//!
//! // Programmatic
//! let config = NodeConfig {
//!     node_id: 1,
//!     raft_addr: "127.0.0.1:5301".parse()?,
//!     ..Default::default()
//! };
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;
use snafu::Snafu;

use crate::raft::storage::StorageBackend;
// SupervisionConfig removed - was legacy from actor-based architecture

/// Configuration for an Aspen cluster node.
///
/// Configuration is loaded in layers with the following precedence (lowest to highest):
/// 1. Environment variables (ASPEN_*)
/// 2. TOML configuration file
/// 3. Command-line arguments
///
/// This means CLI args override TOML config, which overrides environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Logical Raft node identifier.
    pub node_id: u64,

    /// Directory for persistent data storage (metadata, Raft logs, state machine).
    /// Defaults to "./data/node-{node_id}" if not specified.
    pub data_dir: Option<PathBuf>,

    /// Storage backend for Raft log and state machine.
    /// - Redb: Single-fsync ACID-compliant redb storage (recommended for production)
    /// - InMemory: Fast, non-durable (data lost on restart), good for testing
    ///
    /// Default: Redb
    #[serde(default)]
    pub storage_backend: StorageBackend,

    /// Path for redb-backed shared database (log + state machine).
    /// Only used when storage_backend = Redb.
    /// Defaults to "{data_dir}/node_{id}_shared.redb" if not specified.
    pub redb_path: Option<PathBuf>,

    /// Hostname for informational purposes.
    #[serde(default = "default_host")]
    pub host: String,

    /// Shared cookie for cluster authentication.
    #[serde(default = "default_cookie")]
    pub cookie: String,

    /// Address for the HTTP control API.
    #[serde(default = "default_http_addr")]
    pub http_addr: SocketAddr,

    /// Control-plane implementation to use for this node.
    #[serde(default)]
    pub control_backend: ControlBackend,

    /// Raft heartbeat interval in milliseconds.
    #[serde(default = "default_heartbeat_interval_ms")]
    pub heartbeat_interval_ms: u64,

    /// Minimum Raft election timeout in milliseconds.
    #[serde(default = "default_election_timeout_min_ms")]
    pub election_timeout_min_ms: u64,

    /// Maximum Raft election timeout in milliseconds.
    #[serde(default = "default_election_timeout_max_ms")]
    pub election_timeout_max_ms: u64,

    /// Iroh-specific configuration.
    #[serde(default)]
    pub iroh: IrohConfig,

    /// iroh-docs configuration for real-time KV synchronization.
    #[serde(default)]
    pub docs: DocsConfig,

    /// iroh-blobs configuration for content-addressed storage.
    #[serde(default)]
    pub blobs: BlobConfig,

    /// Peer cluster synchronization configuration.
    #[serde(default)]
    pub peer_sync: PeerSyncConfig,

    /// Horizontal sharding configuration.
    #[serde(default)]
    pub sharding: ShardingConfig,

    /// Peer node addresses.
    /// Format: "node_id@endpoint_id:direct_addrs"
    #[serde(default)]
    pub peers: Vec<String>,

    /// Write batching configuration.
    ///
    /// When enabled, multiple write operations are batched together into
    /// a single Raft proposal to amortize fsync costs. This significantly
    /// increases throughput under concurrent load.
    ///
    /// Default: Some(BatchConfig::default()) - batching enabled
    #[serde(default = "default_batch_config")]
    pub batch_config: Option<crate::raft::BatchConfig>,

    /// DNS protocol server configuration.
    ///
    /// When enabled, the node runs a DNS server that resolves queries for
    /// configured zones from the Aspen DNS layer, with optional forwarding
    /// of unknown queries to upstream DNS servers.
    #[serde(default)]
    pub dns_server: DnsServerConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            data_dir: None,
            storage_backend: StorageBackend::default(),
            redb_path: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::default(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            docs: DocsConfig::default(),
            blobs: BlobConfig::default(),
            peer_sync: PeerSyncConfig::default(),
            sharding: ShardingConfig::default(),
            peers: vec![],
            batch_config: default_batch_config(),
            dns_server: DnsServerConfig::default(),
        }
    }
}

/// Relay server mode for Iroh connections.
///
/// Relays facilitate connections when direct peer-to-peer isn't possible (NAT traversal)
/// and help with hole-punching. They are stateless connection facilitators.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

impl IrohConfig {
    /// Apply security defaults based on configuration.
    ///
    /// This method enforces security-by-default policies:
    /// - When Pkarr DHT discovery is enabled, Raft authentication is automatically
    ///   enabled to prevent unauthorized nodes from joining the cluster.
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

/// iroh-docs configuration for real-time KV synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsConfig {
    /// Enable iroh-docs integration for real-time KV synchronization.
    ///
    /// When enabled, committed KV operations are exported to an iroh-docs
    /// namespace for CRDT-based replication to clients.
    ///
    /// Default: false (docs disabled).
    #[serde(default)]
    pub enabled: bool,

    /// Enable periodic full sync from state machine to docs.
    ///
    /// When enabled, the docs exporter periodically scans the entire
    /// state machine to detect and correct any drift between the
    /// committed state and the docs namespace.
    ///
    /// Default: true (background sync enabled).
    #[serde(default = "default_enable_background_sync")]
    pub enable_background_sync: bool,

    /// Interval for background sync in seconds.
    ///
    /// Only relevant when enable_background_sync is true.
    ///
    /// Default: 60 seconds.
    #[serde(default = "default_background_sync_interval_secs")]
    pub background_sync_interval_secs: u64,

    /// Use in-memory storage for iroh-docs instead of persistent storage.
    ///
    /// When true, the docs store uses in-memory storage (data lost on restart).
    /// When false, uses persistent storage in data_dir/docs/.
    ///
    /// Default: false (persistent storage).
    #[serde(default)]
    pub in_memory: bool,

    /// Hex-encoded namespace secret for the docs namespace.
    ///
    /// If not provided, a new namespace is created on first start and the
    /// secret is persisted to data_dir/docs/namespace_secret.
    ///
    /// 64 hex characters = 32 bytes.
    pub namespace_secret: Option<String>,

    /// Hex-encoded author secret for signing docs entries.
    ///
    /// If not provided, a new author is created on first start and the
    /// secret is persisted to data_dir/docs/author_secret.
    ///
    /// 64 hex characters = 32 bytes.
    pub author_secret: Option<String>,
}

impl Default for DocsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            enable_background_sync: default_enable_background_sync(),
            background_sync_interval_secs: default_background_sync_interval_secs(),
            in_memory: false,
            namespace_secret: None,
            author_secret: None,
        }
    }
}

fn default_enable_background_sync() -> bool {
    true
}

fn default_background_sync_interval_secs() -> u64 {
    60
}

/// iroh-blobs configuration for content-addressed storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobConfig {
    /// Enable iroh-blobs integration for content-addressed storage.
    ///
    /// When enabled, the node can store and retrieve blobs (large binary data)
    /// using content-addressed storage with automatic deduplication.
    ///
    /// Default: false (blobs disabled).
    #[serde(default)]
    pub enabled: bool,

    /// Enable automatic offloading of large KV values to blobs.
    ///
    /// When enabled, KV values larger than offload_threshold_bytes are
    /// automatically stored in the blob store, with a reference saved
    /// in the KV store. The original value is reconstructed on read.
    ///
    /// Default: true (auto-offload enabled).
    #[serde(default = "default_auto_offload")]
    pub auto_offload: bool,

    /// Threshold in bytes for automatic blob offloading.
    ///
    /// KV values larger than this size are stored as blobs when auto_offload is enabled.
    ///
    /// Default: 1048576 (1 MB).
    #[serde(default = "default_offload_threshold_bytes")]
    pub offload_threshold_bytes: u32,

    /// Interval for blob garbage collection in seconds.
    ///
    /// Untagged blobs are collected after the GC grace period expires.
    ///
    /// Default: 60 seconds.
    #[serde(default = "default_gc_interval_secs")]
    pub gc_interval_secs: u64,

    /// Grace period for blob garbage collection in seconds.
    ///
    /// Blobs must remain untagged for this duration before being collected.
    /// This prevents race conditions during concurrent tag updates.
    ///
    /// Default: 300 seconds (5 minutes).
    #[serde(default = "default_gc_grace_period_secs")]
    pub gc_grace_period_secs: u64,
}

impl Default for BlobConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_offload: default_auto_offload(),
            offload_threshold_bytes: default_offload_threshold_bytes(),
            gc_interval_secs: default_gc_interval_secs(),
            gc_grace_period_secs: default_gc_grace_period_secs(),
        }
    }
}

fn default_auto_offload() -> bool {
    true
}

fn default_offload_threshold_bytes() -> u32 {
    1_048_576 // 1 MB
}

fn default_gc_interval_secs() -> u64 {
    60
}

fn default_gc_grace_period_secs() -> u64 {
    300 // 5 minutes
}

/// Peer cluster synchronization configuration.
///
/// Controls cluster-to-cluster data synchronization via iroh-docs.
/// When enabled, this cluster can subscribe to other Aspen clusters
/// and receive their KV data with priority-based conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSyncConfig {
    /// Enable peer cluster synchronization.
    ///
    /// When enabled, this node can subscribe to other Aspen clusters
    /// and import their KV data via iroh-docs CRDT sync.
    ///
    /// Default: false (peer sync disabled).
    #[serde(default)]
    pub enabled: bool,

    /// Default priority for imported peer data.
    ///
    /// Lower values have higher priority. Local cluster data has priority 0.
    /// When conflicts occur, the entry with lower priority wins.
    ///
    /// Default: 100 (lower priority than local data).
    #[serde(default = "default_peer_sync_priority")]
    pub default_priority: u32,

    /// Maximum number of peer cluster subscriptions.
    ///
    /// Tiger Style: Bounded to prevent resource exhaustion.
    ///
    /// Default: 32.
    #[serde(default = "default_max_peer_subscriptions")]
    pub max_subscriptions: u32,

    /// Reconnection interval in seconds when peer connection is lost.
    ///
    /// Default: 30 seconds.
    #[serde(default = "default_peer_reconnect_interval_secs")]
    pub reconnect_interval_secs: u64,

    /// Maximum reconnection attempts before giving up.
    ///
    /// Set to 0 for unlimited retries.
    ///
    /// Default: 0 (unlimited retries).
    #[serde(default)]
    pub max_reconnect_attempts: u32,
}

impl Default for PeerSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_priority: default_peer_sync_priority(),
            max_subscriptions: default_max_peer_subscriptions(),
            reconnect_interval_secs: default_peer_reconnect_interval_secs(),
            max_reconnect_attempts: 0,
        }
    }
}

/// Horizontal sharding configuration.
///
/// Enables distributing data across multiple independent Raft clusters (shards)
/// for horizontal scaling. Each shard handles a subset of the key space
/// determined by consistent hashing.
///
/// # Architecture
///
/// ```text
/// ShardedKeyValueStore
///     |
///     +-- ShardRouter (consistent hashing)
///     |
///     +-- shards[0] -> RaftNode (shard-0/ directory)
///     +-- shards[1] -> RaftNode (shard-1/ directory)
///     +-- shards[2] -> RaftNode (shard-2/ directory)
///     ...
/// ```
///
/// # TOML Example
///
/// ```toml
/// [sharding]
/// enabled = true
/// num_shards = 4
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Enable horizontal sharding.
    ///
    /// When enabled, the node will create multiple RaftNode instances (one per shard)
    /// and route operations using consistent hashing.
    ///
    /// Default: false (single-node mode).
    #[serde(default)]
    pub enabled: bool,

    /// Number of shards to create.
    ///
    /// Must be between 1 and 256 (MAX_SHARDS).
    /// Each shard gets its own storage directory and Raft consensus group.
    ///
    /// Default: 4 shards.
    #[serde(default = "default_num_shards")]
    pub num_shards: u32,

    /// List of shard IDs this node should host.
    ///
    /// If empty, the node hosts all shards (0..num_shards).
    /// This allows selective shard placement for multi-node deployments.
    ///
    /// Default: empty (host all shards).
    #[serde(default)]
    pub local_shards: Vec<u32>,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            num_shards: default_num_shards(),
            local_shards: vec![],
        }
    }
}

fn default_num_shards() -> u32 {
    crate::sharding::DEFAULT_SHARDS
}

fn default_peer_sync_priority() -> u32 {
    100
}

fn default_max_peer_subscriptions() -> u32 {
    32
}

fn default_peer_reconnect_interval_secs() -> u64 {
    30
}

/// DNS protocol server configuration.
///
/// Enables local domain name resolution for Aspen nodes and services.
/// The DNS server listens on a configurable port (default 5353) and responds
/// to DNS queries for configured zones, forwarding unknown queries to upstream.
///
/// # Example TOML
///
/// ```toml
/// [dns_server]
/// enabled = true
/// bind_addr = "127.0.0.1:5353"
/// zones = ["aspen.local", "cluster.internal"]
/// upstreams = ["8.8.8.8:53", "8.8.4.4:53"]
/// forwarding_enabled = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServerConfig {
    /// Enable the DNS protocol server.
    ///
    /// When enabled, the node will start a DNS server that responds to
    /// queries for records stored in the Aspen DNS layer.
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// Bind address for DNS server (UDP and TCP).
    ///
    /// Use port 5353 for user-space operation (no root required).
    /// Use port 53 for standard DNS (requires root or CAP_NET_BIND_SERVICE).
    ///
    /// Default: "127.0.0.1:5353"
    #[serde(default = "default_dns_bind_addr")]
    pub bind_addr: SocketAddr,

    /// Zones to serve from Aspen DNS layer.
    ///
    /// Queries for these zones are resolved from local cache.
    /// Other queries are forwarded to upstream if forwarding_enabled is true.
    ///
    /// Tiger Style: Max 64 zones.
    ///
    /// Default: ["aspen.local"]
    #[serde(default = "default_dns_zones")]
    pub zones: Vec<String>,

    /// Upstream DNS servers for forwarding unknown queries.
    ///
    /// When forwarding_enabled is true, queries for domains not in
    /// configured zones are forwarded to these servers.
    ///
    /// Tiger Style: Max 8 upstream servers.
    ///
    /// Default: ["8.8.8.8:53", "8.8.4.4:53"]
    #[serde(default = "default_dns_upstreams")]
    pub upstreams: Vec<SocketAddr>,

    /// Forward queries for unknown domains to upstream.
    ///
    /// When true, queries for domains not in configured zones are
    /// forwarded to upstream DNS servers.
    /// When false, NXDOMAIN is returned for unknown domains.
    ///
    /// Default: true
    #[serde(default = "default_dns_forwarding")]
    pub forwarding_enabled: bool,
}

impl Default for DnsServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_addr: default_dns_bind_addr(),
            zones: default_dns_zones(),
            upstreams: default_dns_upstreams(),
            forwarding_enabled: default_dns_forwarding(),
        }
    }
}

fn default_dns_bind_addr() -> SocketAddr {
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 5353)
}

fn default_dns_zones() -> Vec<String> {
    vec!["aspen.local".to_string()]
}

fn default_dns_upstreams() -> Vec<SocketAddr> {
    vec![
        SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)), 53),
        SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 4, 4)), 53),
    ]
}

fn default_dns_forwarding() -> bool {
    true
}

/// Control-plane backend implementation.
///
/// Selects which implementation handles cluster consensus and coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ControlBackend {
    /// Deterministic in-memory implementation for testing.
    ///
    /// Uses an in-memory store without network communication, allowing
    /// for fast and reproducible tests.
    Deterministic,
    /// Production Raft-backed implementation.
    ///
    /// Uses openraft consensus with persistent storage and network
    /// communication via Iroh.
    #[default]
    RaftActor,
}

impl FromStr for ControlBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "deterministic" => Ok(ControlBackend::Deterministic),
            "raft_actor" | "raftactor" => Ok(ControlBackend::RaftActor),
            _ => Err(format!("invalid control backend: {}", s)),
        }
    }
}

impl NodeConfig {
    /// Load configuration from a TOML file.
    pub fn from_toml_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).context(ReadFileSnafu { path })?;
        toml::from_str(&content).context(ParseTomlSnafu { path })
    }

    /// Load configuration from environment variables.
    ///
    /// Environment variables follow the pattern ASPEN_<FIELD_NAME> (uppercase).
    /// For nested fields like iroh.secret_key, use ASPEN_IROH_SECRET_KEY.
    pub fn from_env() -> Self {
        Self {
            node_id: parse_env("ASPEN_NODE_ID").unwrap_or(0),
            data_dir: parse_env("ASPEN_DATA_DIR"),
            storage_backend: parse_env("ASPEN_STORAGE_BACKEND").unwrap_or_else(StorageBackend::default),
            redb_path: parse_env("ASPEN_REDB_PATH"),
            host: parse_env("ASPEN_HOST").unwrap_or_else(default_host),
            cookie: parse_env("ASPEN_COOKIE").unwrap_or_else(default_cookie),
            http_addr: parse_env("ASPEN_HTTP_ADDR").unwrap_or_else(default_http_addr),
            control_backend: parse_env("ASPEN_CONTROL_BACKEND").unwrap_or(ControlBackend::default()),
            heartbeat_interval_ms: parse_env("ASPEN_HEARTBEAT_INTERVAL_MS")
                .unwrap_or_else(default_heartbeat_interval_ms),
            election_timeout_min_ms: parse_env("ASPEN_ELECTION_TIMEOUT_MIN_MS")
                .unwrap_or_else(default_election_timeout_min_ms),
            election_timeout_max_ms: parse_env("ASPEN_ELECTION_TIMEOUT_MAX_MS")
                .unwrap_or_else(default_election_timeout_max_ms),
            iroh: IrohConfig {
                secret_key: parse_env("ASPEN_IROH_SECRET_KEY"),
                enable_gossip: parse_env("ASPEN_IROH_ENABLE_GOSSIP").unwrap_or_else(default_enable_gossip),
                gossip_ticket: parse_env("ASPEN_IROH_GOSSIP_TICKET"),
                enable_mdns: parse_env("ASPEN_IROH_ENABLE_MDNS").unwrap_or_else(default_enable_mdns),
                enable_dns_discovery: parse_env("ASPEN_IROH_ENABLE_DNS_DISCOVERY").unwrap_or(false),
                dns_discovery_url: parse_env("ASPEN_IROH_DNS_DISCOVERY_URL"),
                enable_pkarr: parse_env("ASPEN_IROH_ENABLE_PKARR").unwrap_or(false),
                enable_pkarr_dht: parse_env("ASPEN_IROH_ENABLE_PKARR_DHT").unwrap_or_else(default_enable_pkarr_dht),
                enable_pkarr_relay: parse_env("ASPEN_IROH_ENABLE_PKARR_RELAY")
                    .unwrap_or_else(default_enable_pkarr_relay),
                include_pkarr_direct_addresses: parse_env("ASPEN_IROH_INCLUDE_PKARR_DIRECT_ADDRESSES")
                    .unwrap_or_else(default_include_pkarr_direct_addresses),
                pkarr_republish_delay_secs: parse_env("ASPEN_IROH_PKARR_REPUBLISH_DELAY_SECS")
                    .unwrap_or_else(default_pkarr_republish_delay_secs),
                pkarr_relay_url: parse_env("ASPEN_IROH_PKARR_RELAY_URL"),
                relay_mode: parse_env("ASPEN_IROH_RELAY_MODE").unwrap_or_default(),
                relay_urls: parse_env_vec("ASPEN_IROH_RELAY_URLS"),
                enable_raft_auth: parse_env("ASPEN_IROH_ENABLE_RAFT_AUTH").unwrap_or(false),
            },
            docs: DocsConfig {
                enabled: parse_env("ASPEN_DOCS_ENABLED").unwrap_or(false),
                enable_background_sync: parse_env("ASPEN_DOCS_ENABLE_BACKGROUND_SYNC")
                    .unwrap_or_else(default_enable_background_sync),
                background_sync_interval_secs: parse_env("ASPEN_DOCS_BACKGROUND_SYNC_INTERVAL_SECS")
                    .unwrap_or_else(default_background_sync_interval_secs),
                in_memory: parse_env("ASPEN_DOCS_IN_MEMORY").unwrap_or(false),
                namespace_secret: parse_env("ASPEN_DOCS_NAMESPACE_SECRET"),
                author_secret: parse_env("ASPEN_DOCS_AUTHOR_SECRET"),
            },
            blobs: BlobConfig {
                enabled: parse_env("ASPEN_BLOBS_ENABLED").unwrap_or(false),
                auto_offload: parse_env("ASPEN_BLOBS_AUTO_OFFLOAD").unwrap_or_else(default_auto_offload),
                offload_threshold_bytes: parse_env("ASPEN_BLOBS_OFFLOAD_THRESHOLD_BYTES")
                    .unwrap_or_else(default_offload_threshold_bytes),
                gc_interval_secs: parse_env("ASPEN_BLOBS_GC_INTERVAL_SECS").unwrap_or_else(default_gc_interval_secs),
                gc_grace_period_secs: parse_env("ASPEN_BLOBS_GC_GRACE_PERIOD_SECS")
                    .unwrap_or_else(default_gc_grace_period_secs),
            },
            peer_sync: PeerSyncConfig {
                enabled: parse_env("ASPEN_PEER_SYNC_ENABLED").unwrap_or(false),
                default_priority: parse_env("ASPEN_PEER_SYNC_DEFAULT_PRIORITY")
                    .unwrap_or_else(default_peer_sync_priority),
                max_subscriptions: parse_env("ASPEN_PEER_SYNC_MAX_SUBSCRIPTIONS")
                    .unwrap_or_else(default_max_peer_subscriptions),
                reconnect_interval_secs: parse_env("ASPEN_PEER_SYNC_RECONNECT_INTERVAL_SECS")
                    .unwrap_or_else(default_peer_reconnect_interval_secs),
                max_reconnect_attempts: parse_env("ASPEN_PEER_SYNC_MAX_RECONNECT_ATTEMPTS").unwrap_or(0),
            },
            sharding: ShardingConfig {
                enabled: parse_env("ASPEN_SHARDING_ENABLED").unwrap_or(false),
                num_shards: parse_env("ASPEN_SHARDING_NUM_SHARDS").unwrap_or_else(default_num_shards),
                local_shards: parse_env_vec("ASPEN_SHARDING_LOCAL_SHARDS")
                    .into_iter()
                    .filter_map(|s| s.parse().ok())
                    .collect(),
            },
            peers: parse_env_vec("ASPEN_PEERS"),
            batch_config: default_batch_config(),
            dns_server: DnsServerConfig {
                enabled: parse_env("ASPEN_DNS_SERVER_ENABLED").unwrap_or(false),
                bind_addr: parse_env("ASPEN_DNS_SERVER_BIND_ADDR").unwrap_or_else(default_dns_bind_addr),
                zones: {
                    let zones = parse_env_vec("ASPEN_DNS_SERVER_ZONES");
                    if zones.is_empty() { default_dns_zones() } else { zones }
                },
                upstreams: {
                    let upstreams: Vec<SocketAddr> = parse_env_vec("ASPEN_DNS_SERVER_UPSTREAMS")
                        .into_iter()
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if upstreams.is_empty() {
                        default_dns_upstreams()
                    } else {
                        upstreams
                    }
                },
                forwarding_enabled: parse_env("ASPEN_DNS_SERVER_FORWARDING_ENABLED")
                    .unwrap_or_else(default_dns_forwarding),
            },
        }
    }

    /// Merge configuration from another source.
    ///
    /// Fields in `other` that are `Some` or non-default will override fields in `self`.
    /// This is used to implement the layered config precedence.
    pub fn merge(&mut self, other: Self) {
        if other.node_id != 0 {
            self.node_id = other.node_id;
        }
        if other.data_dir.is_some() {
            self.data_dir = other.data_dir;
        }
        if other.host != default_host() {
            self.host = other.host;
        }
        if other.cookie != default_cookie() {
            self.cookie = other.cookie;
        }
        if other.http_addr != default_http_addr() {
            self.http_addr = other.http_addr;
        }
        // Tiger Style: Only merge control_backend if explicitly set (non-default)
        // This preserves TOML settings when CLI args don't override them
        if other.control_backend != ControlBackend::default() {
            self.control_backend = other.control_backend;
        }
        // Tiger Style: Only merge storage_backend if explicitly set (non-default)
        // This preserves TOML settings when CLI args don't override them
        // Example: TOML sets `storage_backend = "inmemory"`, CLI doesn't override,
        // so InMemory is preserved instead of being overwritten by default Redb
        if other.storage_backend != StorageBackend::default() {
            self.storage_backend = other.storage_backend;
        }
        if other.redb_path.is_some() {
            self.redb_path = other.redb_path;
        }
        if other.heartbeat_interval_ms != default_heartbeat_interval_ms() {
            self.heartbeat_interval_ms = other.heartbeat_interval_ms;
        }
        if other.election_timeout_min_ms != default_election_timeout_min_ms() {
            self.election_timeout_min_ms = other.election_timeout_min_ms;
        }
        if other.election_timeout_max_ms != default_election_timeout_max_ms() {
            self.election_timeout_max_ms = other.election_timeout_max_ms;
        }
        if other.iroh.secret_key.is_some() {
            self.iroh.secret_key = other.iroh.secret_key;
        }
        if other.iroh.enable_gossip != default_enable_gossip() {
            self.iroh.enable_gossip = other.iroh.enable_gossip;
        }
        if other.iroh.gossip_ticket.is_some() {
            self.iroh.gossip_ticket = other.iroh.gossip_ticket;
        }
        if other.iroh.enable_mdns != default_enable_mdns() {
            self.iroh.enable_mdns = other.iroh.enable_mdns;
        }
        if other.iroh.enable_dns_discovery {
            self.iroh.enable_dns_discovery = other.iroh.enable_dns_discovery;
        }
        if other.iroh.dns_discovery_url.is_some() {
            self.iroh.dns_discovery_url = other.iroh.dns_discovery_url;
        }
        if other.iroh.enable_pkarr {
            self.iroh.enable_pkarr = other.iroh.enable_pkarr;
        }
        if other.iroh.enable_pkarr_dht != default_enable_pkarr_dht() {
            self.iroh.enable_pkarr_dht = other.iroh.enable_pkarr_dht;
        }
        if other.iroh.enable_pkarr_relay != default_enable_pkarr_relay() {
            self.iroh.enable_pkarr_relay = other.iroh.enable_pkarr_relay;
        }
        if other.iroh.include_pkarr_direct_addresses != default_include_pkarr_direct_addresses() {
            self.iroh.include_pkarr_direct_addresses = other.iroh.include_pkarr_direct_addresses;
        }
        if other.iroh.pkarr_republish_delay_secs != default_pkarr_republish_delay_secs() {
            self.iroh.pkarr_republish_delay_secs = other.iroh.pkarr_republish_delay_secs;
        }
        if other.iroh.pkarr_relay_url.is_some() {
            self.iroh.pkarr_relay_url = other.iroh.pkarr_relay_url;
        }
        if other.iroh.relay_mode != RelayMode::default() {
            self.iroh.relay_mode = other.iroh.relay_mode;
        }
        if !other.iroh.relay_urls.is_empty() {
            self.iroh.relay_urls = other.iroh.relay_urls;
        }
        if other.iroh.enable_raft_auth {
            self.iroh.enable_raft_auth = other.iroh.enable_raft_auth;
        }
        // Docs config merging
        if other.docs.enabled {
            self.docs.enabled = other.docs.enabled;
        }
        if other.docs.enable_background_sync != default_enable_background_sync() {
            self.docs.enable_background_sync = other.docs.enable_background_sync;
        }
        if other.docs.background_sync_interval_secs != default_background_sync_interval_secs() {
            self.docs.background_sync_interval_secs = other.docs.background_sync_interval_secs;
        }
        if other.docs.in_memory {
            self.docs.in_memory = other.docs.in_memory;
        }
        if other.docs.namespace_secret.is_some() {
            self.docs.namespace_secret = other.docs.namespace_secret;
        }
        if other.docs.author_secret.is_some() {
            self.docs.author_secret = other.docs.author_secret;
        }
        // Blobs config merging
        if other.blobs.enabled {
            self.blobs.enabled = other.blobs.enabled;
        }
        if other.blobs.auto_offload != default_auto_offload() {
            self.blobs.auto_offload = other.blobs.auto_offload;
        }
        if other.blobs.offload_threshold_bytes != default_offload_threshold_bytes() {
            self.blobs.offload_threshold_bytes = other.blobs.offload_threshold_bytes;
        }
        if other.blobs.gc_interval_secs != default_gc_interval_secs() {
            self.blobs.gc_interval_secs = other.blobs.gc_interval_secs;
        }
        if other.blobs.gc_grace_period_secs != default_gc_grace_period_secs() {
            self.blobs.gc_grace_period_secs = other.blobs.gc_grace_period_secs;
        }
        // Peer sync config merging
        if other.peer_sync.enabled {
            self.peer_sync.enabled = other.peer_sync.enabled;
        }
        if other.peer_sync.default_priority != default_peer_sync_priority() {
            self.peer_sync.default_priority = other.peer_sync.default_priority;
        }
        if other.peer_sync.max_subscriptions != default_max_peer_subscriptions() {
            self.peer_sync.max_subscriptions = other.peer_sync.max_subscriptions;
        }
        if other.peer_sync.reconnect_interval_secs != default_peer_reconnect_interval_secs() {
            self.peer_sync.reconnect_interval_secs = other.peer_sync.reconnect_interval_secs;
        }
        if other.peer_sync.max_reconnect_attempts != 0 {
            self.peer_sync.max_reconnect_attempts = other.peer_sync.max_reconnect_attempts;
        }
        // Sharding config merging
        if other.sharding.enabled {
            self.sharding.enabled = other.sharding.enabled;
        }
        if other.sharding.num_shards != default_num_shards() {
            self.sharding.num_shards = other.sharding.num_shards;
        }
        if !other.sharding.local_shards.is_empty() {
            self.sharding.local_shards = other.sharding.local_shards;
        }
        if !other.peers.is_empty() {
            self.peers = other.peers;
        }
        // DNS server config merging
        if other.dns_server.enabled {
            self.dns_server.enabled = other.dns_server.enabled;
        }
        if other.dns_server.bind_addr != default_dns_bind_addr() {
            self.dns_server.bind_addr = other.dns_server.bind_addr;
        }
        if other.dns_server.zones != default_dns_zones() {
            self.dns_server.zones = other.dns_server.zones;
        }
        if other.dns_server.upstreams != default_dns_upstreams() {
            self.dns_server.upstreams = other.dns_server.upstreams;
        }
        if other.dns_server.forwarding_enabled != default_dns_forwarding() {
            self.dns_server.forwarding_enabled = other.dns_server.forwarding_enabled;
        }
    }

    /// Apply security defaults based on configuration.
    ///
    /// This method enforces security-by-default policies across all config sections.
    /// Call this after loading and merging configuration, before using it.
    ///
    /// Current policies:
    /// - When Pkarr DHT discovery is enabled, Raft authentication is automatically
    ///   enabled to prevent unauthorized nodes from joining the cluster.
    ///
    /// Returns true if any settings were changed.
    pub fn apply_security_defaults(&mut self) -> bool {
        self.iroh.apply_security_defaults()
    }

    /// Validate configuration on startup.
    ///
    /// Performs Tiger Style validation with fail-fast semantics:
    /// - Hard errors for invalid configuration (returns Err)
    /// - Warnings for non-recommended settings (logs via tracing::warn!)
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        use tracing::warn;

        use crate::cluster::validation::check_disk_usage;
        use crate::cluster::validation::check_http_port;
        use crate::cluster::validation::check_raft_timing_sanity;
        use crate::cluster::validation::validate_cookie;
        use crate::cluster::validation::validate_cookie_safety;
        use crate::cluster::validation::validate_node_id;
        use crate::cluster::validation::validate_raft_timings;
        use crate::cluster::validation::validate_secret_key;

        // Validate core fields using extracted pure functions
        validate_node_id(self.node_id).map_err(|e| ConfigError::Validation { message: e.to_string() })?;

        validate_cookie(&self.cookie).map_err(|e| ConfigError::Validation { message: e.to_string() })?;

        // Reject unsafe default cookie (security-critical: prevents shared gossip topics)
        validate_cookie_safety(&self.cookie).map_err(|e| ConfigError::Validation { message: e.to_string() })?;

        validate_raft_timings(self.heartbeat_interval_ms, self.election_timeout_min_ms, self.election_timeout_max_ms)
            .map_err(|e| ConfigError::Validation { message: e.to_string() })?;

        validate_secret_key(self.iroh.secret_key.as_deref())
            .map_err(|e| ConfigError::Validation { message: e.to_string() })?;

        // Log sanity warnings using extracted pure function
        for warning in check_raft_timing_sanity(
            self.heartbeat_interval_ms,
            self.election_timeout_min_ms,
            self.election_timeout_max_ms,
        ) {
            warn!("{}", warning);
        }

        // File path validation (has I/O side effects, kept inline)
        if let Some(ref data_dir) = self.data_dir
            && let Some(parent) = data_dir.parent()
        {
            if !parent.exists() {
                return Err(ConfigError::Validation {
                    message: format!("data_dir parent directory does not exist: {}", parent.display()),
                });
            }

            // Check if data_dir exists or can be created
            if !data_dir.exists() {
                std::fs::create_dir_all(data_dir).map_err(|e| ConfigError::Validation {
                    message: format!("cannot create data_dir {}: {}", data_dir.display(), e),
                })?;
            }

            // Check disk space (warning only, not error)
            match crate::utils::check_disk_space(data_dir) {
                Ok(disk_space) => {
                    if let Some(warning) = check_disk_usage(disk_space.usage_percent) {
                        warn!(
                            data_dir = %data_dir.display(),
                            available_gb = disk_space.available_bytes / (1024 * 1024 * 1024),
                            "{}",
                            warning
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        data_dir = %data_dir.display(),
                        error = %e,
                        "could not check disk space (non-fatal)"
                    );
                }
            }
        }

        // Validate explicit storage paths if provided
        // Tiger Style: Fail fast on invalid paths before node startup
        for (name, path) in [("redb_path", &self.redb_path)] {
            if let Some(path) = path {
                // Validate parent directory exists or can be created
                if let Some(parent) = path.parent()
                    && !parent.as_os_str().is_empty()
                    && !parent.exists()
                {
                    // Try to create parent directory
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return Err(ConfigError::Validation {
                            message: format!(
                                "{} parent directory {} does not exist and cannot be created: {}",
                                name,
                                parent.display(),
                                e
                            ),
                        });
                    }
                }

                // Warn if path exists but is a directory (should be a file)
                if path.exists() && path.is_dir() {
                    return Err(ConfigError::Validation {
                        message: format!("{} path {} exists but is a directory, expected a file", name, path.display()),
                    });
                }
            }
        }

        // Network port validation (using extracted pure function)
        if let Some(warning) = check_http_port(self.http_addr.port()) {
            warn!(http_addr = %self.http_addr, "{}", warning);
        }

        // Sharding configuration validation
        if self.sharding.enabled {
            use crate::sharding::MAX_SHARDS;
            use crate::sharding::MIN_SHARDS;

            // Validate num_shards is within bounds
            if self.sharding.num_shards < MIN_SHARDS || self.sharding.num_shards > MAX_SHARDS {
                return Err(ConfigError::Validation {
                    message: format!(
                        "sharding.num_shards must be between {} and {}, got {}",
                        MIN_SHARDS, MAX_SHARDS, self.sharding.num_shards
                    ),
                });
            }

            // Validate local_shards are within range
            for &shard_id in &self.sharding.local_shards {
                if shard_id >= self.sharding.num_shards {
                    return Err(ConfigError::Validation {
                        message: format!(
                            "sharding.local_shards contains invalid shard_id {}, must be < num_shards ({})",
                            shard_id, self.sharding.num_shards
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the data directory, using the default if not specified.
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone().unwrap_or_else(|| PathBuf::from(format!("./data/node-{}", self.node_id)))
    }
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".into()
}

/// Default cookie value used as a marker.
/// When this exact value is detected, validation will warn and recommend a unique cookie.
/// This prevents multiple independent clusters from accidentally sharing gossip topics.
const DEFAULT_COOKIE_MARKER: &str = "aspen-cookie-UNSAFE-CHANGE-ME";

fn default_cookie() -> String {
    DEFAULT_COOKIE_MARKER.into()
}

fn default_http_addr() -> SocketAddr {
    // Tiger Style: Compile-time constant instead of runtime parsing
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 8080)
}

fn default_heartbeat_interval_ms() -> u64 {
    500
}

fn default_election_timeout_min_ms() -> u64 {
    1500
}

fn default_election_timeout_max_ms() -> u64 {
    3000
}

fn default_enable_gossip() -> bool {
    true
}

fn default_enable_mdns() -> bool {
    true
}

fn default_enable_pkarr_dht() -> bool {
    true // DHT enabled by default when pkarr is on
}

fn default_enable_pkarr_relay() -> bool {
    true // Relay enabled by default for fallback
}

fn default_include_pkarr_direct_addresses() -> bool {
    true // Include direct IPs by default
}

fn default_pkarr_republish_delay_secs() -> u64 {
    600 // 10 minutes default republish
}

fn default_batch_config() -> Option<crate::raft::BatchConfig> {
    Some(crate::raft::BatchConfig::default())
}

// Helper functions for parsing environment variables
fn parse_env<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok()?.parse().ok()
}

fn parse_env_vec(key: &str) -> Vec<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default()
}

/// Configuration loading and parsing errors.
#[derive(Debug, Snafu)]
pub enum ConfigError {
    /// Failed to read configuration file from disk.
    #[snafu(display("failed to read config file {}: {source}", path.display()))]
    ReadFile {
        /// Path to the configuration file that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to parse TOML configuration file.
    #[snafu(display("failed to parse TOML config file {}: {source}", path.display()))]
    ParseToml {
        /// Path to the TOML file that could not be parsed.
        path: PathBuf,
        /// TOML deserialization error.
        source: toml::de::Error,
    },

    /// Configuration validation failed.
    ///
    /// This error occurs when configuration values are invalid (e.g., node_id is zero,
    /// election timeout ordering is wrong, etc.).
    #[snafu(display("configuration validation failed: {message}"))]
    Validation {
        /// Human-readable description of the validation failure.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig {
            node_id: 1,
            cookie: "test-cookie".into(),
            ..Default::default()
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.data_dir(), PathBuf::from("./data/node-1"));
    }

    #[test]
    fn test_validation_node_id_zero() {
        let config = NodeConfig {
            node_id: 0,
            cookie: "test-cookie".into(),
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_election_timeout() {
        let config = NodeConfig {
            node_id: 1,
            election_timeout_min_ms: 3000,
            election_timeout_max_ms: 1500,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    #[allow(deprecated)]
    fn test_merge() {
        let mut base = NodeConfig {
            node_id: 1,
            control_backend: ControlBackend::RaftActor, // Default value
            storage_backend: crate::raft::storage::StorageBackend::Redb, // Default value
            ..Default::default()
        };

        // Override config uses non-default values for control_backend and storage_backend
        // to test that explicit non-default values override
        let override_config = NodeConfig {
            node_id: 2,
            data_dir: Some(PathBuf::from("/custom/data")),
            host: "192.168.1.1".into(),
            cookie: "custom-cookie".into(),
            http_addr: "0.0.0.0:9090".parse().unwrap(),
            control_backend: ControlBackend::Deterministic, // Non-default: should override
            heartbeat_interval_ms: 1000,
            election_timeout_min_ms: 2000,
            election_timeout_max_ms: 4000,
            iroh: IrohConfig {
                secret_key: Some("a".repeat(64)),
                enable_gossip: false,
                gossip_ticket: Some("test-ticket".into()),
                enable_mdns: false,
                enable_dns_discovery: true,
                dns_discovery_url: Some("https://dns.example.com".into()),
                enable_pkarr: true,
                enable_pkarr_dht: true,
                enable_pkarr_relay: true,
                include_pkarr_direct_addresses: true,
                pkarr_republish_delay_secs: 600,
                pkarr_relay_url: Some("https://pkarr.example.com".into()),
                relay_mode: RelayMode::Custom,
                relay_urls: vec!["https://relay1.example.com".into()],
                enable_raft_auth: false,
            },
            peers: vec!["peer1".into()],
            storage_backend: crate::raft::storage::StorageBackend::InMemory, // Non-default: should override
            redb_path: Some(PathBuf::from("/custom/node_shared.redb")),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.node_id, 2);
        assert_eq!(base.data_dir, Some(PathBuf::from("/custom/data")));
        assert_eq!(base.host, "192.168.1.1");
        assert_eq!(base.cookie, "custom-cookie");
        assert_eq!(base.http_addr, "0.0.0.0:9090".parse().unwrap());
        // Deterministic (non-default) should override RaftActor (default)
        assert_eq!(base.control_backend, ControlBackend::Deterministic);
        assert_eq!(base.heartbeat_interval_ms, 1000);
        assert_eq!(base.election_timeout_min_ms, 2000);
        assert_eq!(base.election_timeout_max_ms, 4000);
        assert_eq!(base.iroh.secret_key, Some("a".repeat(64)));
        assert!(!base.iroh.enable_gossip);
        assert_eq!(base.iroh.gossip_ticket, Some("test-ticket".into()));
        assert_eq!(base.peers, vec!["peer1"]);
        // InMemory (non-default) should override Redb (default)
        assert_eq!(base.storage_backend, crate::raft::storage::StorageBackend::InMemory);
    }

    #[test]
    fn test_merge_preserves_explicit_base_values() {
        // Test that default override values don't clobber explicit base values
        // This tests the layered config precedence fix
        let mut base = NodeConfig {
            node_id: 1,
            control_backend: ControlBackend::Deterministic, // Explicit non-default
            storage_backend: crate::raft::storage::StorageBackend::InMemory, // Explicit non-default
            ..Default::default()
        };

        // Override config uses DEFAULT values - should NOT override base
        let override_config = NodeConfig {
            node_id: 0,                                                  // Default: 0 doesn't override
            control_backend: ControlBackend::RaftActor,                  // Default: should NOT override
            storage_backend: crate::raft::storage::StorageBackend::Redb, // Default: should NOT override
            ..Default::default()
        };

        base.merge(override_config);

        // Original non-default values should be preserved (not clobbered by defaults)
        assert_eq!(base.node_id, 1); // Preserved (0 is "unset")
        assert_eq!(base.control_backend, ControlBackend::Deterministic); // Preserved
        assert_eq!(base.storage_backend, crate::raft::storage::StorageBackend::InMemory); // Preserved
    }
}
