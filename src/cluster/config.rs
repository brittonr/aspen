use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

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
    /// When enabled, nodes broadcast their presence and discover peers automatically.
    /// When disabled, only manual peers (from --peers) are used.
    /// Default: true (gossip enabled).
    #[serde(default = "default_enable_gossip")]
    pub enable_gossip: bool,

    /// Aspen cluster ticket for gossip-based bootstrap.
    /// Contains the gossip topic ID and bootstrap peer endpoints.
    /// Format: "aspen{base32-encoded-data}"
    /// If provided, overrides manual peers for gossip bootstrap.
    pub gossip_ticket: Option<String>,
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            relay_url: None,
            enable_gossip: default_enable_gossip(),
            gossip_ticket: None,
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
        if !other.peers.is_empty() {
            self.peers = other.peers;
        }
    }

    /// Validate the configuration.
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.node_id == 0 {
            return Err(ConfigError::Validation {
                message: "node_id must be non-zero".into(),
            });
        }

        if self.heartbeat_interval_ms == 0 {
            return Err(ConfigError::Validation {
                message: "heartbeat_interval_ms must be non-zero".into(),
            });
        }

        if self.election_timeout_min_ms == 0 {
            return Err(ConfigError::Validation {
                message: "election_timeout_min_ms must be non-zero".into(),
            });
        }

        if self.election_timeout_max_ms <= self.election_timeout_min_ms {
            return Err(ConfigError::Validation {
                message: "election_timeout_max_ms must be greater than election_timeout_min_ms"
                    .into(),
            });
        }

        // Validate Iroh secret key format if provided
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
    "127.0.0.1:8080".parse().unwrap()
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
        };

        assert!(config.validate().is_err());
    }

    #[test]
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
            },
            peers: vec!["peer1".into()],
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
        assert_eq!(base.peers, vec!["peer1"]);
    }
}
