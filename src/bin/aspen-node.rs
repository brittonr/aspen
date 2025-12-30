//! Aspen node binary - cluster node entry point.
//!
//! Production binary for running Aspen cluster nodes with Iroh Client RPC for
//! all cluster operations and key-value access. Supports Raft control plane backend
//! and both in-memory and persistent storage. Configuration is loaded from
//! environment variables, TOML files, or CLI arguments.
//!
//! # Architecture
//!
//! - Iroh Client RPC: All client API operations via QUIC/P2P
//! - Raft control plane: Distributed consensus for cluster management
//! - Graceful shutdown: SIGTERM/SIGINT handling with coordinated cleanup
//! - Configuration layers: Environment < TOML < CLI args
//!
//! # Client API (Iroh Client RPC)
//!
//! All client operations are accessed via Iroh Client RPC (ALPN: `aspen-tui`).
//! See `ClientRpcRequest` in `src/client_rpc.rs` for the full API.
//!
//! Control Plane:
//! - InitCluster - Initialize new cluster
//! - AddLearner - Add learner node
//! - ChangeMembership - Promote learners to voters
//! - GetClusterState - Get current cluster topology
//!
//! Key-Value:
//! - ReadKey - Read key (linearizable)
//! - WriteKey - Write key-value (replicated)
//! - DeleteKey - Delete key
//! - ScanKeys - Scan keys by prefix
//!
//! Monitoring:
//! - GetHealth - Health check
//! - GetMetrics - Prometheus-compatible metrics
//! - GetRaftMetrics - Detailed Raft metrics
//!
//! # Tiger Style
//!
//! - Explicit types: u64 for node_id, addresses are type-safe
//! - Fixed limits: Raft batch sizes are bounded
//! - Resource management: Arc for shared state, graceful shutdown cleans up
//! - Error handling: Anyhow for application errors
//! - Fail fast: Configuration validation before server starts
//!
//! # Usage
//!
//! ```bash
//! # Start node with TOML config
//! aspen-node --config /etc/aspen/node.toml
//!
//! # Start node with CLI args
//! aspen-node --node-id 1
//!
//! # Environment variables
//! export ASPEN_NODE_ID=1
//! aspen-node
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use aspen::api::ClusterController;
use aspen::api::DeterministicClusterController;
use aspen::api::DeterministicKeyValueStore;
use aspen::api::KeyValueStore;
use aspen::auth::TokenVerifier;
use aspen::cluster::bootstrap::NodeHandle;
use aspen::cluster::bootstrap::ShardedNodeHandle;
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::bootstrap::bootstrap_sharded_node;
use aspen::cluster::bootstrap::load_config;
use aspen::cluster::config::ControlBackend;
use aspen::cluster::config::IrohConfig;
use aspen::cluster::config::NodeConfig;
#[cfg(feature = "dns")]
use aspen::dns::AspenDnsClient;
#[cfg(feature = "dns")]
use aspen::dns::DnsProtocolServer;
#[cfg(feature = "dns")]
use aspen::dns::spawn_dns_sync_listener;
use aspen::ClientProtocolContext;
use aspen::ClientProtocolHandler;
use aspen::LOG_SUBSCRIBER_ALPN;
use aspen::LogSubscriberProtocolHandler;
use aspen::RAFT_SHARDED_ALPN;
use aspen::RaftProtocolHandler;
use clap::Parser;
use iroh::PublicKey;
use tokio::signal;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "aspen-node")]
struct Args {
    /// Path to TOML configuration file.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Logical Raft node identifier.
    #[arg(long)]
    node_id: Option<u64>,

    /// Directory for persistent data storage (metadata, Raft logs, state machine).
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Storage backend for Raft log and state machine.
    /// Options: "inmemory", "redb", "sqlite" (default)
    #[arg(long)]
    storage_backend: Option<String>,

    /// Path for redb-backed Raft log database.
    /// Only used when storage_backend = redb.
    /// Defaults to "{data_dir}/raft-log.redb" if not specified.
    #[arg(long)]
    redb_log_path: Option<PathBuf>,

    /// Path for redb-backed state machine database.
    /// Only used when storage_backend = redb.
    /// Defaults to "{data_dir}/state-machine.redb" if not specified.
    #[arg(long)]
    redb_sm_path: Option<PathBuf>,

