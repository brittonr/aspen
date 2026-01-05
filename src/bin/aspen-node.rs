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

use anyhow::Context;
use anyhow::Result;
// Note: spawn_dns_sync_listener import commented out since it's not available
// #[cfg(feature = "dns")]
// use aspen_dns::spawn_dns_sync_listener;
use aspen::ClientProtocolContext;
use aspen::ClientProtocolHandler;
use aspen::LOG_SUBSCRIBER_ALPN;
use aspen::LogSubscriberProtocolHandler;
use aspen::RAFT_SHARDED_ALPN;
use aspen::RaftProtocolHandler;
use aspen::api::ClusterController;
use aspen::api::DeterministicClusterController;
use aspen::api::DeterministicKeyValueStore;
use aspen::api::KeyValueStore;
use aspen::auth::CapabilityToken;
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
use aspen_core::context::InMemoryWatchRegistry;
use aspen_core::context::WatchRegistry;
use aspen_jobs::JobManager;
use aspen_raft::node::RaftNode;
use clap::Parser;
use tokio::signal;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;

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
            NodeMode::Single(h) => &h.network.iroh_manager,
            NodeMode::Sharded(h) => &h.base.iroh_manager,
        }
    }

    fn blob_store(&self) -> Option<&Arc<aspen::blob::IrohBlobStore>> {
        match self {
            NodeMode::Single(h) => h.network.blob_store.as_ref(),
            NodeMode::Sharded(h) => h.base.blob_store.as_ref(),
        }
    }

    fn docs_sync(&self) -> Option<&aspen::docs::DocsSyncResources> {
        None
    }

    fn log_broadcast(&self) -> Option<&tokio::sync::broadcast::Sender<aspen::raft::log_subscriber::LogEntryPayload>> {
        match self {
            NodeMode::Single(h) => h.sync.log_broadcast.as_ref(),
            NodeMode::Sharded(h) => h.log_broadcast.as_ref(),
        }
    }

    #[cfg(feature = "global-discovery")]
    fn content_discovery(&self) -> Option<aspen::cluster::content_discovery::ContentDiscoveryService> {
        match self {
            NodeMode::Single(h) => h.discovery.content_discovery.clone(),
            NodeMode::Sharded(h) => h.content_discovery.clone(),
        }
    }

    fn topology(&self) -> &Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>> {
        match self {
            NodeMode::Single(_) => &None,
            NodeMode::Sharded(h) => &h.topology,
        }
    }

    async fn shutdown(self) -> Result<()> {
        match self {
            NodeMode::Single(h) => h.shutdown().await,
            NodeMode::Sharded(h) => h.shutdown().await,
        }
    }

    /// Get the database handle from the state machine (if using Redb storage).
    ///
    /// For sharded mode, returns the database from shard 0 (primary shard).
    /// Returns None if using in-memory storage.
    fn db(&self) -> Option<std::sync::Arc<redb::Database>> {
        match self {
            NodeMode::Single(h) => h.storage.state_machine.db(),
            NodeMode::Sharded(h) => {
                // Use shard 0 as the primary shard for maintenance operations
                h.shard_state_machines.get(&0).and_then(|sm| sm.db())
            }
        }
    }
}

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
    /// Options: "inmemory", "redb" (default)
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
    let mut config = NodeConfig::from_env();
    config.node_id = args.node_id.unwrap_or(0);
    config.data_dir = args.data_dir.clone();
    config.storage_backend = args.storage_backend.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default();
    // Prefer redb_log_path if provided, fall back to redb_sm_path for backwards compat
    config.redb_path = args.redb_log_path.clone().or_else(|| args.redb_sm_path.clone());
    config.host = args.host.clone().unwrap_or_else(|| "127.0.0.1".into());
    config.cookie = args.cookie.clone().unwrap_or_else(|| "aspen-cookie".into());
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

