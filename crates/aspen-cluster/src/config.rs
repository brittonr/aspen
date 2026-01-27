//! Cluster configuration types and validation.
//!
//! Defines configuration structures for Aspen cluster nodes, supporting multi-layer
//! configuration loading from environment variables, Nickel files, and command-line
//! arguments. Configuration precedence is: environment < Nickel < CLI args, ensuring
//! operator overrides always take priority.
//!
//! # Key Components
//!
//! - `NodeConfig`: Top-level node configuration (node_id, data_dir, addresses)
//! - `ControlBackend`: Control plane backend (Raft)
//! - `IrohConfig`: P2P networking configuration (endpoint addresses, tickets)
//! - `StorageBackend`: Log and state machine backend selection
//! - Configuration builders: Programmatic API for testing and deployment tools
//!
//! # Configuration Sources
//!
//! 1. Environment variables: `ASPEN_NODE_ID`, `ASPEN_DATA_DIR`, etc.
//! 2. Nickel configuration file: `--config /path/to/config.ncl`
//! 3. Command-line arguments: `--node-id 1`
//!
//! Precedence: CLI > Nickel > Environment (highest to lowest)
//!
//! # Tiger Style
//!
//! - Explicit types: u64 for node_id, SocketAddr for addresses (type-safe)
//! - Default values: Sensible defaults for optional fields (data_dir, timeouts)
//! - Validation: FromStr implementations fail fast on invalid input
//! - Serialization: Nickel format with contracts for validation
//! - Fixed limits: Bounded retry counts and timeouts
//! - Path handling: Absolute paths preferred, relative paths resolved early
//!
//! # Example Nickel Configuration
//!
//! ```nickel
//! {
//!   node_id = 1,
//!   data_dir = "./data/node-1",
//!   storage_backend = 'redb,
//!   control_backend = 'raft,
//!   cookie = "my-cluster-cookie",
//!
//!   iroh = {
//!     enable_gossip = true,
//!     enable_mdns = true,
//!   },
//! }
//! ```
//!
//! # Example Usage
//!
//! ```ignore
//! use aspen::cluster::config::NodeConfig;
//!
//! // Programmatic
//! let config = NodeConfig {
//!     node_id: 1,
//!     cookie: "my-cluster".to_string(),
//!     ..Default::default()
//! };
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use aspen_constants::MAX_CI_JOB_MEMORY_BYTES;
use aspen_constants::MAX_CONFIG_FILE_SIZE;
use aspen_core::utils::check_disk_space;
use aspen_raft::storage::StorageBackend;
use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;
use snafu::Snafu;

use crate::content_discovery::ContentDiscoveryConfig;
// SupervisionConfig removed - was legacy from actor-based architecture

/// Raft timing profile for different deployment scenarios.
///
/// These profiles adjust heartbeat interval and election timeouts for
/// different operational requirements. Faster profiles enable quicker
/// leader failover but may cause false elections in high-latency networks.
///
/// # Constraint
///
/// All profiles maintain: `heartbeat_interval < election_timeout_min / 3`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RaftTimingProfile {
    /// Default conservative settings for production deployments.
    ///
    /// - heartbeat: 500ms
    /// - election_min: 1500ms
    /// - election_max: 3000ms
    ///
    /// Good for: Cloud deployments, cross-region clusters, high-latency networks.
    #[default]
    Conservative,

    /// Balanced settings for most local/LAN deployments.
    ///
    /// - heartbeat: 100ms
    /// - election_min: 500ms
    /// - election_max: 1000ms
    ///
    /// Good for: LAN clusters, single-datacenter deployments.
    Balanced,

    /// Aggressive settings for fast failover in low-latency environments.
    ///
    /// - heartbeat: 30ms
    /// - election_min: 100ms
    /// - election_max: 200ms
    ///
    /// Good for: Testing, single-machine development, co-located nodes.
    /// Warning: May cause unnecessary elections in high-latency networks.
    Fast,
}

impl std::str::FromStr for RaftTimingProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "conservative" => Ok(Self::Conservative),
            "balanced" => Ok(Self::Balanced),
            "fast" => Ok(Self::Fast),
            _ => Err(format!("invalid raft timing profile: '{}' (expected: conservative, balanced, fast)", s)),
        }
    }
}

impl RaftTimingProfile {
    /// Get the heartbeat interval for this profile in milliseconds.
    pub fn heartbeat_interval_ms(&self) -> u64 {
        match self {
            Self::Conservative => 500,
            Self::Balanced => 100,
            Self::Fast => 30,
        }
    }

    /// Get the minimum election timeout for this profile in milliseconds.
    pub fn election_timeout_min_ms(&self) -> u64 {
        match self {
            Self::Conservative => 1500,
            Self::Balanced => 500,
            Self::Fast => 100,
        }
    }

    /// Get the maximum election timeout for this profile in milliseconds.
    pub fn election_timeout_max_ms(&self) -> u64 {
        match self {
            Self::Conservative => 3000,
            Self::Balanced => 1000,
            Self::Fast => 200,
        }
    }

