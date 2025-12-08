//! Cluster configuration types and validation.
//!
//! Defines configuration structures for Aspen cluster nodes, supporting multi-layer
//! configuration loading from environment variables, TOML files, and command-line
//! arguments. Configuration precedence is: environment < TOML < CLI args, ensuring
//! operator overrides always take priority.
//!
//! # Key Components
//!
//! - `ClusterBootstrapConfig`: Top-level node configuration (node_id, data_dir, addresses)
//! - `ControlBackend`: Control plane selection (Raft or Hiqlite)
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
//! use aspen::cluster::config::ClusterBootstrapConfig;
//!
//! // From TOML
//! let config: ClusterBootstrapConfig = toml::from_str(&toml_str)?;
//!
//! // Programmatic
//! let config = ClusterBootstrapConfig {
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
use crate::raft::supervision::SupervisionConfig;

/// Bootstrap configuration for an Aspen cluster node.
///
/// Configuration is loaded in layers with the following precedence (lowest to highest):
/// 1. Environment variables (ASPEN_*)
/// 2. TOML configuration file
/// 3. Command-line arguments
///
/// This means CLI args override TOML config, which overrides environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterBootstrapConfig {
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

    /// Hostname recorded in the NodeServer's identity (informational).
    #[serde(default = "default_host")]
    pub host: String,

    /// Port for the Ractor node listener.
    /// Use 0 to request an OS-assigned port.
    #[serde(default = "default_ractor_port")]
    pub ractor_port: u16,

    /// Shared cookie for authenticating Ractor nodes.
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
    /// Format: "node_id@endpoint_id:relay_url:direct_addrs"
    #[serde(default)]
    pub peers: Vec<String>,

    /// Supervision configuration for RaftActor.
    #[serde(default)]
    pub supervision_config: SupervisionConfig,

    /// RaftActor mailbox capacity (bounded mailbox size).
    /// Prevents memory exhaustion under high load by enforcing backpressure.
    /// Default: 1000 messages
    /// Maximum: 10000 messages
    #[serde(default = "default_raft_mailbox_capacity")]
    pub raft_mailbox_capacity: u32,
}

fn default_raft_mailbox_capacity() -> u32 {
    1000
}

/// Iroh networking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConfig {
    /// Hex-encoded Iroh secret key (64 hex characters = 32 bytes).
    /// If not provided, a new key is generated.
    pub secret_key: Option<String>,

    /// Iroh relay server URL.
    pub relay_url: Option<String>,

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

    /// Custom Pkarr relay URL.
    ///
    /// If not provided, uses n0's public Pkarr service.
    /// Only relevant when enable_pkarr is true.
    pub pkarr_relay_url: Option<String>,
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            relay_url: None,
            enable_gossip: default_enable_gossip(),
            gossip_ticket: None,
            enable_mdns: default_enable_mdns(),
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
            pkarr_relay_url: None,
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