#[tokio::main]
async fn main() -> Result<()> {
    let (args, config) = initialize_and_load_config().await?;

    // Display version info with git hash
    let git_hash = env!("GIT_HASH", "GIT_HASH not set");
    let build_time = env!("BUILD_TIME", "BUILD_TIME not set");

    info!(
        node_id = config.node_id,
        control_backend = ?config.control_backend,
        sharding_enabled = config.sharding.enabled,
        git_hash = git_hash,
        build_time = build_time,
        "starting aspen node v{} ({})",
        env!("CARGO_PKG_VERSION"),
        git_hash
    );

    // Bootstrap the node based on sharding configuration
    let node_mode = bootstrap_node_and_generate_token(&args, &config).await?;

    // Extract controller, kv_store, and primary_raft_node from node_mode
    let (controller, kv_store, primary_raft_node, network_factory) = extract_node_components(&config, &node_mode)?;

    // Create shared watch registry for tracking active subscriptions
    let watch_registry: Arc<dyn WatchRegistry> = Arc::new(InMemoryWatchRegistry::new());

    let (_token_verifier, client_context, _worker_service_handle, worker_coordinator) = setup_client_protocol(
        &args,
        &config,
        &node_mode,
        &controller,
        &kv_store,
        &primary_raft_node,
        &network_factory,
        watch_registry.clone(),
    )
    .await?;

    // Note: Job queue initialization happens automatically after cluster init RPC succeeds
    // See handle_init_cluster() in aspen-rpc-handlers/src/handlers/cluster.rs
    let client_handler = ClientProtocolHandler::new(client_context);

    // Spawn the Router with all protocol handlers
    let router = setup_router(&config, &node_mode, client_handler, watch_registry);

    let endpoint_id = node_mode.iroh_manager().endpoint().id();
    info!(
        endpoint_id = %endpoint_id,
        sharding = config.sharding.enabled,
        "Iroh Router spawned - all client API available via Iroh Client RPC (ALPN: aspen-tui)"
    );

    // Start DNS protocol server if enabled
    start_dns_server(&config).await;

    // Generate and print cluster ticket
    print_cluster_ticket(&config, endpoint_id);

    // Wait for shutdown signal
    shutdown_signal().await;

    // Stop distributed worker coordinator if started
    if let Some(ref coordinator) = worker_coordinator {
        info!("stopping distributed worker coordinator");
        if let Err(e) = coordinator.stop().await {
            error!(error = %e, "failed to stop distributed worker coordinator");
        }
    }

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

/// Initialize tracing and load configuration.
async fn initialize_and_load_config() -> Result<(Args, NodeConfig)> {
    init_tracing();

    let args = Args::parse();
    let cli_config = build_cluster_config(&args);

    // Load configuration with proper precedence (env < TOML < CLI)
    // Provide actionable error messages for common misconfigurations
    let config = match load_config(args.config.as_deref(), cli_config) {
        Ok(config) => config,
        Err(e) => {
            handle_config_error(e)?;
            unreachable!()
        }
    };

    Ok((args, config))
}

/// Handle configuration errors with actionable user guidance.
fn handle_config_error(e: anyhow::Error) -> Result<()> {
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
        Err(e)
    }
}

/// Bootstrap the node and generate root token if requested.
async fn bootstrap_node_and_generate_token(args: &Args, config: &NodeConfig) -> Result<NodeMode> {
    if config.sharding.enabled {
        // Sharded mode: multiple Raft instances
        let mut sharded_handle = bootstrap_sharded_node(config.clone()).await?;

        // Generate and output root token if requested
        if let Some(ref token_path) = args.output_root_token {
            generate_and_write_root_token(
                token_path,
                sharded_handle.base.iroh_manager.endpoint().secret_key(),
                &mut |token| sharded_handle.root_token = Some(token),
            )
            .await?;
        }

        // For sharded mode, use the ShardedKeyValueStore as the KV store
        // Use shard 0's RaftNode for ClusterController operations
        info!(
            num_shards = sharded_handle.shard_count(),
            local_shards = ?sharded_handle.local_shard_ids(),
            "sharded node bootstrap complete"
        );

        Ok(NodeMode::Sharded(sharded_handle))
    } else {
        // Non-sharded mode: single Raft instance
        let mut handle = bootstrap_node(config.clone()).await?;

        // Generate and output root token if requested
        if let Some(ref token_path) = args.output_root_token {
            generate_and_write_root_token(
                token_path,
                handle.network.iroh_manager.endpoint().secret_key(),
                &mut |token| handle.root_token = Some(token),
            )
            .await?;
        }

        Ok(NodeMode::Single(handle))
    }
}