    /// Hostname for informational purposes.
    #[arg(long)]
    host: Option<String>,

    /// Shared cookie for cluster authentication.
    #[arg(long)]
    cookie: Option<String>,

    /// Control-plane implementation to use for this node.
    #[arg(long)]
    control_backend: Option<ControlBackend>,

    /// Raft heartbeat interval in milliseconds.
    #[arg(long)]
    heartbeat_interval_ms: Option<u64>,

    /// Minimum Raft election timeout in milliseconds.
    #[arg(long)]
    election_timeout_min_ms: Option<u64>,

    /// Maximum Raft election timeout in milliseconds.
    #[arg(long)]
    election_timeout_max_ms: Option<u64>,

    /// Optional Iroh secret key (hex-encoded). If not provided, a new key is generated.
    #[arg(long)]
    iroh_secret_key: Option<String>,

    /// Disable iroh-gossip for automatic peer discovery.
    /// When disabled, only manual peers (from --peers) are used.
    /// Default: gossip is enabled.
    #[arg(long)]
    disable_gossip: bool,

    /// Aspen cluster ticket for gossip-based bootstrap.
    /// Contains the gossip topic ID and bootstrap peer endpoints.
    /// Format: "aspen{base32-encoded-data}"
    #[arg(long)]
    ticket: Option<String>,

    /// Disable mDNS discovery for local network peer discovery.
    /// Default: mDNS is enabled.
    #[arg(long)]
    disable_mdns: bool,

    /// Enable DNS discovery for production peer discovery.
    /// Uses n0's public DNS service by default, or custom URL if --dns-discovery-url is provided.
    /// Default: DNS discovery is disabled.
    #[arg(long)]
    enable_dns_discovery: bool,

    /// Custom DNS discovery service URL.
    /// Only relevant when --enable-dns-discovery is set.
    #[arg(long)]
    dns_discovery_url: Option<String>,

    /// Enable Pkarr publisher for distributed peer discovery.
    /// Publishes node addresses to a Pkarr relay (DHT-based).
    /// Default: Pkarr is disabled.
    #[arg(long)]
    enable_pkarr: bool,

    /// Custom Pkarr relay URL for discovery.
    /// For private infrastructure, run your own pkarr relay and set this URL.
    /// Only relevant when --enable-pkarr is set.
    #[arg(long)]
    pkarr_relay_url: Option<String>,

    /// Relay server mode: "default", "custom", or "disabled".
    /// - default: Use n0's public relay infrastructure (default)
    /// - custom: Use your own relay servers (requires --relay-url)
    /// - disabled: No relays, direct connections only
    #[arg(long)]
    relay_mode: Option<String>,

    /// Custom relay server URLs for connection facilitation.
    /// Required when --relay-mode=custom. Recommended to have 2+ for redundancy.
    /// Can be specified multiple times.
    #[arg(long)]
    relay_url: Vec<String>,

    /// Enable HMAC-SHA256 authentication for Raft RPC.
    /// When enabled, nodes perform mutual authentication using the cluster
    /// cookie before accepting Raft RPC requests.
    /// Default: Raft auth is disabled.
    #[arg(long)]
    enable_raft_auth: bool,

    /// Enable capability-based token authentication for Client RPC.
    /// When enabled, clients must provide valid capability tokens for
    /// authorized operations (read, write, delete, admin).
    /// Default: Token auth is disabled.
    #[arg(long)]
    enable_token_auth: bool,

    /// Require valid tokens for all authorized requests.
    /// Only relevant when --enable-token-auth is set.
    /// When false (default), missing tokens produce warnings but requests proceed.
    /// When true, requests without valid tokens are rejected with 401 Unauthorized.
    #[arg(long)]
    require_token_auth: bool,

    /// Trusted root issuer public keys for capability tokens.
    /// Only tokens signed by these keys (or delegated from them) are accepted.
    /// Format: hex-encoded Ed25519 public key (32 bytes = 64 hex chars).
    /// Can be specified multiple times for multiple trusted roots.
    /// If empty, the node's own Iroh public key is used as the trusted root.
    #[arg(long)]
    trusted_root_key: Vec<String>,