    /// Apply this profile's timing values to a NodeConfig.
    pub fn apply_to(&self, config: &mut NodeConfig) {
        config.heartbeat_interval_ms = self.heartbeat_interval_ms();
        config.election_timeout_min_ms = self.election_timeout_min_ms();
        config.election_timeout_max_ms = self.election_timeout_max_ms();
    }
}

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

    // Legacy HTTP field removed - all APIs now use Iroh Client RPC via QUIC
    /// Control-plane implementation to use for this node.
    #[serde(default)]
    pub control_backend: ControlBackend,

    /// Raft timing profile for quick configuration.
    ///
    /// This provides pre-configured timing values for different deployment
    /// scenarios. When set, it overrides the individual timing fields.
    ///
    /// - `conservative`: 500ms heartbeat, 1.5-3s elections (default, for production)
    /// - `balanced`: 100ms heartbeat, 0.5-1s elections (for LAN)
    /// - `fast`: 30ms heartbeat, 100-200ms elections (for testing)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raft_timing_profile: Option<RaftTimingProfile>,

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

    /// Federation configuration for cross-cluster communication.
    #[serde(default)]
    pub federation: FederationConfig,

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
    pub batch_config: Option<aspen_raft::BatchConfig>,

    /// DNS protocol server configuration.
    ///
    /// When enabled, the node runs a DNS server that resolves queries for
    /// configured zones from the Aspen DNS layer, with optional forwarding
    /// of unknown queries to upstream DNS servers.
    #[serde(default)]
    pub dns_server: DnsServerConfig,

    /// Global content discovery configuration.
    ///
    /// When enabled, the node participates in the BitTorrent Mainline DHT
    /// to announce and discover blobs across clusters without direct federation.
    #[serde(default)]
    pub content_discovery: ContentDiscoveryConfig,

    /// Worker pool configuration for distributed job execution.
    ///
    /// When enabled, this node participates in the distributed job queue
    /// by running workers that process jobs based on affinity and capabilities.
    #[serde(default)]
    pub worker: WorkerConfig,

    /// Event hook system configuration.
    ///
    /// When enabled, KV operations and cluster events trigger registered handlers.
    /// Handlers can be in-process (async closures), shell commands, or cross-cluster
    /// forwarding. Supports NATS-style topic wildcards for flexible routing.
    #[serde(default)]
    pub hooks: aspen_hooks::HooksConfig,

    /// CI/CD pipeline configuration.
    ///
    /// When enabled, the node can orchestrate pipeline execution using
    /// Nickel-based configuration from `.aspen/ci.ncl` files.
    #[serde(default)]
    pub ci: CiConfig,

    /// Nix binary cache configuration.
    ///
    /// When enabled, the node serves Nix store paths via HTTP/3 over Iroh QUIC.
    #[serde(default)]
    pub nix_cache: NixCacheConfig,

    /// SNIX content-addressed storage configuration.
    ///
    /// When enabled, Nix build artifacts are stored using decomposed
    /// content-addressed format for efficient deduplication and P2P distribution.
    #[serde(default)]
    pub snix: SnixConfig,

    /// Forge decentralized git configuration.
    ///
    /// Controls gossip-based announcements for repository synchronization
    /// and automatic CI triggering.
    #[serde(default)]
    pub forge: ForgeConfig,

    /// Secrets management configuration (SOPS-based).
    ///
    /// When enabled, the node loads secrets from SOPS-encrypted files at startup.
    /// Supports loading trusted root keys, token signing keys, and pre-built tokens.
    ///
    /// Requires the `secrets` feature.
    #[cfg(feature = "secrets")]
    #[serde(default)]
    pub secrets: aspen_secrets::SecretsConfig,
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
            control_backend: ControlBackend::default(),
            raft_timing_profile: None,
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            docs: DocsConfig::default(),
            blobs: BlobConfig::default(),
            peer_sync: PeerSyncConfig::default(),
            federation: FederationConfig::default(),
            sharding: ShardingConfig::default(),
            peers: vec![],
            batch_config: default_batch_config(),
            dns_server: DnsServerConfig::default(),
            content_discovery: ContentDiscoveryConfig::default(),
            worker: WorkerConfig::default(),
            hooks: aspen_hooks::HooksConfig::default(),
            ci: CiConfig::default(),
            nix_cache: NixCacheConfig::default(),
            snix: SnixConfig::default(),
            forge: ForgeConfig::default(),
            #[cfg(feature = "secrets")]
            secrets: aspen_secrets::SecretsConfig::default(),
        }
    }
}

impl NodeConfig {
    /// Apply the raft timing profile if set.
    ///
    /// This should be called after loading configuration to ensure the
    /// profile overrides the individual timing fields.
    pub fn apply_raft_timing_profile(&mut self) {
        if let Some(profile) = self.raft_timing_profile {
            profile.apply_to(self);
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

/// iroh-docs configuration for real-time KV synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsConfig {
    /// Enable iroh-docs integration for real-time KV synchronization.
    ///
    /// When enabled, committed KV operations are exported to an iroh-docs
    /// namespace for CRDT-based replication to clients.
    ///
    /// Default: true (docs enabled).
    #[serde(default = "default_docs_enabled")]
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
            enabled: default_docs_enabled(),
            enable_background_sync: default_enable_background_sync(),
            background_sync_interval_secs: default_background_sync_interval_secs(),
            in_memory: false,
            namespace_secret: None,
            author_secret: None,
        }
    }
}

fn default_docs_enabled() -> bool {
    true
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
    /// Default: true (blobs enabled).
    #[serde(default = "default_blobs_enabled")]
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

    // === Replication Configuration ===
    /// Default replication factor for new blobs.
    ///
    /// Specifies how many copies of each blob should be maintained across the cluster.
    /// A replication factor of 3 means the blob will be stored on 3 different nodes.
    ///
    /// Tiger Style: Bounded to MAX_REPLICATION_FACTOR (7).
    ///
    /// Default: 1 (no replication, single copy).
    #[serde(default = "default_replication_factor")]
    pub replication_factor: u32,

    /// Minimum replicas required before a write is considered durable.
    ///
    /// When `enable_quorum_writes` is true, blob writes block until at least
    /// this many replicas have confirmed storage. Must be <= replication_factor.
    ///
    /// For strong durability: min_replicas = (replication_factor / 2) + 1
    ///
    /// Default: 1.
    #[serde(default = "default_min_replicas")]
    pub min_replicas: u32,

    /// Maximum replication factor allowed.
    ///
    /// Tiger Style: Upper bound to prevent excessive replication.
    ///
    /// Default: 5.
    #[serde(default = "default_max_replicas")]
    pub max_replicas: u32,

    /// How often the repair service checks for under-replicated blobs (seconds).
    ///
    /// The repair service scans blob metadata periodically and triggers
    /// replication for blobs with fewer than min_replicas copies.
    ///
    /// Default: 60 seconds.
    #[serde(default = "default_repair_interval_secs")]
    pub repair_interval_secs: u64,

    /// Grace period before repairing under-replicated blobs (seconds).
    ///
    /// When a node goes offline, wait this long before triggering repair.
    /// This avoids unnecessary replication for transient failures.
    ///
    /// Default: 300 seconds (5 minutes).
    #[serde(default = "default_repair_delay_secs")]
    pub repair_delay_secs: u64,

    /// Enable quorum writes (synchronous replication).
    ///
    /// When enabled, blob writes block until `min_replicas` nodes have
    /// confirmed storage. This provides stronger durability guarantees
    /// at the cost of higher write latency.
    ///
    /// When disabled (default), replication happens asynchronously in the
    /// background. The write returns immediately after local storage.
    ///
    /// Default: false (async replication).
    #[serde(default)]
    pub enable_quorum_writes: bool,

    /// Node tag key for failure domain spreading.
    ///
    /// When set, the placement algorithm spreads replicas across different
    /// failure domains. For example, if set to "rack", replicas will be
    /// placed on nodes with different "rack" tags.
    ///
    /// Requires nodes to have the specified tag in their WorkerConfig.tags.
    ///
    /// Default: None (no failure domain awareness).
    #[serde(default)]
    pub failure_domain_key: Option<String>,

    /// Enable automatic replication of blobs.
    ///
    /// When enabled, the replication manager automatically replicates
    /// newly added blobs to achieve the configured replication factor.
    ///
    /// Default: false (manual replication only).
    #[serde(default)]
    pub enable_auto_replication: bool,
}

impl Default for BlobConfig {
    fn default() -> Self {
        Self {
            enabled: default_blobs_enabled(),
            auto_offload: default_auto_offload(),
            offload_threshold_bytes: default_offload_threshold_bytes(),
            gc_interval_secs: default_gc_interval_secs(),
            gc_grace_period_secs: default_gc_grace_period_secs(),
            // Replication defaults
            replication_factor: default_replication_factor(),
            min_replicas: default_min_replicas(),
            max_replicas: default_max_replicas(),
            repair_interval_secs: default_repair_interval_secs(),
            repair_delay_secs: default_repair_delay_secs(),
            enable_quorum_writes: false,
            failure_domain_key: None,
            enable_auto_replication: false,
        }
    }
}