/// Generate and write root token to file.
async fn generate_and_write_root_token<F>(
    token_path: &std::path::Path,
    secret_key: &iroh::SecretKey,
    store_token: &mut F,
) -> Result<()>
where
    F: FnMut(CapabilityToken),
{
    let token = aspen::auth::generate_root_token(secret_key, std::time::Duration::from_secs(365 * 24 * 60 * 60))
        .context("failed to generate root token")?;

    let token_base64 = token.to_base64().context("failed to encode root token")?;
    std::fs::write(token_path, &token_base64)
        .with_context(|| format!("failed to write token to {}", token_path.display()))?;

    info!(
        token_path = %token_path.display(),
        issuer = %token.issuer,
        "root token written to file"
    );

    store_token(token);
    Ok(())
}

/// Setup cluster and key-value store controllers based on configuration.
fn setup_controllers(config: &NodeConfig, handle: &NodeHandle) -> (Arc<dyn ClusterController>, Arc<dyn KeyValueStore>) {
    match config.control_backend {
        ControlBackend::Deterministic => {
            (Arc::new(DeterministicClusterController::new()), Arc::new(DeterministicKeyValueStore::new()))
        }
        ControlBackend::Raft => {
            // Use RaftNode directly as both controller and KV store
            let raft_node = handle.storage.raft_node.clone();
            (raft_node.clone(), raft_node)
        }
    }
}

/// Setup token authentication if enabled.
async fn setup_token_authentication(args: &Args, node_mode: &NodeMode) -> Result<Option<TokenVerifier>> {
    if !args.enable_token_auth {
        return Ok(None);
    }

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
            if key_bytes.len() != 32 {
                anyhow::bail!("Invalid key length: expected 32 bytes (64 hex chars), got {}", key_bytes.len());
            }
            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            let public_key = iroh::SecretKey::from_bytes(&key_array).public();
            verifier = verifier.with_trusted_root(public_key);
            info!(
                trusted_root = %public_key,
                "Token auth enabled with explicit trusted root"
            );
        }
    }

    Ok(Some(verifier))
}

/// Components extracted from a node, regardless of mode.
type NodeComponents = (
    Arc<dyn ClusterController>,
    Arc<dyn KeyValueStore>,
    Arc<RaftNode>,
    Arc<aspen::cluster::IrpcRaftNetworkFactory>,
);

/// Extract node components based on mode (single vs sharded).
fn extract_node_components(config: &NodeConfig, node_mode: &NodeMode) -> Result<NodeComponents> {
    match node_mode {
        NodeMode::Single(handle) => {
            let (controller, kv_store) = setup_controllers(config, handle);
            let primary_raft_node = handle.storage.raft_node.clone();
            let network_factory = handle.network.network_factory.clone();
            Ok((controller, kv_store, primary_raft_node, network_factory))
        }
        NodeMode::Sharded(handle) => {
            let kv_store: Arc<dyn KeyValueStore> = handle.sharded_kv.clone();
            let primary_shard = handle
                .primary_shard()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("shard 0 must be present in sharded mode"))?;
            let controller: Arc<dyn ClusterController> = primary_shard.clone();
            let network_factory = handle.base.network_factory.clone();
            Ok((controller, kv_store, primary_shard, network_factory))
        }
    }
}