    /// Output root token to file during cluster initialization.
    /// Only generates a token when initializing a NEW cluster (not joining existing).
    /// The token will have full cluster access (all keys, admin, delegation).
    #[arg(long)]
    output_root_token: Option<PathBuf>,

    /// Peer node addresses in format: node_id@addr. Example: `"1@node-id:direct-addrs"`
    /// Can be specified multiple times for multiple peers.
    #[arg(long)]
    peers: Vec<String>,
}

type ClusterControllerHandle = Arc<dyn ClusterController>;
type KeyValueStoreHandle = Arc<dyn KeyValueStore>;

/// Initialize tracing subscriber with environment-based filtering.
///
/// Tiger Style: Focused initialization function.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

/// Build cluster configuration from CLI arguments.
///
/// Tiger Style: Focused function for config construction (single responsibility).
fn build_cluster_config(args: &Args) -> NodeConfig {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    let mut config = NodeConfig::from_env();
    config.node_id = args.node_id.unwrap_or(0);
    config.data_dir = args.data_dir.clone();
    config.storage_backend = args.storage_backend.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default();
    // Prefer redb_log_path if provided, fall back to redb_sm_path for backwards compat
    config.redb_path = args.redb_log_path.clone().or_else(|| args.redb_sm_path.clone());
    config.host = args.host.clone().unwrap_or_else(|| "127.0.0.1".into());
    config.cookie = args.cookie.clone().unwrap_or_else(|| "aspen-cookie".into());
    // HTTP API removed - set to localhost:0 (unused)
    config.http_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    config.control_backend = args.control_backend.unwrap_or_default();
    config.heartbeat_interval_ms = args.heartbeat_interval_ms.unwrap_or(500);
    config.election_timeout_min_ms = args.election_timeout_min_ms.unwrap_or(1500);
    config.election_timeout_max_ms = args.election_timeout_max_ms.unwrap_or(3000);
    // Parse relay mode from CLI string
    let relay_mode = args
        .relay_mode
        .as_deref()
        .map(|s| match s.to_lowercase().as_str() {
            "custom" => aspen::cluster::config::RelayMode::Custom,
            "disabled" => aspen::cluster::config::RelayMode::Disabled,
            _ => aspen::cluster::config::RelayMode::Default,
        })
        .unwrap_or_default();

    config.iroh = IrohConfig {
        secret_key: args.iroh_secret_key.clone(),
        enable_gossip: !args.disable_gossip,
        gossip_ticket: args.ticket.clone(),
        enable_mdns: !args.disable_mdns,
        enable_dns_discovery: args.enable_dns_discovery,
        dns_discovery_url: args.dns_discovery_url.clone(),
        enable_pkarr: args.enable_pkarr,
        enable_pkarr_dht: true,               // DHT enabled by default when pkarr is on
        enable_pkarr_relay: true,             // Relay enabled by default for fallback
        include_pkarr_direct_addresses: true, // Include direct IPs by default
        pkarr_republish_delay_secs: 600,      // 10 minutes default republish
        pkarr_relay_url: args.pkarr_relay_url.clone(),
        relay_mode,
        relay_urls: args.relay_url.clone(),
        enable_raft_auth: args.enable_raft_auth,
    };
    config.peers = args.peers.clone();

    // Apply security defaults (e.g., auto-enable raft_auth when pkarr is on)
    config.apply_security_defaults();

    config
}

/// Setup controllers based on control backend configuration.
///
/// Returns tuple of (controller, kv_store).
///
/// Tiger Style: Single responsibility for controller initialization logic.
fn setup_controllers(config: &NodeConfig, handle: &NodeHandle) -> (ClusterControllerHandle, KeyValueStoreHandle) {
    match config.control_backend {
        ControlBackend::Deterministic => (DeterministicClusterController::new(), DeterministicKeyValueStore::new()),
        ControlBackend::RaftActor => {
            // Use RaftNode directly as both controller and KV store
            let raft_node = handle.raft_node.clone();
            (raft_node.clone(), raft_node)
        }
    }
}

/// Unified node handle that wraps both sharded and non-sharded modes.
///
/// This enum allows main() to work uniformly with both bootstrap modes.
enum NodeMode {
    /// Single Raft node (legacy/default mode).
    Single(NodeHandle),
    /// Sharded node with multiple Raft instances.
    Sharded(ShardedNodeHandle),
}

