//! CLI argument parsing for aspen-node.
//!
//! Defines the `Args` struct using clap derive for command-line argument parsing.

use std::path::PathBuf;

use aspen::cluster::config::ControlBackend;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "aspen-node")]
pub struct Args {
    /// Path to TOML configuration file.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Logical Raft node identifier.
    #[arg(long)]
    pub node_id: Option<u64>,

    /// Directory for persistent data storage (metadata, Raft logs, state machine).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    /// Storage backend for Raft log and state machine.
    /// Options: "inmemory", "redb" (default)
    #[arg(long)]
    pub storage_backend: Option<String>,

    /// Path for redb-backed Raft log database.
    /// Only used when storage_backend = redb.
    /// Defaults to "{data_dir}/raft-log.redb" if not specified.
    #[arg(long)]
    pub redb_log_path: Option<PathBuf>,

    /// Path for redb-backed state machine database.
    /// Only used when storage_backend = redb.
    /// Defaults to "{data_dir}/state-machine.redb" if not specified.
    #[arg(long)]
    pub redb_sm_path: Option<PathBuf>,

    /// Hostname for informational purposes.
    #[arg(long)]
    pub host: Option<String>,

    /// Shared cookie for cluster authentication.
    #[arg(long)]
    pub cookie: Option<String>,

    /// Control-plane implementation to use for this node.
    #[arg(long)]
    pub control_backend: Option<ControlBackend>,

    /// Raft heartbeat interval in milliseconds.
    #[arg(long)]
    pub heartbeat_interval_ms: Option<u64>,

    /// Minimum Raft election timeout in milliseconds.
    #[arg(long)]
    pub election_timeout_min_ms: Option<u64>,

    /// Maximum Raft election timeout in milliseconds.
    #[arg(long)]
    pub election_timeout_max_ms: Option<u64>,

    /// Optional Iroh secret key (hex-encoded). If not provided, a new key is generated.
    #[arg(long)]
    pub iroh_secret_key: Option<String>,

    /// Disable iroh-gossip for automatic peer discovery.
    /// When disabled, only manual peers (from --peers) are used.
    /// Default: gossip is enabled.
    #[arg(long)]
    pub disable_gossip: bool,

    /// Aspen cluster ticket for gossip-based bootstrap.
    /// Contains the gossip topic ID and bootstrap peer endpoints.
    /// Format: "aspen{base32-encoded-data}"
    #[arg(long)]
    pub ticket: Option<String>,

    /// Disable mDNS discovery for local network peer discovery.
    /// Default: mDNS is enabled.
    #[arg(long)]
    pub disable_mdns: bool,

    /// Enable DNS discovery for production peer discovery.
    /// Uses n0's public DNS service by default, or custom URL if --dns-discovery-url is provided.
    /// Default: DNS discovery is disabled.
    #[arg(long)]
    pub enable_dns_discovery: bool,

    /// Custom DNS discovery service URL.
    /// Only relevant when --enable-dns-discovery is set.
    #[arg(long)]
    pub dns_discovery_url: Option<String>,

    /// Enable Pkarr publisher for distributed peer discovery.
    /// Publishes node addresses to a Pkarr relay (DHT-based).
    /// Default: Pkarr is disabled.
    #[arg(long)]
    pub enable_pkarr: bool,

    /// Custom Pkarr relay URL for discovery.
    /// For private infrastructure, run your own pkarr relay and set this URL.
    /// Only relevant when --enable-pkarr is set.
    #[arg(long)]
    pub pkarr_relay_url: Option<String>,

    /// Port to bind for QUIC connections.
    /// - 0: Use random port (default)
    /// - Other: Use specific port (e.g., 7777 for VM deployments)
    #[arg(long)]
    pub bind_port: Option<u16>,

    /// Relay server mode: "default", "custom", or "disabled".
    /// - default: Use n0's public relay infrastructure (default)
    /// - custom: Use your own relay servers (requires --relay-url)
    /// - disabled: No relays, direct connections only
    #[arg(long)]
    pub relay_mode: Option<String>,

    /// Custom relay server URLs for connection facilitation.
    /// Required when --relay-mode=custom. Recommended to have 2+ for redundancy.
    /// Can be specified multiple times.
    #[arg(long)]
    pub relay_url: Vec<String>,

