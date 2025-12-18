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
//! storage_backend = "Sqlite"
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

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

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
    /// - Sqlite: ACID-compliant SQLite storage (recommended for production)
    /// - InMemory: Fast, non-durable (data lost on restart), good for testing
    /// - Redb: ACID-compliant redb storage (deprecated, use Sqlite)
    ///
    /// Default: Sqlite
    #[serde(default)]
    pub storage_backend: StorageBackend,

    /// Path for redb-backed Raft log database.
    /// Only used when storage_backend = Redb.
    /// Defaults to "{data_dir}/raft-log.redb" if not specified.
    pub redb_log_path: Option<PathBuf>,

    /// Path for redb-backed state machine database.
    /// Only used when storage_backend = Redb.
    /// Defaults to "{data_dir}/state-machine.redb" if not specified.
    pub redb_sm_path: Option<PathBuf>,

    /// Path for sqlite-backed Raft log database.
    /// Only used when storage_backend = Sqlite.
    /// Defaults to "{data_dir}/raft-log.db" if not specified.
    pub sqlite_log_path: Option<PathBuf>,

    /// Path for sqlite-backed state machine database.
    /// Only used when storage_backend = Sqlite.
    /// Defaults to "{data_dir}/state-machine.db" if not specified.
    pub sqlite_sm_path: Option<PathBuf>,

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

    /// Peer node addresses.
    /// Format: "node_id@endpoint_id:direct_addrs"
    #[serde(default)]
    pub peers: Vec<String>,
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
    /// If not provided, uses n0's public DNS service (https://dns.iroh.link).
    /// Only relevant when enable_dns_discovery is true.
    pub dns_discovery_url: Option<String>,

    /// Enable Pkarr publisher for distributed peer discovery.
    ///
    /// When enabled, nodes publish their addresses to a Pkarr relay (DHT-based).
    /// Other nodes can then discover them via DNS lookups.
    ///
    /// Default: false (Pkarr disabled).
    #[serde(default)]
    pub enable_pkarr: bool,
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
        }
    }
}