impl NodeMode {
    fn iroh_manager(&self) -> &Arc<aspen::cluster::IrohEndpointManager> {
        match self {
            NodeMode::Single(h) => &h.iroh_manager,
            NodeMode::Sharded(h) => &h.base.iroh_manager,
        }
    }

    fn blob_store(&self) -> Option<&Arc<aspen::blob::IrohBlobStore>> {
        match self {
            NodeMode::Single(h) => h.blob_store.as_ref(),
            NodeMode::Sharded(h) => h.base.blob_store.as_ref(),
        }
    }

    fn peer_manager(&self) -> Option<&Arc<aspen::docs::PeerManager>> {
        match self {
            NodeMode::Single(h) => h.peer_manager.as_ref(),
            NodeMode::Sharded(h) => h.peer_manager.as_ref(),
        }
    }

    fn docs_sync(&self) -> Option<&aspen::docs::DocsSyncResources> {
        match self {
            NodeMode::Single(h) => h.docs_sync.as_ref().map(|arc| arc.as_ref()),
            NodeMode::Sharded(h) => h.docs_sync.as_ref().map(|arc| arc.as_ref()),
        }
    }

    fn log_broadcast(&self) -> Option<&tokio::sync::broadcast::Sender<aspen::raft::log_subscriber::LogEntryPayload>> {
        match self {
            NodeMode::Single(h) => h.log_broadcast.as_ref(),
            NodeMode::Sharded(h) => h.log_broadcast.as_ref(),
        }
    }

    #[cfg(feature = "global-discovery")]
    fn content_discovery(&self) -> Option<aspen::cluster::content_discovery::ContentDiscoveryService> {
        match self {
            NodeMode::Single(h) => h.content_discovery.clone(),
            // TODO: Add content_discovery support to ShardedNodeHandle
            NodeMode::Sharded(_) => None,
        }
    }