impl ClusterBootstrapConfig {
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
            ractor_port: parse_env("ASPEN_RACTOR_PORT").unwrap_or_else(default_ractor_port),
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
                relay_url: parse_env("ASPEN_IROH_RELAY_URL"),
                enable_gossip: parse_env("ASPEN_IROH_ENABLE_GOSSIP")
                    .unwrap_or_else(default_enable_gossip),
                gossip_ticket: parse_env("ASPEN_IROH_GOSSIP_TICKET"),
                enable_mdns: parse_env("ASPEN_IROH_ENABLE_MDNS")
                    .unwrap_or_else(default_enable_mdns),
                enable_dns_discovery: parse_env("ASPEN_IROH_ENABLE_DNS_DISCOVERY").unwrap_or(false),
                dns_discovery_url: parse_env("ASPEN_IROH_DNS_DISCOVERY_URL"),
                enable_pkarr: parse_env("ASPEN_IROH_ENABLE_PKARR").unwrap_or(false),
                pkarr_relay_url: parse_env("ASPEN_IROH_PKARR_RELAY_URL"),
            },
            peers: parse_env_vec("ASPEN_PEERS"),
            supervision_config: SupervisionConfig::default(),
            raft_mailbox_capacity: parse_env("ASPEN_RAFT_MAILBOX_CAPACITY")
                .unwrap_or_else(default_raft_mailbox_capacity),
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
        if other.ractor_port != default_ractor_port() {
            self.ractor_port = other.ractor_port;
        }
        if other.cookie != default_cookie() {
            self.cookie = other.cookie;
        }
        if other.http_addr != default_http_addr() {
            self.http_addr = other.http_addr;
        }
        self.control_backend = other.control_backend;
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
        if other.iroh.relay_url.is_some() {
            self.iroh.relay_url = other.iroh.relay_url;
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
        if other.iroh.pkarr_relay_url.is_some() {
            self.iroh.pkarr_relay_url = other.iroh.pkarr_relay_url;
        }
        if !other.peers.is_empty() {
            self.peers = other.peers;
        }
        // Always merge supervision config
        self.supervision_config = other.supervision_config;
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

        // Required fields
        if self.node_id == 0 {
            return Err(ConfigError::Validation {
                message: "node_id must be non-zero".into(),
            });
        }

        if self.cookie.is_empty() {
            return Err(ConfigError::Validation {
                message: "cluster cookie cannot be empty".into(),
            });
        }

        // Numeric ranges
        if self.heartbeat_interval_ms == 0 {
            return Err(ConfigError::Validation {
                message: "heartbeat_interval_ms must be greater than 0".into(),
            });
        }

        if self.election_timeout_min_ms == 0 {
            return Err(ConfigError::Validation {
                message: "election_timeout_min_ms must be greater than 0".into(),
            });
        }

        if self.election_timeout_max_ms == 0 {
            return Err(ConfigError::Validation {
                message: "election_timeout_max_ms must be greater than 0".into(),
            });
        }

        if self.election_timeout_max_ms <= self.election_timeout_min_ms {
            return Err(ConfigError::Validation {
                message: "election_timeout_max_ms must be greater than election_timeout_min_ms"
                    .into(),
            });
        }

        // Raft sanity checks (warnings)
        if self.heartbeat_interval_ms >= self.election_timeout_min_ms {
            warn!(
                heartbeat_interval_ms = self.heartbeat_interval_ms,
                election_timeout_min_ms = self.election_timeout_min_ms,
                "heartbeat interval should be less than election timeout (Raft requirement)"
            );
        }

        if self.election_timeout_min_ms < 1000 {
            warn!(
                election_timeout_min_ms = self.election_timeout_min_ms,
                "election timeout < 1000ms may be too aggressive for production"
            );
        }

        if self.election_timeout_max_ms > 10000 {
            warn!(
                election_timeout_max_ms = self.election_timeout_max_ms,
                "election timeout > 10000ms may be too conservative"
            );
        }

        // File path validation
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
                    if disk_space.usage_percent > 80 {
                        warn!(
                            data_dir = %data_dir.display(),
                            usage_percent = disk_space.usage_percent,
                            available_gb = disk_space.available_bytes / (1024 * 1024 * 1024),
                            "disk space usage high (>80%)"
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

        // Network port validation (warn if using default ports)
        if self.http_addr.port() == 8080 {
            warn!(
                http_addr = %self.http_addr,
                "using default HTTP port 8080 (may conflict in production)"
            );
        }

        if self.ractor_port == 26000 {
            warn!(
                ractor_port = self.ractor_port,
                "using default ractor port 26000 (may conflict in production)"
            );
        }

        // Iroh secret key validation
        if let Some(ref key_hex) = self.iroh.secret_key {
            if key_hex.len() != 64 {
                return Err(ConfigError::Validation {
                    message: "iroh secret key must be 64 hex characters (32 bytes)".into(),
                });
            }
            if hex::decode(key_hex).is_err() {
                return Err(ConfigError::Validation {
                    message: "iroh secret key must be valid hex".into(),
                });
            }
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

fn default_ractor_port() -> u16 {
    26000
}

fn default_cookie() -> String {
    "aspen-cookie".into()
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
        let config = ClusterBootstrapConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            ractor_port: default_ractor_port(),
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
            supervision_config: SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.data_dir(), PathBuf::from("./data/node-1"));
    }

    #[test]
    fn test_validation_node_id_zero() {
        let config = ClusterBootstrapConfig {
            node_id: 0,
            data_dir: None,
            host: default_host(),
            ractor_port: default_ractor_port(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::default(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            election_timeout_min_ms: default_election_timeout_min_ms(),
            election_timeout_max_ms: default_election_timeout_max_ms(),
            iroh: IrohConfig::default(),
            peers: vec![],
            supervision_config: SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
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
        let config = ClusterBootstrapConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            ractor_port: default_ractor_port(),
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
            supervision_config: SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    #[allow(deprecated)]
    fn test_merge() {
        let mut base = ClusterBootstrapConfig {
            node_id: 1,
            data_dir: None,
            host: default_host(),
            ractor_port: default_ractor_port(),
            cookie: default_cookie(),
            http_addr: default_http_addr(),
            control_backend: ControlBackend::Deterministic,
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
            supervision_config: SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
        };

        let override_config = ClusterBootstrapConfig {
            node_id: 2,
            data_dir: Some(PathBuf::from("/custom/data")),
            host: "192.168.1.1".into(),
            ractor_port: 26001,
            cookie: "custom-cookie".into(),
            http_addr: "0.0.0.0:9090".parse().unwrap(),
            control_backend: ControlBackend::RaftActor,
            heartbeat_interval_ms: 1000,
            election_timeout_min_ms: 2000,
            election_timeout_max_ms: 4000,
            iroh: IrohConfig {
                secret_key: Some("a".repeat(64)),
                relay_url: Some("https://relay.example.com".into()),
                enable_gossip: false,
                gossip_ticket: Some("test-ticket".into()),
                enable_mdns: false,
                enable_dns_discovery: true,
                dns_discovery_url: Some("https://dns.example.com".into()),
                enable_pkarr: true,
                pkarr_relay_url: Some("https://pkarr.example.com".into()),
            },
            peers: vec!["peer1".into()],
            storage_backend: crate::raft::storage::StorageBackend::Redb,
            redb_log_path: Some(PathBuf::from("/custom/raft-log.redb")),
            redb_sm_path: Some(PathBuf::from("/custom/state-machine.redb")),
            sqlite_log_path: None,
            sqlite_sm_path: None,
            supervision_config: SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
        };

        base.merge(override_config);

        assert_eq!(base.node_id, 2);
        assert_eq!(base.data_dir, Some(PathBuf::from("/custom/data")));
        assert_eq!(base.host, "192.168.1.1");
        assert_eq!(base.ractor_port, 26001);
        assert_eq!(base.cookie, "custom-cookie");
        assert_eq!(base.http_addr, "0.0.0.0:9090".parse().unwrap());
        assert_eq!(base.control_backend, ControlBackend::RaftActor);
        assert_eq!(base.heartbeat_interval_ms, 1000);
        assert_eq!(base.election_timeout_min_ms, 2000);
        assert_eq!(base.election_timeout_max_ms, 4000);
        assert_eq!(base.iroh.secret_key, Some("a".repeat(64)));
        assert_eq!(
            base.iroh.relay_url,
            Some("https://relay.example.com".into())
        );
        assert!(!base.iroh.enable_gossip);
        assert_eq!(base.iroh.gossip_ticket, Some("test-ticket".into()));
        assert_eq!(base.peers, vec!["peer1"]);
    }
}