/// Initialize job manager and optionally start worker service.
async fn initialize_job_system(
    config: &NodeConfig,
    node_mode: &NodeMode,
    kv_store: Arc<dyn KeyValueStore>,
) -> Result<(
    Arc<JobManager<dyn KeyValueStore>>,
    Option<Arc<aspen::cluster::worker_service::WorkerService>>,
    Option<tokio_util::sync::CancellationToken>,
)> {
    use aspen::cluster::worker_service::WorkerService;
    use aspen_jobs::workers::MaintenanceWorker;
    use tokio_util::sync::CancellationToken;

    // Create JobManager
    let job_manager = Arc::new(JobManager::new(kv_store.clone()));

    // Initialize the job manager if the cluster is already initialized
    // For new clusters, this will happen in handle_init_cluster
    // For existing clusters, we need to initialize here
    match job_manager.initialize().await {
        Ok(()) => info!("job manager initialized with priority queues"),
        Err(e) => {
            // Check if it's because the cluster isn't initialized yet
            // In that case, it will be initialized later in handle_init_cluster
            debug!("job manager initialization deferred: {}", e);
        }
    }

    // Start worker service if enabled
    if config.worker.enabled {
        info!(
            worker_count = config.worker.worker_count,
            job_types = ?config.worker.job_types,
            tags = ?config.worker.tags,
            "initializing worker service"
        );

        // Create endpoint provider adapter
        let endpoint_provider =
            Arc::new(aspen::protocol_adapters::EndpointProviderAdapter::new(node_mode.iroh_manager().clone()))
                as Arc<dyn aspen_core::EndpointProvider>;

        // Create worker service with shared job manager
        let mut worker_service = WorkerService::new(
            config.node_id,
            config.worker.clone(),
            job_manager.clone(),
            kv_store.clone(),
            endpoint_provider,
        )
        .context("failed to create worker service")?;

        // Register built-in workers
        // Register maintenance worker for system tasks (only when using Redb storage)
        if let Some(db) = node_mode.db() {
            let maintenance_worker = MaintenanceWorker::new(config.node_id, db.clone());
            worker_service
                .register_handler("compact_storage", maintenance_worker.clone())
                .await
                .context("failed to register maintenance worker")?;
            worker_service
                .register_handler("cleanup_blobs", maintenance_worker.clone())
                .await
                .context("failed to register maintenance worker")?;
            worker_service
                .register_handler("health_check", maintenance_worker)
                .await
                .context("failed to register maintenance worker")?;
            info!("maintenance workers registered (Redb storage backend)");
        } else {
            info!("maintenance workers skipped (in-memory storage backend)");
        }

        // Register VM executor worker for sandboxed job execution
        #[cfg(all(feature = "vm-executor", target_os = "linux"))]
        {
            use aspen_jobs::HyperlightWorker;
            // Try to get blob store from node mode
            if let Some(blob_store) = node_mode.blob_store() {
                let blob_store_dyn: Arc<dyn aspen_blob::BlobStore> = blob_store.clone();
                match HyperlightWorker::new(blob_store_dyn) {
                    Ok(vm_worker) => {
                        worker_service
                            .register_handler("vm_execute", vm_worker)
                            .await
                            .context("failed to register VM executor worker")?;
                        info!("VM executor worker registered (Hyperlight with blob store)");
                    }
                    Err(e) => {
                        warn!("Failed to create HyperlightWorker: {}. VM execution disabled.", e);
                        warn!("VM execution requires KVM support on Linux");
                    }
                }
            } else {
                warn!("No blob store available for VM executor. VM execution disabled.");
                warn!("Enable blob storage in node configuration to use VM executor.");
            }
        }

        // Register echo worker as fallback for unregistered job types
        // This handles arbitrary job types by echoing the payload as the result
        use aspen_jobs::workers::EchoWorker;
        worker_service.register_handler("*", EchoWorker).await.context("failed to register echo worker")?;
        info!("echo worker registered as fallback handler");

        // Start the worker service
        worker_service.start().await.context("failed to start worker service")?;

        let worker_service = Arc::new(worker_service);
        let cancel_token = CancellationToken::new();

        info!(worker_count = config.worker.worker_count, "worker service started");

        Ok((job_manager, Some(worker_service), Some(cancel_token)))
    } else {
        info!("worker service disabled in configuration");
        Ok((job_manager, None, None))
    }
}