fn default_blobs_enabled() -> bool {
    true
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

fn default_replication_factor() -> u32 {
    1 // No replication by default (single copy)
}

fn default_min_replicas() -> u32 {
    1 // At least one replica (the local one)
}

fn default_max_replicas() -> u32 {
    5 // Tiger Style: Upper bound on replication
}

fn default_repair_interval_secs() -> u64 {
    60 // Check every minute
}

fn default_repair_delay_secs() -> u64 {
    300 // 5 minute grace period before repair
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
    aspen_sharding::DEFAULT_SHARDS
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

/// Federation configuration for cross-cluster communication.
///
/// Enables independent Aspen clusters to discover each other, share content,
/// and synchronize resources across organizational boundaries.
///
/// # Architecture
///
/// - **Cluster Identity**: Each cluster has a stable Ed25519 keypair
/// - **DHT Discovery**: Clusters discover each other via BitTorrent Mainline DHT
/// - **Pull-Based Sync**: Cross-cluster sync is eventual consistent
///
/// # Example TOML
///
/// ```toml
/// [federation]
/// enabled = true
/// cluster_name = "my-organization"
/// # cluster_key is auto-generated if not specified
///
/// # Optional: explicit trusted clusters
/// trusted_clusters = [
///     "abc123def456...",  # public key of trusted cluster
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Enable federation support.
    ///
    /// When enabled, this cluster can participate in cross-cluster
    /// discovery and synchronization.
    ///
    /// Default: false (federation disabled).
    #[serde(default)]
    pub enabled: bool,

    /// Human-readable cluster name.
    ///
    /// Used in announcements and for identification in logs.
    /// Tiger Style: Max 128 characters.
    ///
    /// Default: hostname or "aspen-cluster"
    #[serde(default = "default_federation_cluster_name")]
    pub cluster_name: String,

    /// Hex-encoded cluster secret key (64 hex chars = 32 bytes).
    ///
    /// If not provided, a new key is generated on first start and
    /// stored in data_dir/federation/cluster_key.
    ///
    /// The cluster key should be the same across all nodes in the cluster.
    pub cluster_key: Option<String>,

    /// Path to file containing the cluster secret key.
    ///
    /// Alternative to inline cluster_key. Key is read from this file.
    /// File should contain 64 hex characters (32 bytes).
    pub cluster_key_path: Option<std::path::PathBuf>,

    /// Enable DHT-based cluster discovery.
    ///
    /// When enabled, the cluster announces itself to the BitTorrent
    /// Mainline DHT and discovers other federated clusters.
    ///
    /// Default: true (when federation is enabled)
    #[serde(default = "default_federation_dht_discovery")]
    pub enable_dht_discovery: bool,

    /// Enable gossip-based federation announcements.
    ///
    /// When enabled, the cluster participates in a global gossip topic
    /// for faster discovery of nearby federated clusters.
    ///
    /// Default: true (when federation is enabled)
    #[serde(default = "default_federation_gossip")]
    pub enable_gossip: bool,

    /// List of trusted cluster public keys.
    ///
    /// For resources with AllowList federation mode, only clusters
    /// in this list can sync. For resources with Public mode, this
    /// list is informational only.
    ///
    /// Tiger Style: Max 256 entries.
    #[serde(default)]
    pub trusted_clusters: Vec<String>,

    /// Interval for DHT announcements in seconds.
    ///
    /// How often to re-announce the cluster to the DHT to maintain
    /// discoverability.
    ///
    /// Default: 1800 seconds (30 minutes)
    #[serde(default = "default_federation_announce_interval_secs")]
    pub announce_interval_secs: u64,

    /// Maximum number of federated peers to track.
    ///
    /// Tiger Style: Bounded to prevent resource exhaustion.
    ///
    /// Default: 256
    #[serde(default = "default_federation_max_peers")]
    pub max_peers: u32,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cluster_name: default_federation_cluster_name(),
            cluster_key: None,
            cluster_key_path: None,
            enable_dht_discovery: default_federation_dht_discovery(),
            enable_gossip: default_federation_gossip(),
            trusted_clusters: Vec::new(),
            announce_interval_secs: default_federation_announce_interval_secs(),
            max_peers: default_federation_max_peers(),
        }
    }
}

fn default_federation_cluster_name() -> String {
    "aspen-cluster".to_string()
}

fn default_federation_dht_discovery() -> bool {
    true
}

fn default_federation_gossip() -> bool {
    true
}

fn default_federation_announce_interval_secs() -> u64 {
    1800 // 30 minutes
}

fn default_federation_max_peers() -> u32 {
    256
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

/// Worker pool configuration for distributed job execution.
///
/// Configures how this node participates in the distributed job queue system.
/// Workers can be specialized by job type, tagged with capabilities, and
/// configured with resource limits.
///
/// # Tiger Style
///
/// - Fixed limits: Max workers, concurrent jobs bounded
/// - Explicit types: Tags and job types as Vec<String>
/// - Sensible defaults: CPU cores for worker count
///
/// # Example
///
/// ```toml
/// [worker]
/// enabled = true
/// worker_count = 4
/// max_concurrent_jobs = 10
/// job_types = ["process_data", "ml_inference"]
/// tags = ["gpu", "high_memory"]
/// prefer_local = true
/// data_locality_weight = 0.8
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Enable workers on this node.
    ///
    /// When enabled, the node starts a worker pool to process jobs
    /// from the distributed queue.
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// Number of workers to start.
    ///
    /// Each worker can process one job at a time. More workers allow
    /// parallel job execution but consume more resources.
    ///
    /// Tiger Style: Max 64 workers per node.
    ///
    /// Default: Number of CPU cores (capped at 8)
    #[serde(default = "default_worker_count")]
    pub worker_count: usize,

    /// Maximum concurrent jobs per worker.
    ///
    /// Limits how many jobs a single worker can execute in parallel.
    /// Usually set to 1 for CPU-bound work, higher for I/O-bound.
    ///
    /// Tiger Style: Max 100 concurrent jobs per worker.
    ///
    /// Default: 1
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,

    /// Job types this node can handle.
    ///
    /// Empty means the node can handle any job type.
    /// When specified, only jobs matching these types are accepted.
    ///
    /// Tiger Style: Max 32 job types per node.
    ///
    /// Default: [] (handle all types)
    #[serde(default)]
    pub job_types: Vec<String>,

    /// Node capability tags.
    ///
    /// Tags describe node capabilities (e.g., "gpu", "ssd", "high_memory").
    /// Jobs can request specific tags for placement affinity.
    ///
    /// Tiger Style: Max 16 tags per node.
    ///
    /// Default: []
    #[serde(default)]
    pub tags: Vec<String>,

    /// Prefer executing jobs with local data.
    ///
    /// When true, jobs are preferentially routed to nodes that have
    /// the required data (iroh-blobs) locally available.
    ///
    /// Default: true
    #[serde(default = "default_prefer_local")]
    pub prefer_local: bool,

    /// Weight for data locality in job placement (0.0 to 1.0).
    ///
    /// Higher values prioritize data locality over other factors
    /// like load balancing or network proximity.
    ///
    /// Default: 0.7
    #[serde(default = "default_data_locality_weight")]
    pub data_locality_weight: f32,

    /// Poll interval for checking job queue (milliseconds).
    ///
    /// How often workers check for new jobs when idle.
    /// Lower values reduce latency but increase load.
    ///
    /// Tiger Style: Min 100ms, Max 60000ms.
    ///
    /// Default: 1000 (1 second)
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,

    /// Visibility timeout for dequeued jobs (seconds).
    ///
    /// How long a job remains invisible to other workers after being
    /// dequeued. Should be longer than typical job execution time.
    ///
    /// Tiger Style: Max 3600 seconds (1 hour).
    ///
    /// Default: 300 (5 minutes)
    #[serde(default = "default_visibility_timeout_secs")]
    pub visibility_timeout_secs: u64,

    /// Heartbeat interval for worker health checks (milliseconds).
    ///
    /// How often workers report their status to the cluster.
    ///
    /// Default: 5000 (5 seconds)
    #[serde(default = "default_worker_heartbeat_ms")]
    pub heartbeat_interval_ms: u64,

    /// Shutdown timeout for graceful worker termination (milliseconds).
    ///
    /// Maximum time to wait for workers to finish current jobs
    /// during shutdown.
    ///
    /// Default: 30000 (30 seconds)
    #[serde(default = "default_shutdown_timeout_ms")]
    pub shutdown_timeout_ms: u64,

    /// Enable distributed worker coordination.
    ///
    /// When enabled, workers coordinate across nodes for load balancing,
    /// work stealing, and failover.
    ///
    /// Default: false
    #[serde(default)]
    pub enable_distributed: bool,

    /// Enable work stealing from overloaded nodes.
    ///
    /// When enabled, idle workers can steal jobs from overloaded nodes
    /// to improve cluster-wide load balancing.
    ///
    /// Default: None (uses distributed coordinator default)
    #[serde(default)]
    pub enable_work_stealing: Option<bool>,

    /// Load balancing strategy for distributed coordination.
    ///
    /// Options: "round_robin", "least_loaded", "affinity", "consistent_hash"
    ///
    /// Default: None (uses distributed coordinator default)
    #[serde(default)]
    pub load_balancing_strategy: Option<String>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            worker_count: default_worker_count(),
            max_concurrent_jobs: default_max_concurrent_jobs(),
            job_types: vec![],
            tags: vec![],
            prefer_local: default_prefer_local(),
            data_locality_weight: default_data_locality_weight(),
            poll_interval_ms: default_poll_interval_ms(),
            visibility_timeout_secs: default_visibility_timeout_secs(),
            heartbeat_interval_ms: default_worker_heartbeat_ms(),
            shutdown_timeout_ms: default_shutdown_timeout_ms(),
            enable_distributed: false,
            enable_work_stealing: None,
            load_balancing_strategy: None,
        }
    }
}