/// Control-plane backend implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ControlBackend {
    /// Deterministic in-memory implementation for testing.
    Deterministic,
    /// Production Raft-backed implementation.
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
            storage_backend: parse_env("ASPEN_STORAGE_BACKEND")
                .unwrap_or_else(StorageBackend::default),
            redb_log_path: parse_env("ASPEN_REDB_LOG_PATH"),
            redb_sm_path: parse_env("ASPEN_REDB_SM_PATH"),
            sqlite_log_path: parse_env("ASPEN_SQLITE_LOG_PATH"),
            sqlite_sm_path: parse_env("ASPEN_SQLITE_SM_PATH"),
            host: parse_env("ASPEN_HOST").unwrap_or_else(default_host),
            cookie: parse_env("ASPEN_COOKIE").unwrap_or_else(default_cookie),
            http_addr: parse_env("ASPEN_HTTP_ADDR").unwrap_or_else(default_http_addr),
            control_backend: parse_env("ASPEN_CONTROL_BACKEND")
                .unwrap_or(ControlBackend::default()),
            heartbeat_interval_ms: parse_env("ASPEN_HEARTBEAT_INTERVAL_MS")
                .unwrap_or_else(default_heartbeat_interval_ms),
            election_timeout_min_ms: parse_env("ASPEN_ELECTION_TIMEOUT_MIN_MS")
                .unwrap_or_else(default_election_timeout_min_ms),
            election_timeout_max_ms: parse_env("ASPEN_ELECTION_TIMEOUT_MAX_MS")
                .unwrap_or_else(default_election_timeout_max_ms),
            iroh: IrohConfig {
                secret_key: parse_env("ASPEN_IROH_SECRET_KEY"),
                enable_gossip: parse_env("ASPEN_IROH_ENABLE_GOSSIP")
                    .unwrap_or_else(default_enable_gossip),
                gossip_ticket: parse_env("ASPEN_IROH_GOSSIP_TICKET"),
                enable_mdns: parse_env("ASPEN_IROH_ENABLE_MDNS")
                    .unwrap_or_else(default_enable_mdns),
                enable_dns_discovery: parse_env("ASPEN_IROH_ENABLE_DNS_DISCOVERY").unwrap_or(false),
                dns_discovery_url: parse_env("ASPEN_IROH_DNS_DISCOVERY_URL"),
                enable_pkarr: parse_env("ASPEN_IROH_ENABLE_PKARR").unwrap_or(false),
            },
            peers: parse_env_vec("ASPEN_PEERS"),
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
        // so InMemory is preserved instead of being overwritten by default Sqlite
        if other.storage_backend != StorageBackend::default() {
            self.storage_backend = other.storage_backend;
        }
        if other.redb_log_path.is_some() {
            self.redb_log_path = other.redb_log_path;
        }
        if other.redb_sm_path.is_some() {
            self.redb_sm_path = other.redb_sm_path;
        }
        if other.sqlite_log_path.is_some() {
            self.sqlite_log_path = other.sqlite_log_path;
        }
        if other.sqlite_sm_path.is_some() {
            self.sqlite_sm_path = other.sqlite_sm_path;
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
        if !other.peers.is_empty() {
            self.peers = other.peers;
        }
        // supervision_config removed - was legacy from actor-based architecture
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
        use crate::cluster::validation::{
            check_cookie_safety, check_disk_usage, check_http_port, check_raft_timing_sanity,
            validate_cookie, validate_node_id, validate_raft_timings, validate_secret_key,
        };
        use tracing::warn;

        // Validate core fields using extracted pure functions
        validate_node_id(self.node_id).map_err(|e| ConfigError::Validation {
            message: e.to_string(),
        })?;

        validate_cookie(&self.cookie).map_err(|e| ConfigError::Validation {
            message: e.to_string(),
        })?;

        // Check for unsafe default cookie (security warning)
        if let Some(warning) = check_cookie_safety(&self.cookie) {
            warn!("{}", warning);
        }

        validate_raft_timings(
            self.heartbeat_interval_ms,
            self.election_timeout_min_ms,
            self.election_timeout_max_ms,
        )
        .map_err(|e| ConfigError::Validation {
            message: e.to_string(),
        })?;

        validate_secret_key(self.iroh.secret_key.as_deref()).map_err(|e| {
            ConfigError::Validation {
                message: e.to_string(),
            }
        })?;

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
                    message: format!(
                        "data_dir parent directory does not exist: {}",
                        parent.display()
                    ),
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
        for (name, path) in [
            ("redb_log_path", &self.redb_log_path),
            ("redb_sm_path", &self.redb_sm_path),
            ("sqlite_log_path", &self.sqlite_log_path),
            ("sqlite_sm_path", &self.sqlite_sm_path),
        ] {
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
                        message: format!(
                            "{} path {} exists but is a directory, expected a file",
                            name,
                            path.display()
                        ),
                    });
                }
            }
        }

        // Network port validation (using extracted pure function)
        if let Some(warning) = check_http_port(self.http_addr.port()) {
            warn!(http_addr = %self.http_addr, "{}", warning);
        }

        Ok(())
    }

    /// Get the data directory, using the default if not specified.
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("./data/node-{}", self.node_id)))
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
    SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        8080,
    )
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
    #[snafu(display("failed to read config file {}: {source}", path.display()))]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("failed to parse TOML config file {}: {source}", path.display()))]
    ParseToml {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[snafu(display("configuration validation failed: {message}"))]
    Validation { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::default(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::default(),
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.data_dir(), PathBuf::from("./data/node-1"));
    }

    #[test]
    fn test_validation_node_id_zero() {
        let config = NodeConfig {
            node_id: 0,
            data_dir: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::default(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::default(),
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_election_timeout() {
        let config = NodeConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::default(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: 3000,
            election_timeout_max_ms: 1500,
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::default(),
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    #[allow(deprecated)]
    fn test_merge() {
        let mut base = NodeConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::RaftActor, // Default value
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::Sqlite, // Default value
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
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
            },
            peers: vec!["peer1".into()],
            storage_backend: crate::raft::storage::StorageBackend::InMemory, // Non-default: should override
            redb_log_path: Some(PathBuf::from("/custom/raft-log.redb")),
            redb_sm_path: Some(PathBuf::from("/custom/state-machine.redb")),
            sqlite_log_path: None,
            sqlite_sm_path: None,
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
        // InMemory (non-default) should override Sqlite (default)
        assert_eq!(
            base.storage_backend,
            crate::raft::storage::StorageBackend::InMemory
        );
    }

    #[test]
    fn test_merge_preserves_explicit_base_values() {
        // Test that default override values don't clobber explicit base values
        // This tests the layered config precedence fix
        let mut base = NodeConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::Deterministic, // Explicit non-default
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::InMemory, // Explicit non-default
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
        };

        // Override config uses DEFAULT values - should NOT override base
        let override_config = NodeConfig {
            node_id: 0, // Default: 0 doesn't override
            data_dir: None,
            host: default_host(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::RaftActor, // Default: should NOT override
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: crate::raft::storage::StorageBackend::Sqlite, // Default: should NOT override
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
        };

        base.merge(override_config);

        // Original non-default values should be preserved (not clobbered by defaults)
        assert_eq!(base.node_id, 1); // Preserved (0 is "unset")
        assert_eq!(base.control_backend, ControlBackend::Deterministic); // Preserved
        assert_eq!(
            base.storage_backend,
            crate::raft::storage::StorageBackend::InMemory
        ); // Preserved
    }
}