/// Setup client protocol context and handler.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
async fn setup_client_protocol(
    args: &Args,
    config: &NodeConfig,
    node_mode: &NodeMode,
    controller: &Arc<dyn ClusterController>,
    kv_store: &Arc<dyn KeyValueStore>,
    primary_raft_node: &Arc<RaftNode>,
    network_factory: &Arc<aspen::cluster::IrpcRaftNetworkFactory>,
    watch_registry: Arc<dyn WatchRegistry>,
) -> Result<(
    Option<Arc<TokenVerifier>>,
    ClientProtocolContext,
    Option<Arc<aspen::cluster::worker_service::WorkerService>>,
    Option<Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
)> {
    // Create token verifier if authentication is enabled
    let token_verifier_arc = setup_token_authentication(args, node_mode).await?.map(Arc::new);

    // Create Client protocol context and handler
    // Since docs_sync returns None, use stub implementation
    let docs_sync_arc: Option<Arc<dyn aspen::api::DocsSyncProvider>> = None;
    let peer_manager_arc: Option<Arc<dyn aspen::api::PeerManager>> = None;

    // Create adapter for endpoint manager
    let endpoint_manager_adapter =
        Arc::new(aspen::protocol_adapters::EndpointProviderAdapter::new(node_mode.iroh_manager().clone()))
            as Arc<dyn aspen::api::EndpointProvider>;

    // Use the real IrpcRaftNetworkFactory which implements aspen_core::NetworkFactory
    let network_factory_arc: Arc<dyn aspen::api::NetworkFactory> = network_factory.clone();

    // Initialize ForgeNode if blob_store is available
    #[cfg(feature = "forge")]
    let _forge_node = node_mode.blob_store().map(|blob_store| {
        let secret_key = node_mode.iroh_manager().secret_key().clone();
        Arc::new(aspen::forge::ForgeNode::new(blob_store.clone(), kv_store.clone(), secret_key))
    });

    // Initialize PijulStore if blob_store and data_dir are available
    #[cfg(feature = "pijul")]
    let _pijul_store = node_mode.blob_store().and_then(|blob_store| {
        config.data_dir.as_ref().map(|data_dir| {
            Arc::new(aspen::pijul::PijulStore::new(blob_store.clone(), kv_store.clone(), data_dir.clone()))
        })
    });

    // Initialize JobManager and start worker service if enabled
    let (job_manager, worker_service_handle, _worker_service_cancel) =
        initialize_job_system(config, node_mode, kv_store.clone()).await?;

    // Create distributed worker coordinator for external worker registration
    let worker_coordinator = Arc::new(aspen_coordination::DistributedWorkerCoordinator::new(kv_store.clone()));

    // Start coordinator background tasks if distributed mode is enabled
    // Return the coordinator handle for shutdown if started
    let coordinator_for_shutdown = if config.worker.enable_distributed {
        Arc::clone(&worker_coordinator)
            .start()
            .await
            .context("failed to start distributed worker coordinator")?;
        info!("distributed worker coordinator started with background tasks");
        Some(Arc::clone(&worker_coordinator))
    } else {
        None
    };

    let client_context = ClientProtocolContext {
        node_id: config.node_id,
        controller: controller.clone(),
        kv_store: kv_store.clone(),
        #[cfg(feature = "sql")]
        sql_executor: primary_raft_node.clone(),
        state_machine: Some(primary_raft_node.state_machine().clone()),
        endpoint_manager: endpoint_manager_adapter,
        blob_store: node_mode.blob_store().cloned(),
        peer_manager: peer_manager_arc,
        docs_sync: docs_sync_arc,
        cluster_cookie: config.cookie.clone(),
        start_time: std::time::Instant::now(),
        network_factory: Some(network_factory_arc),
        token_verifier: token_verifier_arc.clone(),
        require_auth: args.require_token_auth,
        topology: node_mode.topology().clone(),
        #[cfg(feature = "global-discovery")]
        content_discovery: node_mode.content_discovery().map(|service| {
            Arc::new(aspen::protocol_adapters::ContentDiscoveryAdapter::new(Arc::new(service)))
                as Arc<dyn aspen_core::ContentDiscovery>
        }),
        job_manager: Some(job_manager),
        worker_service: worker_service_handle.clone(),
        worker_coordinator: Some(worker_coordinator),
        watch_registry: Some(watch_registry),
    };

    Ok((token_verifier_arc, client_context, worker_service_handle, coordinator_for_shutdown))
}

