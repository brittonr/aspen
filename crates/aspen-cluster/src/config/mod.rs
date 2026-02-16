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

mod blobs;
mod ci;
mod cluster;
mod dns;
mod docs;
mod federation;
mod forge;
mod iroh;
mod nix_cache;
mod peer_sync;
mod sharding;
mod snix;
mod worker;

// Re-export all public types from submodules
use std::net::SocketAddr;
use std::path::PathBuf;

use aspen_disk::check_disk_space;
use aspen_raft::storage::StorageBackend;
// Import default functions from submodules for use in NodeConfig
use blobs::default_auto_offload;
use blobs::default_blobs_enabled;
use blobs::default_gc_grace_period_secs;
use blobs::default_gc_interval_secs;
use blobs::default_max_replicas;
use blobs::default_min_replicas;
use blobs::default_offload_threshold_bytes;
use blobs::default_repair_delay_secs;
use blobs::default_repair_interval_secs;
use blobs::default_replication_factor;
pub use blobs::*;
use ci::default_ci_avoid_leader;
use ci::default_ci_distributed_execution;
use ci::default_ci_max_concurrent_runs;
use ci::default_ci_max_memory_bytes;
use ci::default_ci_pipeline_timeout_secs;
use ci::default_ci_resource_isolation;
pub use ci::*;
pub use cluster::*;
use dns::default_dns_bind_addr;
use dns::default_dns_forwarding;
use dns::default_dns_upstreams;
use dns::default_dns_zones;
pub use dns::*;
use docs::default_background_sync_interval_secs;
use docs::default_docs_enabled;
use docs::default_enable_background_sync;
pub use docs::*;
use federation::default_federation_announce_interval_secs;
use federation::default_federation_cluster_name;
use federation::default_federation_dht_discovery;
use federation::default_federation_gossip;
use federation::default_federation_max_peers;
pub use federation::*;
pub use forge::*;
use iroh::default_enable_gossip;
use iroh::default_enable_mdns;
use iroh::default_enable_pkarr_dht;
use iroh::default_enable_pkarr_relay;
use iroh::default_include_pkarr_direct_addresses;
use iroh::default_pkarr_republish_delay_secs;
pub use iroh::*;
use nix_cache::default_enable_ci_substituter;
use nix_cache::default_nix_cache_priority;
use nix_cache::default_nix_cache_transit_mount;
use nix_cache::default_nix_store_dir;
use nix_cache::default_want_mass_query;
pub use nix_cache::*;
use peer_sync::default_max_peer_subscriptions;
use peer_sync::default_peer_reconnect_interval_secs;
use peer_sync::default_peer_sync_priority;
pub use peer_sync::*;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use sharding::default_num_shards;
pub use sharding::*;
use snafu::Snafu;
use snix::default_migration_workers;
use snix::default_snix_dir_prefix;
use snix::default_snix_pathinfo_prefix;
pub use snix::*;
use worker::default_data_locality_weight;
use worker::default_max_concurrent_jobs;
use worker::default_poll_interval_ms;
use worker::default_prefer_local;
use worker::default_shutdown_timeout_ms;
use worker::default_visibility_timeout_secs;
use worker::default_worker_count;
use worker::default_worker_heartbeat_ms;
pub use worker::*;

#[cfg(feature = "blob")]
use crate::content_discovery::ContentDiscoveryConfig;