fn default_worker_count() -> usize {
    std::cmp::min(num_cpus::get(), 8)
}

fn default_max_concurrent_jobs() -> usize {
    1
}

fn default_prefer_local() -> bool {
    true
}

fn default_data_locality_weight() -> f32 {
    0.7
}

fn default_poll_interval_ms() -> u64 {
    1000
}

fn default_visibility_timeout_secs() -> u64 {
    300
}

fn default_worker_heartbeat_ms() -> u64 {
    5000
}

fn default_shutdown_timeout_ms() -> u64 {
    30000
}

/// CI/CD pipeline configuration.
///
/// Configures the CI/CD system that executes pipelines defined in
/// `.aspen/ci.ncl` files when triggered by ref updates or manual invocation.
///
/// The CI system uses the aspen-jobs infrastructure for distributed
/// pipeline execution with Nickel-based type-safe configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CiConfig {
    /// Enable CI/CD orchestration on this node.
    ///
    /// When enabled, the node can:
    /// - Accept pipeline trigger requests
    /// - Execute pipeline jobs via the job system
    /// - Track pipeline run status
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// Enable automatic CI triggering on ref updates.
    ///
    /// When enabled, the trigger service watches for forge gossip
    /// events and automatically triggers CI for repositories that
    /// have `.aspen/ci.ncl` configurations.
    ///
    /// Default: false (manual triggering only)
    #[serde(default)]
    pub auto_trigger: bool,

    /// Maximum concurrent pipeline runs per repository.
    ///
    /// Limits how many pipelines can run simultaneously for a single
    /// repository. Older runs are queued or rejected based on strategy.
    ///
    /// Tiger Style: Max 10 concurrent runs per repo.
    ///
    /// Default: 3
    #[serde(default = "default_ci_max_concurrent_runs")]
    pub max_concurrent_runs: usize,

    /// Default pipeline timeout in seconds.
    ///
    /// Maximum duration for a complete pipeline run. Individual job
    /// timeouts can override this for specific stages.
    ///
    /// Tiger Style: Max 86400 seconds (24 hours).
    ///
    /// Default: 3600 (1 hour)
    #[serde(default = "default_ci_pipeline_timeout_secs")]
    pub pipeline_timeout_secs: u64,

    /// Repository IDs to automatically watch for CI triggers.
    ///
    /// When auto_trigger is enabled, the TriggerService will watch these
    /// repositories for ref updates and automatically start CI pipelines.
    ///
    /// Each entry is a hex-encoded 32-byte repository ID.
    ///
    /// Tiger Style: Max 100 watched repositories.
    ///
    /// Environment variable: `ASPEN_CI_WATCHED_REPOS` (comma-separated hex IDs)
    ///
    /// Default: empty (no repos watched initially)
    #[serde(default)]
    pub watched_repos: Vec<String>,

    /// Enable distributed execution for CI jobs.
    ///
    /// When enabled, CI jobs are distributed across all cluster nodes
    /// instead of running only on the node that triggered the pipeline.
    /// This prevents resource exhaustion on the leader node during
    /// intensive test runs.
    ///
    /// Requires `worker.enable_distributed = true` to be effective.
    ///
    /// Default: true (recommended for production clusters)
    #[serde(default = "default_ci_distributed_execution")]
    pub distributed_execution: bool,

    /// Avoid scheduling CI jobs on the Raft leader node.
    ///
    /// When enabled, CI jobs will prefer follower nodes to prevent
    /// resource contention with Raft consensus operations. This helps
    /// maintain cluster stability during intensive CI workloads.
    ///
    /// Only effective when `distributed_execution` is also true.
    ///
    /// Default: true (recommended for stability)
    #[serde(default = "default_ci_avoid_leader")]
    pub avoid_leader: bool,

    /// Enable cgroup-based resource isolation for CI jobs.
    ///
    /// When enabled, CI job processes are placed in cgroups with memory
    /// and CPU limits to prevent resource exhaustion. Requires cgroups v2
    /// to be available on the system.
    ///
    /// Default: true (falls back to no limits if cgroups unavailable)
    #[serde(default = "default_ci_resource_isolation")]
    pub resource_isolation: bool,

    /// Maximum memory per CI job in bytes.
    ///
    /// Hard limit on memory usage for individual CI jobs. Jobs exceeding
    /// this limit will be OOM killed by the cgroup.
    ///
    /// Default: 4 GB (4294967296 bytes)
    #[serde(default = "default_ci_max_memory_bytes")]
    pub max_job_memory_bytes: u64,
}

fn default_ci_max_concurrent_runs() -> usize {
    3
}

fn default_ci_pipeline_timeout_secs() -> u64 {
    3600
}

fn default_ci_distributed_execution() -> bool {
    true
}

fn default_ci_avoid_leader() -> bool {
    true
}

fn default_ci_resource_isolation() -> bool {
    true
}