/// Setup the Iroh Router with all protocol handlers.
fn setup_router(
    config: &NodeConfig,
    node_mode: &NodeMode,
    client_handler: ClientProtocolHandler,
    watch_registry: Arc<dyn WatchRegistry>,
) -> iroh::protocol::Router {
    use aspen::CLIENT_ALPN;
    use aspen::RAFT_ALPN;
    use iroh::protocol::Router;
    use iroh_gossip::ALPN as GOSSIP_ALPN;

    let mut builder = Router::builder(node_mode.iroh_manager().endpoint().clone());

    // Register Raft protocol handler(s) based on mode
    match &node_mode {
        NodeMode::Single(handle) => {
            // Legacy mode: single Raft handler
            // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are identical
            // but Rust treats them as different types. We transmute to convert between them.
            let transport_raft: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
                unsafe { std::mem::transmute(handle.storage.raft_node.raft().as_ref().clone()) };
            let raft_handler = RaftProtocolHandler::new(transport_raft);
            builder = builder.accept(RAFT_ALPN, raft_handler);
        }
        NodeMode::Sharded(handle) => {
            // Sharded mode: register sharded Raft handler for multi-shard routing
            builder = builder.accept(RAFT_SHARDED_ALPN, handle.sharded_handler.clone());

            // Also register legacy ALPN routing to shard 0 for backward compatibility
            if let Some(shard_0) = handle.primary_shard() {
                // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are identical
                // but Rust treats them as different types. We transmute to convert between them.
                let transport_raft: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
                    unsafe { std::mem::transmute(shard_0.raft().as_ref().clone()) };
                let legacy_handler = RaftProtocolHandler::new(transport_raft);
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
        // aspen_raft::log_subscriber::LogEntryPayload is now a re-export from
        // aspen_transport::log_subscriber::LogEntryPayload, so they're the same type
        let log_subscriber_handler = LogSubscriberProtocolHandler::with_sender(
            &config.cookie,
            config.node_id,
            log_sender.clone(),
            committed_index,
        )
        .with_watch_registry(watch_registry);
        builder = builder.accept(LOG_SUBSCRIBER_ALPN, log_subscriber_handler);
        info!("Log subscriber protocol handler registered (ALPN: aspen-logs)");
    }

    builder.spawn()
}

/// Start the DNS protocol server if enabled.
async fn start_dns_server(config: &NodeConfig) {
    #[cfg(feature = "dns")]
    if config.dns_server.enabled {
        let dns_client = Arc::new(AspenDnsClient::new());
        let dns_config = config.dns_server.clone();

        // DNS cache sync disabled - docs_sync feature not yet implemented
        info!("DNS cache sync disabled - docs_sync feature not yet implemented");

        // Convert cluster DnsServerConfig to dns crate DnsServerConfig
        let dns_server_config = aspen::dns::DnsServerConfig {
            enabled: dns_config.enabled,
            bind_addr: dns_config.bind_addr,
            zones: dns_config.zones.clone(),
            upstreams: dns_config.upstreams.clone(),
            forwarding_enabled: dns_config.forwarding_enabled,
        };
        match DnsProtocolServer::new(dns_server_config, Arc::clone(&dns_client)).await {
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
}

/// Generate and print cluster ticket for TUI connection.
fn print_cluster_ticket(config: &NodeConfig, endpoint_id: iroh::PublicKey) {
    use aspen::cluster::ticket::AspenClusterTicket;
    use iroh_gossip::proto::TopicId;

    let hash = blake3::hash(config.cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    let cluster_ticket = AspenClusterTicket::with_bootstrap(topic_id, config.cookie.clone(), endpoint_id);

    let ticket_str = cluster_ticket.serialize();
    info!(ticket = %ticket_str, "cluster ticket generated");
    println!();
    println!("Connect with TUI:");
    println!("  aspen-tui --ticket {}", ticket_str);
    println!();
}