/// Configuration for an Aspen cluster node.
///
/// Configuration is loaded in layers with the following precedence (lowest to highest):
/// 1. Environment variables (ASPEN_*)
/// 2. TOML configuration file
/// 3. Command-line arguments
///
/// This means CLI args override TOML config, which overrides environment variables.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    #[cfg(feature = "blob")]
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
    pub hooks: aspen_hooks_types::HooksConfig,

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
            #[cfg(feature = "blob")]
            content_discovery: ContentDiscoveryConfig::default(),
            worker: WorkerConfig::default(),
            hooks: aspen_hooks_types::HooksConfig::default(),
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
            iroh: from_env_iroh(),
            docs: from_env_docs(),
            blobs: from_env_blobs(),
            peer_sync: from_env_peer_sync(),
            federation: from_env_federation(),
            sharding: from_env_sharding(),
            peers: parse_env_vec("ASPEN_PEERS"),
            batch_config: default_batch_config(),
            dns_server: from_env_dns(),
            #[cfg(feature = "blob")]
            content_discovery: from_env_content_discovery(),
            worker: from_env_worker(),
            hooks: aspen_hooks_types::HooksConfig {
                is_enabled: parse_env("ASPEN_HOOKS_ENABLED").unwrap_or(false),
                ..Default::default()
            },
            ci: from_env_ci(),
            nix_cache: from_env_nix_cache(),
            snix: from_env_snix(),
            forge: ForgeConfig {
                enable_gossip: parse_env("ASPEN_FORGE_ENABLE_GOSSIP").unwrap_or(false),
            },
            #[cfg(feature = "secrets")]
            secrets: from_env_secrets(),
        }
    }

    /// Merge configuration from another source.
    ///
    /// Fields in `other` that are `Some` or non-default will override fields in `self`.
    /// This is used to implement the layered config precedence.
    pub fn merge(&mut self, other: Self) {
        merge_core_config(self, &other);
        merge_iroh_config(&mut self.iroh, other.iroh);
        merge_docs_config(&mut self.docs, other.docs);
        merge_blobs_config(&mut self.blobs, other.blobs);
        merge_peer_sync_config(&mut self.peer_sync, other.peer_sync);
        merge_sharding_config(&mut self.sharding, other.sharding);
        if !other.peers.is_empty() {
            self.peers = other.peers;
        }
        merge_dns_config(&mut self.dns_server, other.dns_server);
        #[cfg(feature = "blob")]
        merge_content_discovery_config(&mut self.content_discovery, other.content_discovery);
        merge_worker_config(&mut self.worker, other.worker);
        if other.hooks.is_enabled {
            self.hooks.is_enabled = other.hooks.is_enabled;
        }
        #[cfg(feature = "secrets")]
        merge_secrets_config(&mut self.secrets, other.secrets);
        merge_ci_config(&mut self.ci, other.ci);
        merge_nix_cache_config(&mut self.nix_cache, other.nix_cache);
        merge_snix_config(&mut self.snix, other.snix);
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
                // Decomposed: check existence first, then check type
                let does_exist = path.exists();
                let is_directory = path.is_dir();
                if does_exist && is_directory {
                    return Err(ConfigError::Validation {
                        message: format!("{} path {} exists but is a directory, expected a file", name, path.display()),
                    });
                }
            }
        }

        // Sharding configuration validation
        if self.sharding.is_enabled {
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

        if self.ci.is_enabled {
            // Warn if CI is enabled but blobs are disabled
            if !self.blobs.is_enabled {
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
            if !self.worker.is_enabled {
                warnings.push(
                    "CI is enabled but worker is disabled - this node cannot execute CI jobs, only orchestrate them"
                        .to_string(),
                );
            }

            // Warn if auto_trigger is enabled but no watched repos
            // Decomposed: check if auto_trigger is on, then check if repos are empty
            let is_auto_trigger_enabled = self.ci.auto_trigger;
            let has_no_repos = self.ci.watched_repos.is_empty();
            if is_auto_trigger_enabled && has_no_repos {
                warnings.push(
                    "CI auto_trigger is enabled but watched_repos is empty - no repositories will be automatically triggered"
                        .to_string(),
                );
            }

            // Warn if nix_cache is not enabled (reduced functionality)
            if !self.nix_cache.is_enabled {
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

// --- Merge helpers: per-domain merge functions for layered config precedence ---

/// Merge core node-level fields (node_id, data_dir, host, cookie, backends, raft timings).
fn merge_core_config(target: &mut NodeConfig, other: &NodeConfig) {
    if other.node_id != 0 {
        target.node_id = other.node_id;
    }
    if other.data_dir.is_some() {
        target.data_dir = other.data_dir.clone();
    }
    if other.host != default_host() {
        target.host = other.host.clone();
    }
    if other.cookie != default_cookie() {
        target.cookie = other.cookie.clone();
    }
    // Tiger Style: Only merge control_backend if explicitly set (non-default)
    // This preserves TOML settings when CLI args don't override them
    if other.control_backend != ControlBackend::default() {
        target.control_backend = other.control_backend;
    }
    // Tiger Style: Only merge storage_backend if explicitly set (non-default)
    // Example: TOML sets `storage_backend = "inmemory"`, CLI doesn't override,
    // so InMemory is preserved instead of being overwritten by default Redb
    if other.storage_backend != StorageBackend::default() {
        target.storage_backend = other.storage_backend;
    }
    if other.redb_path.is_some() {
        target.redb_path = other.redb_path.clone();
    }
    if other.heartbeat_interval_ms != default_heartbeat_interval_ms() {
        target.heartbeat_interval_ms = other.heartbeat_interval_ms;
    }
    if other.election_timeout_min_ms != default_election_timeout_min_ms() {
        target.election_timeout_min_ms = other.election_timeout_min_ms;
    }
    if other.election_timeout_max_ms != default_election_timeout_max_ms() {
        target.election_timeout_max_ms = other.election_timeout_max_ms;
    }
}

/// Merge Iroh P2P networking configuration.
fn merge_iroh_config(target: &mut IrohConfig, other: IrohConfig) {
    if other.secret_key.is_some() {
        target.secret_key = other.secret_key;
    }
    if other.enable_gossip != default_enable_gossip() {
        target.enable_gossip = other.enable_gossip;
    }
    if other.gossip_ticket.is_some() {
        target.gossip_ticket = other.gossip_ticket;
    }
    if other.enable_mdns != default_enable_mdns() {
        target.enable_mdns = other.enable_mdns;
    }
    if other.enable_dns_discovery {
        target.enable_dns_discovery = other.enable_dns_discovery;
    }
    if other.dns_discovery_url.is_some() {
        target.dns_discovery_url = other.dns_discovery_url;
    }
    if other.enable_pkarr {
        target.enable_pkarr = other.enable_pkarr;
    }
    if other.enable_pkarr_dht != default_enable_pkarr_dht() {
        target.enable_pkarr_dht = other.enable_pkarr_dht;
    }
    if other.enable_pkarr_relay != default_enable_pkarr_relay() {
        target.enable_pkarr_relay = other.enable_pkarr_relay;
    }
    if other.include_pkarr_direct_addresses != default_include_pkarr_direct_addresses() {
        target.include_pkarr_direct_addresses = other.include_pkarr_direct_addresses;
    }
    if other.pkarr_republish_delay_secs != default_pkarr_republish_delay_secs() {
        target.pkarr_republish_delay_secs = other.pkarr_republish_delay_secs;
    }
    if other.pkarr_relay_url.is_some() {
        target.pkarr_relay_url = other.pkarr_relay_url;
    }
    if other.relay_mode != RelayMode::default() {
        target.relay_mode = other.relay_mode;
    }
    if !other.relay_urls.is_empty() {
        target.relay_urls = other.relay_urls;
    }
    if other.enable_raft_auth {
        target.enable_raft_auth = other.enable_raft_auth;
    }
}

/// Merge iroh-docs real-time synchronization configuration.
fn merge_docs_config(target: &mut DocsConfig, other: DocsConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.enable_background_sync != default_enable_background_sync() {
        target.enable_background_sync = other.enable_background_sync;
    }
    if other.background_sync_interval_secs != default_background_sync_interval_secs() {
        target.background_sync_interval_secs = other.background_sync_interval_secs;
    }
    if other.in_memory {
        target.in_memory = other.in_memory;
    }
    if other.namespace_secret.is_some() {
        target.namespace_secret = other.namespace_secret;
    }
    if other.author_secret.is_some() {
        target.author_secret = other.author_secret;
    }
}

/// Merge iroh-blobs content-addressed storage configuration.
fn merge_blobs_config(target: &mut BlobConfig, other: BlobConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.auto_offload != default_auto_offload() {
        target.auto_offload = other.auto_offload;
    }
    if other.offload_threshold_bytes != default_offload_threshold_bytes() {
        target.offload_threshold_bytes = other.offload_threshold_bytes;
    }
    if other.gc_interval_secs != default_gc_interval_secs() {
        target.gc_interval_secs = other.gc_interval_secs;
    }
    if other.gc_grace_period_secs != default_gc_grace_period_secs() {
        target.gc_grace_period_secs = other.gc_grace_period_secs;
    }
}

/// Merge peer cluster synchronization configuration.
fn merge_peer_sync_config(target: &mut PeerSyncConfig, other: PeerSyncConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.default_priority != default_peer_sync_priority() {
        target.default_priority = other.default_priority;
    }
    if other.max_subscriptions != default_max_peer_subscriptions() {
        target.max_subscriptions = other.max_subscriptions;
    }
    if other.reconnect_interval_secs != default_peer_reconnect_interval_secs() {
        target.reconnect_interval_secs = other.reconnect_interval_secs;
    }
    if other.max_reconnect_attempts != 0 {
        target.max_reconnect_attempts = other.max_reconnect_attempts;
    }
}

/// Merge horizontal sharding configuration.
fn merge_sharding_config(target: &mut ShardingConfig, other: ShardingConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.num_shards != default_num_shards() {
        target.num_shards = other.num_shards;
    }
    if !other.local_shards.is_empty() {
        target.local_shards = other.local_shards;
    }
}

/// Merge DNS protocol server configuration.
fn merge_dns_config(target: &mut DnsServerConfig, other: DnsServerConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.bind_addr != default_dns_bind_addr() {
        target.bind_addr = other.bind_addr;
    }
    if other.zones != default_dns_zones() {
        target.zones = other.zones;
    }
    if other.upstreams != default_dns_upstreams() {
        target.upstreams = other.upstreams;
    }
    if other.forwarding_enabled != default_dns_forwarding() {
        target.forwarding_enabled = other.forwarding_enabled;
    }
}

/// Merge global content discovery (DHT) configuration.
#[cfg(feature = "blob")]
fn merge_content_discovery_config(target: &mut ContentDiscoveryConfig, other: ContentDiscoveryConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.server_mode {
        target.server_mode = other.server_mode;
    }
    if !other.bootstrap_nodes.is_empty() {
        target.bootstrap_nodes = other.bootstrap_nodes;
    }
    if other.dht_port != 0 {
        target.dht_port = other.dht_port;
    }
    if other.auto_announce {
        target.auto_announce = other.auto_announce;
    }
    if other.max_concurrent_queries != 8 {
        target.max_concurrent_queries = other.max_concurrent_queries;
    }
}

/// Merge worker pool configuration for distributed job execution.
fn merge_worker_config(target: &mut WorkerConfig, other: WorkerConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.worker_count != default_worker_count() {
        target.worker_count = other.worker_count;
    }
    if other.max_concurrent_jobs != default_max_concurrent_jobs() {
        target.max_concurrent_jobs = other.max_concurrent_jobs;
    }
    if !other.job_types.is_empty() {
        target.job_types = other.job_types;
    }
    if !other.tags.is_empty() {
        target.tags = other.tags;
    }
    if other.prefer_local != default_prefer_local() {
        target.prefer_local = other.prefer_local;
    }
    if other.data_locality_weight != default_data_locality_weight() {
        target.data_locality_weight = other.data_locality_weight;
    }
    if other.poll_interval_ms != default_poll_interval_ms() {
        target.poll_interval_ms = other.poll_interval_ms;
    }
    if other.visibility_timeout_secs != default_visibility_timeout_secs() {
        target.visibility_timeout_secs = other.visibility_timeout_secs;
    }
    if other.heartbeat_interval_ms != default_worker_heartbeat_ms() {
        target.heartbeat_interval_ms = other.heartbeat_interval_ms;
    }
    if other.shutdown_timeout_ms != default_shutdown_timeout_ms() {
        target.shutdown_timeout_ms = other.shutdown_timeout_ms;
    }
}

/// Merge secrets management configuration (SOPS-based).
#[cfg(feature = "secrets")]
fn merge_secrets_config(target: &mut aspen_secrets::SecretsConfig, other: aspen_secrets::SecretsConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.secrets_file.is_some() {
        target.secrets_file = other.secrets_file;
    }
    if other.age_identity_file.is_some() {
        target.age_identity_file = other.age_identity_file;
    }
    if other.age_identity_env != "SOPS_AGE_KEY" {
        target.age_identity_env = other.age_identity_env;
    }
    if other.kv_secrets_prefix != "_system:secrets:" {
        target.kv_secrets_prefix = other.kv_secrets_prefix;
    }
    if !other.cache_enabled {
        target.cache_enabled = other.cache_enabled;
    }
    if other.cache_ttl_secs != 300 {
        target.cache_ttl_secs = other.cache_ttl_secs;
    }
}

/// Merge CI/CD pipeline configuration.
fn merge_ci_config(target: &mut CiConfig, other: CiConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.auto_trigger {
        target.auto_trigger = other.auto_trigger;
    }
    if other.max_concurrent_runs != default_ci_max_concurrent_runs() {
        target.max_concurrent_runs = other.max_concurrent_runs;
    }
    if other.pipeline_timeout_secs != default_ci_pipeline_timeout_secs() {
        target.pipeline_timeout_secs = other.pipeline_timeout_secs;
    }
    if !other.watched_repos.is_empty() {
        target.watched_repos = other.watched_repos;
    }
}

/// Merge Nix binary cache configuration.
fn merge_nix_cache_config(target: &mut NixCacheConfig, other: NixCacheConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.store_dir != default_nix_store_dir() {
        target.store_dir = other.store_dir;
    }
    if other.priority != default_nix_cache_priority() {
        target.priority = other.priority;
    }
    if !other.want_mass_query {
        // Only merge if explicitly disabled (default is true)
        target.want_mass_query = other.want_mass_query;
    }
    if other.cache_name.is_some() {
        target.cache_name = other.cache_name;
    }
    if other.signing_key_name.is_some() {
        target.signing_key_name = other.signing_key_name;
    }
    if other.transit_mount != default_nix_cache_transit_mount() {
        target.transit_mount = other.transit_mount;
    }
}

/// Merge SNIX content-addressed storage configuration.
fn merge_snix_config(target: &mut SnixConfig, other: SnixConfig) {
    if other.is_enabled {
        target.is_enabled = other.is_enabled;
    }
    if other.directory_prefix != default_snix_dir_prefix() {
        target.directory_prefix = other.directory_prefix;
    }
    if other.pathinfo_prefix != default_snix_pathinfo_prefix() {
        target.pathinfo_prefix = other.pathinfo_prefix;
    }
    if other.migration_enabled {
        target.migration_enabled = other.migration_enabled;
    }
    if other.migration_workers != default_migration_workers() {
        target.migration_workers = other.migration_workers;
    }
}

// --- from_env helpers: per-section environment variable parsers ---

/// Parse Iroh P2P networking configuration from environment variables.
fn from_env_iroh() -> IrohConfig {
    IrohConfig {
        secret_key: parse_env("ASPEN_IROH_SECRET_KEY"),
        enable_gossip: parse_env("ASPEN_IROH_ENABLE_GOSSIP").unwrap_or_else(default_enable_gossip),
        gossip_ticket: parse_env("ASPEN_IROH_GOSSIP_TICKET"),
        enable_mdns: parse_env("ASPEN_IROH_ENABLE_MDNS").unwrap_or_else(default_enable_mdns),
        enable_dns_discovery: parse_env("ASPEN_IROH_ENABLE_DNS_DISCOVERY").unwrap_or(false),
        dns_discovery_url: parse_env("ASPEN_IROH_DNS_DISCOVERY_URL"),
        enable_pkarr: parse_env("ASPEN_IROH_ENABLE_PKARR").unwrap_or(false),
        enable_pkarr_dht: parse_env("ASPEN_IROH_ENABLE_PKARR_DHT").unwrap_or_else(default_enable_pkarr_dht),
        enable_pkarr_relay: parse_env("ASPEN_IROH_ENABLE_PKARR_RELAY").unwrap_or_else(default_enable_pkarr_relay),
        include_pkarr_direct_addresses: parse_env("ASPEN_IROH_INCLUDE_PKARR_DIRECT_ADDRESSES")
            .unwrap_or_else(default_include_pkarr_direct_addresses),
        pkarr_republish_delay_secs: parse_env("ASPEN_IROH_PKARR_REPUBLISH_DELAY_SECS")
            .unwrap_or_else(default_pkarr_republish_delay_secs),
        pkarr_relay_url: parse_env("ASPEN_IROH_PKARR_RELAY_URL"),
        relay_mode: parse_env("ASPEN_IROH_RELAY_MODE").unwrap_or_default(),
        relay_urls: parse_env_vec("ASPEN_IROH_RELAY_URLS"),
        enable_raft_auth: parse_env("ASPEN_IROH_ENABLE_RAFT_AUTH").unwrap_or(false),
        bind_port: parse_env("ASPEN_IROH_BIND_PORT").unwrap_or(0),
    }
}

/// Parse iroh-docs configuration from environment variables.
fn from_env_docs() -> DocsConfig {
    DocsConfig {
        is_enabled: parse_env("ASPEN_DOCS_ENABLED").unwrap_or(false),
        enable_background_sync: parse_env("ASPEN_DOCS_ENABLE_BACKGROUND_SYNC")
            .unwrap_or_else(default_enable_background_sync),
        background_sync_interval_secs: parse_env("ASPEN_DOCS_BACKGROUND_SYNC_INTERVAL_SECS")
            .unwrap_or_else(default_background_sync_interval_secs),
        in_memory: parse_env("ASPEN_DOCS_IN_MEMORY").unwrap_or(false),
        namespace_secret: parse_env("ASPEN_DOCS_NAMESPACE_SECRET"),
        author_secret: parse_env("ASPEN_DOCS_AUTHOR_SECRET"),
    }
}

/// Parse iroh-blobs configuration from environment variables.
fn from_env_blobs() -> BlobConfig {
    BlobConfig {
        is_enabled: parse_env("ASPEN_BLOBS_ENABLED").unwrap_or_else(default_blobs_enabled),
        auto_offload: parse_env("ASPEN_BLOBS_AUTO_OFFLOAD").unwrap_or_else(default_auto_offload),
        offload_threshold_bytes: parse_env("ASPEN_BLOBS_OFFLOAD_THRESHOLD_BYTES")
            .unwrap_or_else(default_offload_threshold_bytes),
        gc_interval_secs: parse_env("ASPEN_BLOBS_GC_INTERVAL_SECS").unwrap_or_else(default_gc_interval_secs),
        gc_grace_period_secs: parse_env("ASPEN_BLOBS_GC_GRACE_PERIOD_SECS")
            .unwrap_or_else(default_gc_grace_period_secs),
        replication_factor: parse_env("ASPEN_BLOBS_REPLICATION_FACTOR").unwrap_or_else(default_replication_factor),
        min_replicas: parse_env("ASPEN_BLOBS_MIN_REPLICAS").unwrap_or_else(default_min_replicas),
        max_replicas: parse_env("ASPEN_BLOBS_MAX_REPLICAS").unwrap_or_else(default_max_replicas),
        repair_interval_secs: parse_env("ASPEN_BLOBS_REPAIR_INTERVAL_SECS")
            .unwrap_or_else(default_repair_interval_secs),
        repair_delay_secs: parse_env("ASPEN_BLOBS_REPAIR_DELAY_SECS").unwrap_or_else(default_repair_delay_secs),
        enable_quorum_writes: parse_env("ASPEN_BLOBS_ENABLE_QUORUM_WRITES").unwrap_or(false),
        failure_domain_key: parse_env("ASPEN_BLOBS_FAILURE_DOMAIN_KEY"),
        enable_auto_replication: parse_env("ASPEN_BLOBS_ENABLE_AUTO_REPLICATION").unwrap_or(false),
    }
}

/// Parse peer sync configuration from environment variables.
fn from_env_peer_sync() -> PeerSyncConfig {
    PeerSyncConfig {
        is_enabled: parse_env("ASPEN_PEER_SYNC_ENABLED").unwrap_or(false),
        default_priority: parse_env("ASPEN_PEER_SYNC_DEFAULT_PRIORITY").unwrap_or_else(default_peer_sync_priority),
        max_subscriptions: parse_env("ASPEN_PEER_SYNC_MAX_SUBSCRIPTIONS")
            .unwrap_or_else(default_max_peer_subscriptions),
        reconnect_interval_secs: parse_env("ASPEN_PEER_SYNC_RECONNECT_INTERVAL_SECS")
            .unwrap_or_else(default_peer_reconnect_interval_secs),
        max_reconnect_attempts: parse_env("ASPEN_PEER_SYNC_MAX_RECONNECT_ATTEMPTS").unwrap_or(0),
    }
}

/// Parse federation configuration from environment variables.
fn from_env_federation() -> FederationConfig {
    FederationConfig {
        is_enabled: parse_env("ASPEN_FEDERATION_ENABLED").unwrap_or(false),
        cluster_name: parse_env("ASPEN_FEDERATION_CLUSTER_NAME").unwrap_or_else(default_federation_cluster_name),
        cluster_key: parse_env("ASPEN_FEDERATION_CLUSTER_KEY"),
        cluster_key_path: parse_env("ASPEN_FEDERATION_CLUSTER_KEY_PATH"),
        enable_dht_discovery: parse_env("ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY")
            .unwrap_or_else(default_federation_dht_discovery),
        enable_gossip: parse_env("ASPEN_FEDERATION_ENABLE_GOSSIP").unwrap_or_else(default_federation_gossip),
        trusted_clusters: parse_env_vec("ASPEN_FEDERATION_TRUSTED_CLUSTERS"),
        announce_interval_secs: parse_env("ASPEN_FEDERATION_ANNOUNCE_INTERVAL_SECS")
            .unwrap_or_else(default_federation_announce_interval_secs),
        max_peers: parse_env("ASPEN_FEDERATION_MAX_PEERS").unwrap_or_else(default_federation_max_peers),
    }
}

/// Parse sharding configuration from environment variables.
fn from_env_sharding() -> ShardingConfig {
    ShardingConfig {
        is_enabled: parse_env("ASPEN_SHARDING_ENABLED").unwrap_or(false),
        num_shards: parse_env("ASPEN_SHARDING_NUM_SHARDS").unwrap_or_else(default_num_shards),
        local_shards: parse_env_vec("ASPEN_SHARDING_LOCAL_SHARDS").into_iter().filter_map(|s| s.parse().ok()).collect(),
    }
}

/// Parse DNS server configuration from environment variables.
fn from_env_dns() -> DnsServerConfig {
    let zones = parse_env_vec("ASPEN_DNS_SERVER_ZONES");
    let upstreams: Vec<SocketAddr> =
        parse_env_vec("ASPEN_DNS_SERVER_UPSTREAMS").into_iter().filter_map(|s| s.parse().ok()).collect();

    DnsServerConfig {
        is_enabled: parse_env("ASPEN_DNS_SERVER_ENABLED").unwrap_or(false),
        bind_addr: parse_env("ASPEN_DNS_SERVER_BIND_ADDR").unwrap_or_else(default_dns_bind_addr),
        zones: if zones.is_empty() { default_dns_zones() } else { zones },
        upstreams: if upstreams.is_empty() {
            default_dns_upstreams()
        } else {
            upstreams
        },
        forwarding_enabled: parse_env("ASPEN_DNS_SERVER_FORWARDING_ENABLED").unwrap_or_else(default_dns_forwarding),
    }
}

/// Parse content discovery configuration from environment variables.
#[cfg(feature = "blob")]
fn from_env_content_discovery() -> ContentDiscoveryConfig {
    ContentDiscoveryConfig {
        is_enabled: parse_env("ASPEN_CONTENT_DISCOVERY_ENABLED").unwrap_or(false),
        server_mode: parse_env("ASPEN_CONTENT_DISCOVERY_SERVER_MODE").unwrap_or(false),
        bootstrap_nodes: parse_env_vec("ASPEN_CONTENT_DISCOVERY_BOOTSTRAP_NODES"),
        dht_port: parse_env("ASPEN_CONTENT_DISCOVERY_DHT_PORT").unwrap_or(0),
        auto_announce: parse_env("ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE").unwrap_or(false),
        max_concurrent_queries: parse_env("ASPEN_CONTENT_DISCOVERY_MAX_CONCURRENT_QUERIES").unwrap_or(8),
    }
}

/// Parse worker pool configuration from environment variables.
fn from_env_worker() -> WorkerConfig {
    WorkerConfig {
        is_enabled: parse_env("ASPEN_WORKER_ENABLED").unwrap_or(false),
        worker_count: parse_env("ASPEN_WORKER_COUNT").unwrap_or_else(default_worker_count),
        max_concurrent_jobs: parse_env("ASPEN_WORKER_MAX_CONCURRENT_JOBS").unwrap_or_else(default_max_concurrent_jobs),
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
        shutdown_timeout_ms: parse_env("ASPEN_WORKER_SHUTDOWN_TIMEOUT_MS").unwrap_or_else(default_shutdown_timeout_ms),
        enable_distributed: parse_env("ASPEN_WORKER_ENABLE_DISTRIBUTED").unwrap_or(false),
        enable_work_stealing: parse_env("ASPEN_WORKER_ENABLE_WORK_STEALING"),
        load_balancing_strategy: parse_env("ASPEN_WORKER_LOAD_BALANCING_STRATEGY"),
    }
}

/// Parse CI/CD pipeline configuration from environment variables.
fn from_env_ci() -> CiConfig {
    CiConfig {
        is_enabled: parse_env("ASPEN_CI_ENABLED").unwrap_or(false),
        auto_trigger: parse_env("ASPEN_CI_AUTO_TRIGGER").unwrap_or(false),
        max_concurrent_runs: parse_env("ASPEN_CI_MAX_CONCURRENT_RUNS").unwrap_or_else(default_ci_max_concurrent_runs),
        pipeline_timeout_secs: parse_env("ASPEN_CI_PIPELINE_TIMEOUT_SECS")
            .unwrap_or_else(default_ci_pipeline_timeout_secs),
        watched_repos: parse_env_vec("ASPEN_CI_WATCHED_REPOS"),
        distributed_execution: parse_env("ASPEN_CI_DISTRIBUTED_EXECUTION")
            .unwrap_or_else(default_ci_distributed_execution),
        avoid_leader: parse_env("ASPEN_CI_AVOID_LEADER").unwrap_or_else(default_ci_avoid_leader),
        resource_isolation: parse_env("ASPEN_CI_RESOURCE_ISOLATION").unwrap_or_else(default_ci_resource_isolation),
        max_job_memory_bytes: parse_env("ASPEN_CI_MAX_JOB_MEMORY_BYTES").unwrap_or_else(default_ci_max_memory_bytes),
    }
}

/// Parse Nix binary cache configuration from environment variables.
fn from_env_nix_cache() -> NixCacheConfig {
    NixCacheConfig {
        is_enabled: parse_env("ASPEN_NIX_CACHE_ENABLED").unwrap_or(false),
        store_dir: parse_env("ASPEN_NIX_CACHE_STORE_DIR").unwrap_or_else(default_nix_store_dir),
        priority: parse_env("ASPEN_NIX_CACHE_PRIORITY").unwrap_or_else(default_nix_cache_priority),
        want_mass_query: parse_env("ASPEN_NIX_CACHE_WANT_MASS_QUERY").unwrap_or_else(default_want_mass_query),
        cache_name: parse_env("ASPEN_NIX_CACHE_NAME"),
        signing_key_name: parse_env("ASPEN_NIX_CACHE_SIGNING_KEY_NAME"),
        transit_mount: parse_env("ASPEN_NIX_CACHE_TRANSIT_MOUNT").unwrap_or_else(default_nix_cache_transit_mount),
        enable_ci_substituter: parse_env("ASPEN_NIX_CACHE_ENABLE_CI_SUBSTITUTER")
            .unwrap_or_else(default_enable_ci_substituter),
    }
}

/// Parse SNIX content-addressed storage configuration from environment variables.
fn from_env_snix() -> SnixConfig {
    SnixConfig {
        is_enabled: parse_env("ASPEN_SNIX_ENABLED").unwrap_or(false),
        directory_prefix: parse_env("ASPEN_SNIX_DIRECTORY_PREFIX").unwrap_or_else(default_snix_dir_prefix),
        pathinfo_prefix: parse_env("ASPEN_SNIX_PATHINFO_PREFIX").unwrap_or_else(default_snix_pathinfo_prefix),
        migration_enabled: parse_env("ASPEN_SNIX_MIGRATION_ENABLED").unwrap_or(false),
        migration_workers: parse_env("ASPEN_SNIX_MIGRATION_WORKERS").unwrap_or_else(default_migration_workers),
    }
}

/// Parse secrets management configuration from environment variables.
#[cfg(feature = "secrets")]
fn from_env_secrets() -> aspen_secrets::SecretsConfig {
    aspen_secrets::SecretsConfig {
        is_enabled: parse_env("ASPEN_SECRETS_ENABLED").unwrap_or(false),
        secrets_file: parse_env("ASPEN_SECRETS_FILE"),
        age_identity_file: parse_env("ASPEN_AGE_IDENTITY_FILE"),
        age_identity_env: parse_env("ASPEN_AGE_IDENTITY_ENV").unwrap_or_else(|| "SOPS_AGE_KEY".into()),
        kv_secrets_prefix: parse_env("ASPEN_SECRETS_KV_PREFIX").unwrap_or_else(|| "_system:secrets:".into()),
        cache_enabled: parse_env("ASPEN_SECRETS_CACHE_ENABLED").unwrap_or(true),
        cache_ttl_secs: parse_env("ASPEN_SECRETS_CACHE_TTL_SECS").unwrap_or(300),
    }
}

// Default value functions for NodeConfig
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