fn default_ci_max_memory_bytes() -> u64 {
    MAX_CI_JOB_MEMORY_BYTES
}

// =============================================================================
// Nix Cache Gateway Configuration
// =============================================================================

/// Nix binary cache configuration.
///
/// Enables serving Nix store paths via HTTP/3 over Iroh QUIC. Clients connect
/// using the `iroh+h3` ALPN and can fetch NARs, narinfo files, and perform
/// cache queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixCacheConfig {
    /// Enable the Nix binary cache HTTP/3 gateway.
    ///
    /// When enabled, the node serves Nix store paths via the Nix binary cache
    /// protocol over HTTP/3 (using Iroh's QUIC transport).
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// Nix store directory path.
    ///
    /// The local Nix store directory to serve. This is used for cache metadata.
    ///
    /// Default: "/nix/store"
    #[serde(default = "default_nix_store_dir")]
    pub store_dir: PathBuf,

    /// Cache priority for substitution (lower = preferred).
    ///
    /// When clients query multiple caches, lower priority values are tried first.
    /// cache.nixos.org uses priority 40, so values < 40 will be preferred.
    ///
    /// Default: 30
    #[serde(default = "default_nix_cache_priority")]
    pub priority: u32,

    /// Enable mass query support for efficient batch lookups.
    ///
    /// When enabled, clients can query multiple store paths in a single request.
    /// This significantly improves performance for large dependency trees.
    ///
    /// Default: true
    #[serde(default = "default_want_mass_query")]
    pub want_mass_query: bool,

    /// Optional cache name for signing.
    ///
    /// When set, NARs are signed with the corresponding key from the secrets
    /// manager. The signing key must be loaded via the `secrets` feature.
    ///
    /// Example: "cache.example.com-1"
    pub cache_name: Option<String>,

    /// Name of the Transit signing key for narinfo signatures.
    ///
    /// When both `cache_name` and `signing_key_name` are set, narinfo files
    /// are signed using the Ed25519 key stored in the Transit secrets engine.
    /// The key must exist and be accessible via the node's secrets configuration.
    ///
    /// Example: "nix-cache-signing-key"
    pub signing_key_name: Option<String>,

    /// Transit mount path for the signing key.
    ///
    /// Specifies the Transit secrets engine mount where the signing key is stored.
    /// If not specified, uses the default "nix-cache" mount.
    ///
    /// Example: "transit" or "nix-cache"
    #[serde(default = "default_nix_cache_transit_mount")]
    pub transit_mount: String,
}

impl Default for NixCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            store_dir: default_nix_store_dir(),
            priority: default_nix_cache_priority(),
            want_mass_query: default_want_mass_query(),
            cache_name: None,
            signing_key_name: None,
            transit_mount: default_nix_cache_transit_mount(),
        }
    }
}

fn default_nix_store_dir() -> PathBuf {
    PathBuf::from("/nix/store")
}

fn default_nix_cache_priority() -> u32 {
    30
}

fn default_want_mass_query() -> bool {
    true
}

fn default_nix_cache_transit_mount() -> String {
    "nix-cache".to_string()
}

// =============================================================================
// SNIX Content-Addressed Storage Configuration
// =============================================================================

/// SNIX content-addressed storage configuration.
///
/// SNIX provides decomposed content-addressed storage for Nix artifacts:
/// - Blobs: Raw content chunks stored in iroh-blobs
/// - Directories: Merkle tree nodes stored in Raft KV
/// - PathInfo: Nix store path metadata stored in Raft KV
///
/// This enables efficient deduplication and P2P distribution of build artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixConfig {
    /// Enable SNIX storage layer.
    ///
    /// When enabled, Nix build artifacts are stored using SNIX's decomposed
    /// content-addressed format instead of monolithic NAR archives.
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// KV key prefix for directory metadata.
    ///
    /// Directory nodes (Merkle tree structure) are stored under this prefix.
    ///
    /// Default: "_snix:dir:"
    #[serde(default = "default_snix_dir_prefix")]
    pub directory_prefix: String,

    /// KV key prefix for PathInfo metadata.
    ///
    /// Nix store path metadata (NAR hash, size, references) stored under this prefix.
    ///
    /// Default: "_snix:pathinfo:"
    #[serde(default = "default_snix_pathinfo_prefix")]
    pub pathinfo_prefix: String,

    /// Enable automatic migration from legacy NAR storage.
    ///
    /// When enabled, existing NAR archives in iroh-blobs are automatically
    /// decomposed into SNIX format during background migration.
    ///
    /// Default: false
    #[serde(default)]
    pub migration_enabled: bool,

    /// Number of concurrent migration workers.
    ///
    /// Controls parallelism for background migration from legacy NAR storage.
    /// Higher values speed up migration but increase resource usage.
    ///
    /// Tiger Style: Max 16 workers.
    ///
    /// Default: 4
    #[serde(default = "default_migration_workers")]
    pub migration_workers: u32,
}

impl Default for SnixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            directory_prefix: default_snix_dir_prefix(),
            pathinfo_prefix: default_snix_pathinfo_prefix(),
            migration_enabled: false,
            migration_workers: default_migration_workers(),
        }
    }
}

fn default_snix_dir_prefix() -> String {
    "_snix:dir:".to_string()
}

fn default_snix_pathinfo_prefix() -> String {
    "_snix:pathinfo:".to_string()
}

fn default_migration_workers() -> u32 {
    4
}

// =============================================================================
// Forge Configuration
// =============================================================================

/// Forge decentralized git configuration.
///
/// Controls the Forge subsystem which provides decentralized Git hosting
/// via iroh-blobs for object storage and Raft KV for ref storage.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgeConfig {
    /// Enable Forge gossip announcements for ref updates.
    ///
    /// When enabled, ref updates are broadcast via iroh-gossip to other nodes.
    /// This enables automatic CI triggering and repository synchronization.
    ///
    /// Requires the `forge` feature and `iroh.enable_gossip = true`.
    ///
    /// Default: false
    #[serde(default)]
    pub enable_gossip: bool,
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
    Raft,
}

impl FromStr for ControlBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "deterministic" => Ok(ControlBackend::Deterministic),
            "raft" | "raft_actor" | "raftactor" => Ok(ControlBackend::Raft),
            _ => Err(format!("invalid control backend: {}", s)),
        }
    }
}