    /// Enable HMAC-SHA256 authentication for Raft RPC.
    /// When enabled, nodes perform mutual authentication using the cluster
    /// cookie before accepting Raft RPC requests.
    /// Default: Raft auth is disabled.
    #[arg(long)]
    pub enable_raft_auth: bool,

    /// Enable capability-based token authentication for Client RPC.
    /// When enabled, clients must provide valid capability tokens for
    /// authorized operations (read, write, delete, admin).
    /// Default: Token auth is disabled.
    #[arg(long)]
    pub enable_token_auth: bool,

    /// Require valid tokens for all authorized requests.
    /// Only relevant when --enable-token-auth is set.
    /// When false (default), missing tokens produce warnings but requests proceed.
    /// When true, requests without valid tokens are rejected with 401 Unauthorized.
    #[arg(long)]
    pub require_token_auth: bool,

    /// Trusted root issuer public keys for capability tokens.
    /// Only tokens signed by these keys (or delegated from them) are accepted.
    /// Format: hex-encoded Ed25519 public key (32 bytes = 64 hex chars).
    /// Can be specified multiple times for multiple trusted roots.
    /// If empty, the node's own Iroh public key is used as the trusted root.
    #[arg(long)]
    pub trusted_root_key: Vec<String>,

    /// Output root token to file during cluster initialization.
    /// Only generates a token when initializing a NEW cluster (not joining existing).
    /// The token will have full cluster access (all keys, admin, delegation).
    #[arg(long)]
    pub output_root_token: Option<PathBuf>,

    /// Peer node addresses in format: node_id@addr. Example: `"1@node-id:direct-addrs"`
    /// Can be specified multiple times for multiple peers.
    #[arg(long)]
    pub peers: Vec<String>,

    /// Path to SOPS-encrypted secrets file.
    /// Contains trusted roots, signing key, and pre-built capability tokens.
    /// Format: TOML encrypted with age via SOPS.
    #[cfg(feature = "secrets")]
    #[arg(long)]
    pub secrets_file: Option<PathBuf>,

    /// Path to age identity file for decrypting SOPS secrets.
    /// Defaults to $XDG_CONFIG_HOME/sops/age/keys.txt if not specified.
    #[cfg(feature = "secrets")]
    #[arg(long)]
    pub age_identity_file: Option<PathBuf>,

    // === Worker Configuration ===
    /// Enable job workers on this node.
    ///
    /// When enabled, the node starts a worker pool to process jobs
    /// from the distributed queue. Workers execute CI pipelines,
    /// maintenance tasks, and other scheduled work.
    #[arg(long)]
    pub enable_workers: bool,

    /// Number of workers to start (default: CPU count, max: 64).
    ///
    /// Each worker can process one job at a time. More workers allow
    /// parallel job execution but consume more resources.
    #[arg(long)]
    pub worker_count: Option<usize>,

    /// Job types this worker handles (empty = all).
    ///
    /// When specified, this worker only accepts jobs matching these types.
    /// Examples: "ci_build", "maintenance", "nix_build"
    #[arg(long)]
    pub worker_job_types: Vec<String>,

    // === CI/CD Configuration ===
    /// Enable CI/CD pipeline orchestration.
    ///
    /// When enabled, the node can receive pipeline trigger requests and
    /// orchestrate pipeline execution using the job system. Requires
    /// workers to be enabled for actual job execution.
    #[arg(long)]
    pub enable_ci: bool,

    /// Enable automatic CI triggering on ref updates.
    ///
    /// When enabled alongside --enable-ci, the node watches for forge
    /// gossip events and automatically triggers CI for repositories with
    /// `.aspen/ci.ncl` configurations.
    #[arg(long)]
    pub ci_auto_trigger: bool,

    // === Worker-Only Mode ===
    /// Run as an ephemeral CI worker without Raft consensus.
    ///
    /// In worker-only mode, the node:
    /// - Connects to an existing cluster via ticket (required)
    /// - Does NOT participate in Raft consensus
    /// - Processes CI jobs and uploads artifacts to SNIX
    /// - Uses RPC to access cluster services (KV, SNIX metadata)
    ///
    /// This mode is designed for CI VMs that need full SNIX access
    /// but shouldn't be cluster consensus participants.
    ///
    /// Requires: --ticket (cluster connection ticket)
    /// Also configurable via: ASPEN_MODE=ci_worker
    #[arg(long)]
    pub worker_only: bool,
}