    async fn shutdown(self) -> Result<()> {
        match self {
            NodeMode::Single(h) => h.shutdown().await,
            NodeMode::Sharded(h) => h.shutdown().await,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();
    let cli_config = build_cluster_config(&args);

    // Load configuration with proper precedence (env < TOML < CLI)
    // Provide actionable error messages for common misconfigurations
    let config = match load_config(args.config.as_deref(), cli_config) {
        Ok(config) => config,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("node_id must be non-zero") {
                eprintln!("Error: node_id must be non-zero");
                eprintln!();
                eprintln!("Required configuration missing. To start an Aspen node, you must provide:");
                eprintln!();
                eprintln!("  1. A unique node ID (positive integer, e.g., 1, 2, 3)");
                eprintln!("  2. A cluster cookie (shared secret for cluster authentication)");
                eprintln!();
                eprintln!("Examples:");
                eprintln!();
                eprintln!("  # Using command-line arguments:");
                eprintln!("  aspen-node --node-id 1 --cookie my-cluster-secret");
                eprintln!();
                eprintln!("  # Using environment variables:");
                eprintln!("  export ASPEN_NODE_ID=1");
                eprintln!("  export ASPEN_COOKIE=my-cluster-secret");
                eprintln!("  aspen-node");
                eprintln!();
                eprintln!("  # Using a config file:");
                eprintln!("  aspen-node --config /path/to/config.toml");
                eprintln!();
                eprintln!("  Example config.toml:");
                eprintln!("    node_id = 1");
                eprintln!("    cookie = \"my-cluster-secret\"");
                eprintln!("    data_dir = \"./data/node-1\"");
                eprintln!();
                eprintln!("For a 3-node cluster, use: nix run .#cluster");
                std::process::exit(1);
            } else if error_msg.contains("default cluster cookie") || error_msg.contains("UNSAFE-CHANGE-ME") {
                eprintln!("Error: using default cluster cookie is not allowed");
                eprintln!();
                eprintln!("Security: You must set a unique cluster cookie to prevent");
                eprintln!("accidental cluster merges. All nodes in a cluster share the");
                eprintln!("same gossip topic derived from the cookie.");
                eprintln!();
                eprintln!("Set a unique cookie via:");
                eprintln!("  --cookie my-cluster-secret");
                eprintln!("  ASPEN_COOKIE=my-cluster-secret");
                eprintln!("  cookie = \"my-cluster-secret\" in config.toml");
                std::process::exit(1);
            } else {
                // For other errors, return the original error
                return Err(e);
            }
        }
    };

    info!(
        node_id = config.node_id,
        control_backend = ?config.control_backend,
        sharding_enabled = config.sharding.enabled,
        "starting aspen node"
    );

    // Bootstrap the node based on sharding configuration
    let (node_mode, controller, kv_store, primary_raft_node, network_factory) = if config.sharding.enabled {
        // Sharded mode: multiple Raft instances
        let mut sharded_handle = bootstrap_sharded_node(config.clone()).await?;

        // Generate and output root token if requested
        if let Some(ref token_path) = args.output_root_token {
            let secret_key = sharded_handle.base.iroh_manager.endpoint().secret_key();
            let token =
                aspen::auth::generate_root_token(secret_key, std::time::Duration::from_secs(365 * 24 * 60 * 60))
                    .context("failed to generate root token")?;

            let token_base64 = token.to_base64().context("failed to encode root token")?;
            std::fs::write(token_path, &token_base64)
                .with_context(|| format!("failed to write token to {}", token_path.display()))?;

            info!(
                token_path = %token_path.display(),
                issuer = %token.issuer,
                "root token written to file"
            );
            sharded_handle.root_token = Some(token);
        }

        // For sharded mode, use the ShardedKeyValueStore as the KV store
        // Use shard 0's RaftNode for ClusterController operations
        let kv_store: KeyValueStoreHandle = sharded_handle.sharded_kv.clone();
        let primary_shard = sharded_handle
            .primary_shard()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("shard 0 must be present in sharded mode"))?;
        let controller: ClusterControllerHandle = primary_shard.clone();
        let network_factory = sharded_handle.base.network_factory.clone();

        info!(
            num_shards = sharded_handle.shard_count(),
            local_shards = ?sharded_handle.local_shard_ids(),
            "sharded node bootstrap complete"
        );

        (NodeMode::Sharded(sharded_handle), controller, kv_store, primary_shard, network_factory)
    } else {
        // Non-sharded mode: single Raft instance
        let mut handle = bootstrap_node(config.clone()).await?;

        // Generate and output root token if requested
        if let Some(ref token_path) = args.output_root_token {
            let secret_key = handle.iroh_manager.endpoint().secret_key();
            let token =
                aspen::auth::generate_root_token(secret_key, std::time::Duration::from_secs(365 * 24 * 60 * 60))
                    .context("failed to generate root token")?;

            let token_base64 = token.to_base64().context("failed to encode root token")?;
            std::fs::write(token_path, &token_base64)
                .with_context(|| format!("failed to write token to {}", token_path.display()))?;

            info!(
                token_path = %token_path.display(),
                issuer = %token.issuer,
                "root token written to file"
            );
            handle.root_token = Some(token);
        }

        let (controller, kv_store) = setup_controllers(&config, &handle);
        let primary_raft_node = handle.raft_node.clone();
        let network_factory = handle.network_factory.clone();

        (NodeMode::Single(handle), controller, kv_store, primary_raft_node, network_factory)
    };

    // Create token verifier if authentication is enabled
    let token_verifier = if args.enable_token_auth {
        let mut verifier = TokenVerifier::new();

        if args.trusted_root_key.is_empty() {
            let node_public_key = node_mode.iroh_manager().endpoint().id();
            verifier = verifier.with_trusted_root(node_public_key);
            info!(
                trusted_root = %node_public_key,
                "Token auth enabled with node's own key as trusted root"
            );
        } else {
            for key_hex in &args.trusted_root_key {
                let key_bytes = hex::decode(key_hex).context("Invalid hex in --trusted-root-key")?;
                let key = PublicKey::try_from(key_bytes.as_slice())
                    .context("Invalid Ed25519 public key in --trusted-root-key")?;
                verifier = verifier.with_trusted_root(key);
                info!(trusted_root = %key, "Added trusted root key");
            }
        }

        if args.require_token_auth {
            info!("Token auth enabled with strict mode (requests without tokens will be rejected)");
        } else {
            warn!("Token auth enabled in migration mode (requests without tokens will be allowed with warnings)");
        }

        Some(Arc::new(verifier))
    } else {
        info!("Token auth disabled - all client requests are allowed");
        None
    };