impl NodeConfig {
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
            control_backend: parse_env("ASPEN_CONTROL_BACKEND").unwrap_or(ControlBackend::default()),
            raft_timing_profile: parse_env("ASPEN_RAFT_TIMING_PROFILE"),
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
                bind_port: parse_env("ASPEN_IROH_BIND_PORT").unwrap_or(0),
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
                enabled: parse_env("ASPEN_BLOBS_ENABLED").unwrap_or_else(default_blobs_enabled),
                auto_offload: parse_env("ASPEN_BLOBS_AUTO_OFFLOAD").unwrap_or_else(default_auto_offload),
                offload_threshold_bytes: parse_env("ASPEN_BLOBS_OFFLOAD_THRESHOLD_BYTES")
                    .unwrap_or_else(default_offload_threshold_bytes),
                gc_interval_secs: parse_env("ASPEN_BLOBS_GC_INTERVAL_SECS").unwrap_or_else(default_gc_interval_secs),
                gc_grace_period_secs: parse_env("ASPEN_BLOBS_GC_GRACE_PERIOD_SECS")
                    .unwrap_or_else(default_gc_grace_period_secs),
                // Replication configuration
                replication_factor: parse_env("ASPEN_BLOBS_REPLICATION_FACTOR")
                    .unwrap_or_else(default_replication_factor),
                min_replicas: parse_env("ASPEN_BLOBS_MIN_REPLICAS").unwrap_or_else(default_min_replicas),
                max_replicas: parse_env("ASPEN_BLOBS_MAX_REPLICAS").unwrap_or_else(default_max_replicas),
                repair_interval_secs: parse_env("ASPEN_BLOBS_REPAIR_INTERVAL_SECS")
                    .unwrap_or_else(default_repair_interval_secs),
                repair_delay_secs: parse_env("ASPEN_BLOBS_REPAIR_DELAY_SECS").unwrap_or_else(default_repair_delay_secs),
                enable_quorum_writes: parse_env("ASPEN_BLOBS_ENABLE_QUORUM_WRITES").unwrap_or(false),
                failure_domain_key: parse_env("ASPEN_BLOBS_FAILURE_DOMAIN_KEY"),
                enable_auto_replication: parse_env("ASPEN_BLOBS_ENABLE_AUTO_REPLICATION").unwrap_or(false),
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
            federation: FederationConfig {
                enabled: parse_env("ASPEN_FEDERATION_ENABLED").unwrap_or(false),
                cluster_name: parse_env("ASPEN_FEDERATION_CLUSTER_NAME")
                    .unwrap_or_else(default_federation_cluster_name),
                cluster_key: parse_env("ASPEN_FEDERATION_CLUSTER_KEY"),
                cluster_key_path: parse_env("ASPEN_FEDERATION_CLUSTER_KEY_PATH"),
                enable_dht_discovery: parse_env("ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY")
                    .unwrap_or_else(default_federation_dht_discovery),
                enable_gossip: parse_env("ASPEN_FEDERATION_ENABLE_GOSSIP").unwrap_or_else(default_federation_gossip),
                trusted_clusters: parse_env_vec("ASPEN_FEDERATION_TRUSTED_CLUSTERS"),
                announce_interval_secs: parse_env("ASPEN_FEDERATION_ANNOUNCE_INTERVAL_SECS")
                    .unwrap_or_else(default_federation_announce_interval_secs),
                max_peers: parse_env("ASPEN_FEDERATION_MAX_PEERS").unwrap_or_else(default_federation_max_peers),
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
            content_discovery: ContentDiscoveryConfig {
                enabled: parse_env("ASPEN_CONTENT_DISCOVERY_ENABLED").unwrap_or(false),
                server_mode: parse_env("ASPEN_CONTENT_DISCOVERY_SERVER_MODE").unwrap_or(false),
                bootstrap_nodes: parse_env_vec("ASPEN_CONTENT_DISCOVERY_BOOTSTRAP_NODES"),
                dht_port: parse_env("ASPEN_CONTENT_DISCOVERY_DHT_PORT").unwrap_or(0),
                auto_announce: parse_env("ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE").unwrap_or(false),
                max_concurrent_queries: parse_env("ASPEN_CONTENT_DISCOVERY_MAX_CONCURRENT_QUERIES").unwrap_or(8),
            },
            worker: WorkerConfig {
                enabled: parse_env("ASPEN_WORKER_ENABLED").unwrap_or(false),
                worker_count: parse_env("ASPEN_WORKER_COUNT").unwrap_or_else(default_worker_count),
                max_concurrent_jobs: parse_env("ASPEN_WORKER_MAX_CONCURRENT_JOBS")
                    .unwrap_or_else(default_max_concurrent_jobs),
                job_types: parse_env_vec("ASPEN_WORKER_JOB_TYPES"),
                tags: parse_env_vec("ASPEN_WORKER_TAGS"),
                prefer_local: parse_env("ASPEN_WORKER_PREFER_LOCAL").unwrap_or_else(default_prefer_local),
                data_locality_weight: parse_env("ASPEN_WORKER_DATA_LOCALITY_WEIGHT")
                    .unwrap_or_else(default_data_locality_weight),
                poll_interval_ms: parse_env("ASPEN_WORKER_POLL_INTERVAL_MS").unwrap_or_else(default_poll_interval_ms),
                visibility_timeout_secs: parse_env("ASPEN_WORKER_VISIBILITY_TIMEOUT_SECS")
                    .unwrap_or_else(default_visibility_timeout_secs),
                heartbeat_interval_ms: parse_env("ASPEN_WORKER_HEARTBEAT_INTERVAL_MS")
                    .unwrap_or_else(default_worker_heartbeat_ms),
                shutdown_timeout_ms: parse_env("ASPEN_WORKER_SHUTDOWN_TIMEOUT_MS")
                    .unwrap_or_else(default_shutdown_timeout_ms),
                enable_distributed: parse_env("ASPEN_WORKER_ENABLE_DISTRIBUTED").unwrap_or(false),
                enable_work_stealing: parse_env("ASPEN_WORKER_ENABLE_WORK_STEALING"),
                load_balancing_strategy: parse_env("ASPEN_WORKER_LOAD_BALANCING_STRATEGY"),
            },
            hooks: aspen_hooks::HooksConfig {
                enabled: parse_env("ASPEN_HOOKS_ENABLED").unwrap_or(false),
                ..Default::default()
            },
            ci: CiConfig {
                enabled: parse_env("ASPEN_CI_ENABLED").unwrap_or(false),
                auto_trigger: parse_env("ASPEN_CI_AUTO_TRIGGER").unwrap_or(false),
                max_concurrent_runs: parse_env("ASPEN_CI_MAX_CONCURRENT_RUNS")
                    .unwrap_or_else(default_ci_max_concurrent_runs),
                pipeline_timeout_secs: parse_env("ASPEN_CI_PIPELINE_TIMEOUT_SECS")
                    .unwrap_or_else(default_ci_pipeline_timeout_secs),
                watched_repos: parse_env_vec("ASPEN_CI_WATCHED_REPOS"),
                distributed_execution: parse_env("ASPEN_CI_DISTRIBUTED_EXECUTION")
                    .unwrap_or_else(default_ci_distributed_execution),
                avoid_leader: parse_env("ASPEN_CI_AVOID_LEADER").unwrap_or_else(default_ci_avoid_leader),
                resource_isolation: parse_env("ASPEN_CI_RESOURCE_ISOLATION")
                    .unwrap_or_else(default_ci_resource_isolation),
                max_job_memory_bytes: parse_env("ASPEN_CI_MAX_JOB_MEMORY_BYTES")
                    .unwrap_or_else(default_ci_max_memory_bytes),
            },
            nix_cache: NixCacheConfig {
                enabled: parse_env("ASPEN_NIX_CACHE_ENABLED").unwrap_or(false),
                store_dir: parse_env("ASPEN_NIX_CACHE_STORE_DIR").unwrap_or_else(default_nix_store_dir),
                priority: parse_env("ASPEN_NIX_CACHE_PRIORITY").unwrap_or_else(default_nix_cache_priority),
                want_mass_query: parse_env("ASPEN_NIX_CACHE_WANT_MASS_QUERY").unwrap_or_else(default_want_mass_query),
                cache_name: parse_env("ASPEN_NIX_CACHE_NAME"),
                signing_key_name: parse_env("ASPEN_NIX_CACHE_SIGNING_KEY_NAME"),
                transit_mount: parse_env("ASPEN_NIX_CACHE_TRANSIT_MOUNT")
                    .unwrap_or_else(default_nix_cache_transit_mount),
            },
            snix: SnixConfig {
                enabled: parse_env("ASPEN_SNIX_ENABLED").unwrap_or(false),
                directory_prefix: parse_env("ASPEN_SNIX_DIRECTORY_PREFIX").unwrap_or_else(default_snix_dir_prefix),
                pathinfo_prefix: parse_env("ASPEN_SNIX_PATHINFO_PREFIX").unwrap_or_else(default_snix_pathinfo_prefix),
                migration_enabled: parse_env("ASPEN_SNIX_MIGRATION_ENABLED").unwrap_or(false),
                migration_workers: parse_env("ASPEN_SNIX_MIGRATION_WORKERS").unwrap_or_else(default_migration_workers),
            },
            forge: ForgeConfig {
                enable_gossip: parse_env("ASPEN_FORGE_ENABLE_GOSSIP").unwrap_or(false),
            },
            #[cfg(feature = "secrets")]
            secrets: aspen_secrets::SecretsConfig {
                enabled: parse_env("ASPEN_SECRETS_ENABLED").unwrap_or(false),
                secrets_file: parse_env("ASPEN_SECRETS_FILE"),
                age_identity_file: parse_env("ASPEN_AGE_IDENTITY_FILE"),
                age_identity_env: parse_env("ASPEN_AGE_IDENTITY_ENV").unwrap_or_else(|| "SOPS_AGE_KEY".into()),
                kv_secrets_prefix: parse_env("ASPEN_SECRETS_KV_PREFIX").unwrap_or_else(|| "_system:secrets:".into()),
                cache_enabled: parse_env("ASPEN_SECRETS_CACHE_ENABLED").unwrap_or(true),
                cache_ttl_secs: parse_env("ASPEN_SECRETS_CACHE_TTL_SECS").unwrap_or(300),
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
        // Content discovery config merging
        if other.content_discovery.enabled {
            self.content_discovery.enabled = other.content_discovery.enabled;
        }
        if other.content_discovery.server_mode {
            self.content_discovery.server_mode = other.content_discovery.server_mode;
        }
        if !other.content_discovery.bootstrap_nodes.is_empty() {
            self.content_discovery.bootstrap_nodes = other.content_discovery.bootstrap_nodes;
        }
        if other.content_discovery.dht_port != 0 {
            self.content_discovery.dht_port = other.content_discovery.dht_port;
        }
        if other.content_discovery.auto_announce {
            self.content_discovery.auto_announce = other.content_discovery.auto_announce;
        }
        if other.content_discovery.max_concurrent_queries != 8 {
            self.content_discovery.max_concurrent_queries = other.content_discovery.max_concurrent_queries;
        }

        // Merge worker configuration
        if other.worker.enabled {
            self.worker.enabled = other.worker.enabled;
        }
        if other.worker.worker_count != default_worker_count() {
            self.worker.worker_count = other.worker.worker_count;
        }
        if other.worker.max_concurrent_jobs != default_max_concurrent_jobs() {
            self.worker.max_concurrent_jobs = other.worker.max_concurrent_jobs;
        }
        if !other.worker.job_types.is_empty() {
            self.worker.job_types = other.worker.job_types;
        }
        if !other.worker.tags.is_empty() {
            self.worker.tags = other.worker.tags;
        }
        if other.worker.prefer_local != default_prefer_local() {
            self.worker.prefer_local = other.worker.prefer_local;
        }
        if other.worker.data_locality_weight != default_data_locality_weight() {
            self.worker.data_locality_weight = other.worker.data_locality_weight;
        }
        if other.worker.poll_interval_ms != default_poll_interval_ms() {
            self.worker.poll_interval_ms = other.worker.poll_interval_ms;
        }
        if other.worker.visibility_timeout_secs != default_visibility_timeout_secs() {
            self.worker.visibility_timeout_secs = other.worker.visibility_timeout_secs;
        }
        if other.worker.heartbeat_interval_ms != default_worker_heartbeat_ms() {
            self.worker.heartbeat_interval_ms = other.worker.heartbeat_interval_ms;
        }
        if other.worker.shutdown_timeout_ms != default_shutdown_timeout_ms() {
            self.worker.shutdown_timeout_ms = other.worker.shutdown_timeout_ms;
        }

        // Hooks config merging
        if other.hooks.enabled {
            self.hooks.enabled = other.hooks.enabled;
        }

        // Secrets config merging
        #[cfg(feature = "secrets")]
        {
            if other.secrets.enabled {
                self.secrets.enabled = other.secrets.enabled;
            }
            if other.secrets.secrets_file.is_some() {
                self.secrets.secrets_file = other.secrets.secrets_file;
            }
            if other.secrets.age_identity_file.is_some() {
                self.secrets.age_identity_file = other.secrets.age_identity_file;
            }
            if other.secrets.age_identity_env != "SOPS_AGE_KEY" {
                self.secrets.age_identity_env = other.secrets.age_identity_env;
            }
            if other.secrets.kv_secrets_prefix != "_system:secrets:" {
                self.secrets.kv_secrets_prefix = other.secrets.kv_secrets_prefix;
            }
            if !other.secrets.cache_enabled {
                self.secrets.cache_enabled = other.secrets.cache_enabled;
            }
            if other.secrets.cache_ttl_secs != 300 {
                self.secrets.cache_ttl_secs = other.secrets.cache_ttl_secs;
            }
        }

        // CI config merging
        if other.ci.enabled {
            self.ci.enabled = other.ci.enabled;
        }
        if other.ci.auto_trigger {
            self.ci.auto_trigger = other.ci.auto_trigger;
        }
        if other.ci.max_concurrent_runs != default_ci_max_concurrent_runs() {
            self.ci.max_concurrent_runs = other.ci.max_concurrent_runs;
        }
        if other.ci.pipeline_timeout_secs != default_ci_pipeline_timeout_secs() {
            self.ci.pipeline_timeout_secs = other.ci.pipeline_timeout_secs;
        }
        if !other.ci.watched_repos.is_empty() {
            self.ci.watched_repos = other.ci.watched_repos;
        }

        // Nix cache config merging
        if other.nix_cache.enabled {
            self.nix_cache.enabled = other.nix_cache.enabled;
        }
        if other.nix_cache.store_dir != default_nix_store_dir() {
            self.nix_cache.store_dir = other.nix_cache.store_dir;
        }
        if other.nix_cache.priority != default_nix_cache_priority() {
            self.nix_cache.priority = other.nix_cache.priority;
        }
        if !other.nix_cache.want_mass_query {
            // Only merge if explicitly disabled (default is true)
            self.nix_cache.want_mass_query = other.nix_cache.want_mass_query;
        }
        if other.nix_cache.cache_name.is_some() {
            self.nix_cache.cache_name = other.nix_cache.cache_name;
        }
        if other.nix_cache.signing_key_name.is_some() {
            self.nix_cache.signing_key_name = other.nix_cache.signing_key_name;
        }
        if other.nix_cache.transit_mount != default_nix_cache_transit_mount() {
            self.nix_cache.transit_mount = other.nix_cache.transit_mount;
        }

        // SNIX config merging
        if other.snix.enabled {
            self.snix.enabled = other.snix.enabled;
        }
        if other.snix.directory_prefix != default_snix_dir_prefix() {
            self.snix.directory_prefix = other.snix.directory_prefix;
        }
        if other.snix.pathinfo_prefix != default_snix_pathinfo_prefix() {
            self.snix.pathinfo_prefix = other.snix.pathinfo_prefix;
        }
        if other.snix.migration_enabled {
            self.snix.migration_enabled = other.snix.migration_enabled;
        }
        if other.snix.migration_workers != default_migration_workers() {
            self.snix.migration_workers = other.snix.migration_workers;
        }

        // Forge config merging
        if other.forge.enable_gossip {
            self.forge.enable_gossip = other.forge.enable_gossip;
        }
    }

    /// Apply security defaults based on configuration.
    ///
    /// This method enforces security-by-default policies across all config sections.
    /// Call this after loading and merging configuration, before using it.
    ///
    /// Current policies:
    /// - When Pkarr DHT discovery is enabled, Raft authentication is automatically enabled to
    ///   prevent unauthorized nodes from joining the cluster.
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

        use crate::validation::check_disk_usage;
        use crate::validation::check_raft_timing_sanity;
        use crate::validation::validate_cookie;
        use crate::validation::validate_cookie_safety;
        use crate::validation::validate_node_id;
        use crate::validation::validate_raft_timings;
        use crate::validation::validate_secret_key;

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
            match check_disk_space(data_dir) {
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

        // Sharding configuration validation
        if self.sharding.enabled {
            use aspen_sharding::MAX_SHARDS;
            use aspen_sharding::MIN_SHARDS;

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

        // CI configuration validation (warnings only, not errors)
        for ci_warning in self.validate_ci_config() {
            warn!("{}", ci_warning);
        }

        Ok(())
    }

    /// Validate CI configuration and return warnings for potential issues.
    ///
    /// This method checks for common misconfigurations that could cause CI
    /// to fail silently or produce unexpected results.
    ///
    /// # Returns
    ///
    /// A vector of warning messages. Empty if configuration is valid.
    pub fn validate_ci_config(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.ci.enabled {
            // Warn if CI is enabled but blobs are disabled
            if !self.blobs.enabled {
                warnings.push(
                    "CI is enabled but blobs are disabled - build artifacts will not be stored in distributed storage"
                        .to_string(),
                );
            }

            // Warn if CI is enabled but no data_dir
            if self.data_dir.is_none() {
                warnings.push(
                    "CI is enabled but data_dir is not set - pipeline runs will not persist across restarts"
                        .to_string(),
                );
            }

            // Warn if CI is enabled but workers are disabled
            if !self.worker.enabled {
                warnings.push(
                    "CI is enabled but worker is disabled - this node cannot execute CI jobs, only orchestrate them"
                        .to_string(),
                );
            }

            // Warn if auto_trigger is enabled but no watched repos
            if self.ci.auto_trigger && self.ci.watched_repos.is_empty() {
                warnings.push(
                    "CI auto_trigger is enabled but watched_repos is empty - no repositories will be automatically triggered"
                        .to_string(),
                );
            }

            // Warn if nix_cache is not enabled (reduced functionality)
            if !self.nix_cache.enabled {
                warnings.push(
                    "CI is enabled but nix_cache is disabled - built store paths will not be served via binary cache"
                        .to_string(),
                );
            }
        }

        warnings
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

fn default_batch_config() -> Option<aspen_raft::BatchConfig> {
    Some(aspen_raft::BatchConfig::default())
}

// Helper functions for parsing environment variables
fn parse_env<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok()?.parse().ok()
}

fn parse_env_vec(key: &str) -> Vec<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
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

    /// Configuration file too large.
    ///
    /// Tiger Style: Prevents memory exhaustion from oversized config files.
    #[snafu(display("config file {} is too large: {} bytes (max {})", path.display(), size, max))]
    FileTooLarge {
        /// Path to the configuration file.
        path: PathBuf,
        /// Actual file size in bytes.
        size: u64,
        /// Maximum allowed size in bytes.
        max: u64,
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
            control_backend: ControlBackend::Raft, // Default value
            storage_backend: aspen_raft::storage::StorageBackend::Redb, // Default value
            ..Default::default()
        };

        // Override config uses non-default values for control_backend and storage_backend
        // to test that explicit non-default values override
        let override_config = NodeConfig {
            node_id: 2,
            data_dir: Some(PathBuf::from("/custom/data")),
            host: "192.168.1.1".into(),
            cookie: "custom-cookie".into(),
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
                bind_port: 7777,
            },
            peers: vec!["peer1".into()],
            storage_backend: aspen_raft::storage::StorageBackend::InMemory, // Non-default: should override
            redb_path: Some(PathBuf::from("/custom/node_shared.redb")),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.node_id, 2);
        assert_eq!(base.data_dir, Some(PathBuf::from("/custom/data")));
        assert_eq!(base.host, "192.168.1.1");
        assert_eq!(base.cookie, "custom-cookie");
        // Deterministic (non-default) should override Raft (default)
        assert_eq!(base.control_backend, ControlBackend::Deterministic);
        assert_eq!(base.heartbeat_interval_ms, 1000);
        assert_eq!(base.election_timeout_min_ms, 2000);
        assert_eq!(base.election_timeout_max_ms, 4000);
        assert_eq!(base.iroh.secret_key, Some("a".repeat(64)));
        assert!(!base.iroh.enable_gossip);
        assert_eq!(base.iroh.gossip_ticket, Some("test-ticket".into()));
        assert_eq!(base.peers, vec!["peer1"]);
        // InMemory (non-default) should override Redb (default)
        assert_eq!(base.storage_backend, aspen_raft::storage::StorageBackend::InMemory);
    }

    #[test]
    fn test_merge_preserves_explicit_base_values() {
        // Test that default override values don't clobber explicit base values
        // This tests the layered config precedence fix
        let mut base = NodeConfig {
            node_id: 1,
            control_backend: ControlBackend::Deterministic, // Explicit non-default
            storage_backend: aspen_raft::storage::StorageBackend::InMemory, // Explicit non-default
            ..Default::default()
        };

        // Override config uses DEFAULT values - should NOT override base
        let override_config = NodeConfig {
            node_id: 0,                                                 // Default: 0 doesn't override
            control_backend: ControlBackend::Raft,                      // Default: should NOT override
            storage_backend: aspen_raft::storage::StorageBackend::Redb, // Default: should NOT override
            ..Default::default()
        };

        base.merge(override_config);

        // Original non-default values should be preserved (not clobbered by defaults)
        assert_eq!(base.node_id, 1); // Preserved (0 is "unset")
        assert_eq!(base.control_backend, ControlBackend::Deterministic); // Preserved
        assert_eq!(base.storage_backend, aspen_raft::storage::StorageBackend::InMemory); // Preserved
    }
}