    // Create Client protocol context and handler
    // Wrap docs_sync in Arc for sharing with the context
    let docs_sync_arc = node_mode.docs_sync().map(|ds| {
        Arc::new(aspen::docs::DocsSyncResources {
            sync_handle: ds.sync_handle.clone(),
            namespace_id: ds.namespace_id,
            author: ds.author.clone(),
            docs_dir: ds.docs_dir.clone(),
        })
    });

    // Initialize ForgeNode if blob_store is available
    #[cfg(feature = "forge")]
    let forge_node = node_mode.blob_store().map(|blob_store| {
        let secret_key = node_mode.iroh_manager().secret_key().clone();
        Arc::new(aspen::forge::ForgeNode::new(
            blob_store.clone(),
            kv_store.clone(),
            secret_key,
        ))
    });

    // Initialize PijulStore if blob_store and data_dir are available
    #[cfg(feature = "pijul")]
    let pijul_store = node_mode
        .blob_store()
        .and_then(|blob_store| {
            config.data_dir.as_ref().map(|data_dir| {
                Arc::new(aspen::pijul::PijulStore::new(
                    blob_store.clone(),
                    kv_store.clone(),
                    data_dir.clone(),
                ))
            })
        });

    let client_context = ClientProtocolContext {
        node_id: config.node_id,
        controller: controller.clone(),
        kv_store: kv_store.clone(),
        #[cfg(feature = "sql")]
        sql_executor: primary_raft_node.clone(),
        state_machine: Some(primary_raft_node.state_machine().clone()),
        endpoint_manager: node_mode.iroh_manager().clone(),
        blob_store: node_mode.blob_store().cloned(),
        peer_manager: node_mode.peer_manager().cloned(),
        docs_sync: docs_sync_arc,
        cluster_cookie: config.cookie.clone(),
        start_time: Instant::now(),
        network_factory: Some(network_factory),
        token_verifier,
        require_auth: args.require_token_auth,
        topology: None, // TODO: Wire up sharding topology when enabled
        #[cfg(feature = "global-discovery")]
        content_discovery: node_mode.content_discovery(),
        #[cfg(feature = "forge")]
        forge_node,
        #[cfg(feature = "pijul")]
        pijul_store,
    };
    let client_handler = ClientProtocolHandler::new(client_context);

    // Spawn the Router with all protocol handlers
    let router = {
        use aspen::CLIENT_ALPN;
        use aspen::RAFT_ALPN;
        use iroh::protocol::Router;
        use iroh_gossip::ALPN as GOSSIP_ALPN;

        let mut builder = Router::builder(node_mode.iroh_manager().endpoint().clone());

        // Register Raft protocol handler(s) based on mode
        match &node_mode {
            NodeMode::Single(handle) => {
                // Legacy mode: single Raft handler
                let raft_handler = RaftProtocolHandler::new(handle.raft_node.raft().as_ref().clone());
                builder = builder.accept(RAFT_ALPN, raft_handler);
            }
            NodeMode::Sharded(handle) => {
                // Sharded mode: register sharded Raft handler for multi-shard routing
                builder = builder.accept(RAFT_SHARDED_ALPN, handle.sharded_handler.clone());

                // Also register legacy ALPN routing to shard 0 for backward compatibility
                if let Some(shard_0) = handle.primary_shard() {
                    let legacy_handler = RaftProtocolHandler::new(shard_0.raft().as_ref().clone());
                    builder = builder.accept(RAFT_ALPN, legacy_handler);
                    info!("Legacy RAFT_ALPN routing to shard 0 for backward compatibility");
                }
            }
        }

        builder = builder.accept(CLIENT_ALPN, client_handler);

        // Add gossip handler if enabled
        if let Some(gossip) = node_mode.iroh_manager().gossip() {
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
        }

        // Add blobs protocol handler if blob store is enabled
        if let Some(blob_store) = node_mode.blob_store() {
            let blobs_handler = blob_store.protocol_handler();
            builder = builder.accept(iroh_blobs::ALPN, blobs_handler);
            info!("Blobs protocol handler registered");
        }

        // Add docs sync protocol handler if docs sync is enabled
        if let Some(docs_sync) = node_mode.docs_sync() {
            use aspen::docs::DOCS_SYNC_ALPN;
            let docs_handler = docs_sync.protocol_handler();
            builder = builder.accept(DOCS_SYNC_ALPN, docs_handler);
            info!(
                namespace = %docs_sync.namespace_id,
                "Docs sync protocol handler registered"
            );
        }

        // Add log subscriber protocol handler if log broadcast is enabled
        if let Some(log_sender) = node_mode.log_broadcast() {
            use std::sync::atomic::AtomicU64;
            let committed_index = Arc::new(AtomicU64::new(0));
            let log_subscriber_handler = LogSubscriberProtocolHandler::with_sender(
                &config.cookie,
                config.node_id,
                log_sender.clone(),
                committed_index,
            );
            builder = builder.accept(LOG_SUBSCRIBER_ALPN, log_subscriber_handler);
            info!("Log subscriber protocol handler registered (ALPN: aspen-logs)");
        }

        builder.spawn()
    };

    let endpoint_id = node_mode.iroh_manager().endpoint().id();
    info!(
        endpoint_id = %endpoint_id,
        sharding = config.sharding.enabled,
        "Iroh Router spawned - all client API available via Iroh Client RPC (ALPN: aspen-tui)"
    );

    // Start DNS protocol server if enabled
    #[cfg(feature = "dns")]
    if config.dns_server.enabled {
        let dns_client = Arc::new(AspenDnsClient::new());
        let dns_config = config.dns_server.clone();

        // Wire up DNS cache sync from iroh-docs if both docs_sync and blob_store are available
        if let (Some(docs_sync), Some(blob_store)) = (node_mode.docs_sync(), node_mode.blob_store()) {
            match spawn_dns_sync_listener(Arc::clone(&dns_client), docs_sync, Arc::clone(blob_store)).await {
                Ok(_cancel_token) => {
                    info!(
                        namespace = %docs_sync.namespace_id,
                        "DNS cache sync listener started - DNS records will sync from cluster"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Failed to start DNS cache sync listener - DNS will serve static records only");
                }
            }
        } else {
            info!("DNS cache sync disabled - docs_sync or blob_store not available");
        }

        match DnsProtocolServer::new(dns_config.clone(), Arc::clone(&dns_client)) {
            Ok(dns_server) => {
                info!(
                    bind_addr = %dns_config.bind_addr,
                    zones = ?dns_config.zones,
                    forwarding = dns_config.forwarding_enabled,
                    "DNS protocol server starting"
                );

                tokio::spawn(async move {
                    if let Err(e) = dns_server.run().await {
                        error!(error = %e, "DNS protocol server error");
                    }
                });
            }
            Err(e) => {
                warn!(error = %e, "Failed to create DNS protocol server");
            }
        }
    }

    // Generate and print cluster ticket for TUI connection
    let cluster_ticket = {
        use aspen::cluster::ticket::AspenClusterTicket;
        use iroh_gossip::proto::TopicId;

        let hash = blake3::hash(config.cookie.as_bytes());
        let topic_id = TopicId::from_bytes(*hash.as_bytes());

        AspenClusterTicket::with_bootstrap(topic_id, config.cookie.clone(), endpoint_id)
    };

    let ticket_str = cluster_ticket.serialize();
    info!(ticket = %ticket_str, "cluster ticket generated");
    println!();
    println!("Connect with TUI:");
    println!("  aspen-tui --ticket {}", ticket_str);
    println!();

    // Wait for shutdown signal
    shutdown_signal().await;

    // Gracefully shutdown Iroh Router
    info!("shutting down Iroh Router");
    router.shutdown().await?;

    // Gracefully shutdown the node
    node_mode.shutdown().await?;

    Ok(())
}

/// Wait for shutdown signal (SIGINT or SIGTERM).
///
/// Tiger Style: Handles both signals for graceful shutdown in production
/// (systemd sends SIGTERM) and development (Ctrl-C sends SIGINT).
async fn shutdown_signal() {
    let ctrl_c = async {
        match signal::ctrl_c().await {
            Ok(()) => {}
            Err(err) => error!("failed to install Ctrl+C handler: {}", err),
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => error!("failed to install SIGTERM handler: {}", err),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("received SIGINT, initiating graceful shutdown");
        }
        _ = terminate => {
            info!("received SIGTERM, initiating graceful shutdown");
        }
    }
}
