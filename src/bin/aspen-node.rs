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
use aspen::cluster::bootstrap::initialize_blob_replication;
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
#[cfg(feature = "secrets")]
use aspen_rpc_handlers::handlers::SecretsService;
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
    Single(Box<NodeHandle>),
    /// Sharded node with multiple Raft instances.
    Sharded(Box<ShardedNodeHandle>),
}

impl NodeMode {
    fn iroh_manager(&self) -> &Arc<aspen::cluster::IrohEndpointManager> {
        match self {
            NodeMode::Single(h) => &h.network.iroh_manager,
            NodeMode::Sharded(h) => &h.base.network.iroh_manager,
        }
    }

    fn blob_store(&self) -> Option<&Arc<aspen::blob::IrohBlobStore>> {
        match self {
            NodeMode::Single(h) => h.network.blob_store.as_ref(),
            NodeMode::Sharded(h) => h.base.network.blob_store.as_ref(),
        }
    }

    fn docs_sync(&self) -> Option<&Arc<aspen::docs::DocsSyncResources>> {
        match self {
            NodeMode::Single(h) => h.sync.docs_sync.as_ref(),
            NodeMode::Sharded(h) => h.sync.docs_sync.as_ref(),
        }
    }

    fn peer_manager(&self) -> Option<&Arc<aspen::docs::PeerManager>> {
        match self {
            NodeMode::Single(h) => h.sync.peer_manager.as_ref(),
            NodeMode::Sharded(h) => h.sync.peer_manager.as_ref(),
        }
    }

    fn log_broadcast(&self) -> Option<&tokio::sync::broadcast::Sender<aspen::raft::log_subscriber::LogEntryPayload>> {
        match self {
            NodeMode::Single(h) => h.sync.log_broadcast.as_ref(),
            NodeMode::Sharded(h) => h.sync.log_broadcast.as_ref(),
        }
    }

    #[cfg(feature = "global-discovery")]
    fn content_discovery(&self) -> Option<aspen::cluster::content_discovery::ContentDiscoveryService> {
        match self {
            NodeMode::Single(h) => h.discovery.content_discovery.clone(),
            NodeMode::Sharded(h) => h.discovery.content_discovery.clone(),
        }
    }

    fn topology(&self) -> &Option<Arc<tokio::sync::RwLock<aspen_sharding::ShardTopology>>> {
        match self {
            NodeMode::Single(_) => &None,
            NodeMode::Sharded(h) => &h.sharding.topology,
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
                h.sharding.shard_state_machines.get(&0).and_then(|sm| sm.db())
            }
        }
    }

    /// Get the hook service for event-driven automation (if enabled).
    fn hook_service(&self) -> Option<Arc<aspen_hooks::HookService>> {
        match self {
            NodeMode::Single(h) => h.hooks.hook_service.clone(),
            NodeMode::Sharded(h) => h.hooks.hook_service.clone(),
        }
    }

    /// Get the hooks configuration.
    fn hooks_config(&self) -> aspen_hooks::HooksConfig {
        match self {
            NodeMode::Single(h) => h.config.hooks.clone(),
            NodeMode::Sharded(h) => h.base.config.hooks.clone(),
        }
    }

    /// Get the shutdown token.
    fn shutdown_token(&self) -> tokio_util::sync::CancellationToken {
        match self {
            NodeMode::Single(h) => h.shutdown.shutdown_token.clone(),
            NodeMode::Sharded(h) => h.base.shutdown_token.clone(),
        }
    }

    /// Get mutable reference to blob replication resources (non-sharded mode only).
    fn blob_replication_mut(&mut self) -> Option<&mut aspen::cluster::bootstrap::BlobReplicationResources> {
        match self {
            NodeMode::Single(h) => Some(&mut h.blob_replication),
            NodeMode::Sharded(_) => None, // Blob replication not supported in sharded mode
        }
    }

    /// Get the blob replication manager (non-sharded mode only).
    fn blob_replication_manager(&self) -> Option<aspen_blob::BlobReplicationManager> {
        match self {
            NodeMode::Single(h) => h.blob_replication.replication_manager.clone(),
            NodeMode::Sharded(_) => None, // Blob replication not supported in sharded mode
        }
    }

    /// Get the node configuration.
    fn config(&self) -> &aspen::cluster::config::NodeConfig {
        match self {
            NodeMode::Single(h) => &h.config,
            NodeMode::Sharded(h) => &h.base.config,
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

    /// Port to bind for QUIC connections.
    /// - 0: Use random port (default)
    /// - Other: Use specific port (e.g., 7777 for VM deployments)
    #[arg(long)]
    bind_port: Option<u16>,

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

    /// Path to SOPS-encrypted secrets file.
    /// Contains trusted roots, signing key, and pre-built capability tokens.
    /// Format: TOML encrypted with age via SOPS.
    #[cfg(feature = "secrets")]
    #[arg(long)]
    secrets_file: Option<PathBuf>,

    /// Path to age identity file for decrypting SOPS secrets.
    /// Defaults to $XDG_CONFIG_HOME/sops/age/keys.txt if not specified.
    #[cfg(feature = "secrets")]
    #[arg(long)]
    age_identity_file: Option<PathBuf>,

    // === Worker Configuration ===
    /// Enable job workers on this node.
    ///
    /// When enabled, the node starts a worker pool to process jobs
    /// from the distributed queue. Workers execute CI pipelines,
    /// maintenance tasks, and other scheduled work.
    #[arg(long)]
    enable_workers: bool,

    /// Number of workers to start (default: CPU count, max: 64).
    ///
    /// Each worker can process one job at a time. More workers allow
    /// parallel job execution but consume more resources.
    #[arg(long)]
    worker_count: Option<usize>,

    /// Job types this worker handles (empty = all).
    ///
    /// When specified, this worker only accepts jobs matching these types.
    /// Examples: "ci_build", "maintenance", "nix_build"
    #[arg(long)]
    worker_job_types: Vec<String>,

    // === CI/CD Configuration ===
    /// Enable CI/CD pipeline orchestration.
    ///
    /// When enabled, the node can receive pipeline trigger requests and
    /// orchestrate pipeline execution using the job system. Requires
    /// workers to be enabled for actual job execution.
    #[arg(long)]
    enable_ci: bool,

    /// Enable automatic CI triggering on ref updates.
    ///
    /// When enabled alongside --enable-ci, the node watches for forge
    /// gossip events and automatically triggers CI for repositories with
    /// `.aspen/ci.ncl` configurations.
    #[arg(long)]
    ci_auto_trigger: bool,
}

/// Initialize tracing subscriber with environment-based filtering.
///
/// Tiger Style: Focused initialization function.
fn init_tracing() {
    // Suppress noisy warnings from network-related crates:
    // - netlink_packet_route: kernel has newer NLA attributes than crate expects
    // - quinn_udp: IPv6 unreachable errors when IPv6 is not available
    const NOISY_CRATES: &str = ",netlink_packet_route=error,quinn_udp=error,netlink_packet_core=error";

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(format!("info{NOISY_CRATES}")));
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
        bind_port: args.bind_port.unwrap_or(0),
    };
    config.peers = args.peers.clone();

    // Apply worker configuration from CLI flags
    if args.enable_workers {
        config.worker.enabled = true;
    }
    if let Some(count) = args.worker_count {
        // Tiger Style: Cap at 64 workers per node
        config.worker.worker_count = count.min(64);
    }
    if !args.worker_job_types.is_empty() {
        config.worker.job_types = args.worker_job_types.clone();
    }

    // Apply CI configuration from CLI flags
    if args.enable_ci {
        config.ci.enabled = true;
    }
    if args.ci_auto_trigger {
        config.ci.auto_trigger = true;
        // CI auto-trigger requires forge gossip to receive ref update announcements
        config.forge.enable_gossip = true;
    }

    // Apply security defaults (e.g., auto-enable raft_auth when pkarr is on)
    config.apply_security_defaults();

    config
}

/// Main entry point for aspen-node.
///
/// Uses a custom tokio runtime with increased worker thread stack size (16 MiB)
/// to handle deep async call chains in Raft replication and CI pipeline execution.
/// The default 2 MiB stack is insufficient for debug builds with complex async
/// state machines spanning multiple subsystems (raft -> kv -> workflow -> jobs).
fn main() -> Result<()> {
    // Build runtime with larger stack size for worker threads.
    // Default is 2 MiB, but complex async chains in debug builds need more.
    // The CI pipeline execution path goes through multiple async layers:
    // RPC handler -> workflow manager -> job manager -> Nickel eval -> Raft
    // Each layer adds significant stack frames, especially in debug builds.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(16 * 1024 * 1024) // 16 MiB stack per worker
        .build()
        .context("failed to build tokio runtime")?;

    runtime.block_on(async_main())
}

async fn async_main() -> Result<()> {
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
    let mut node_mode = bootstrap_node_and_generate_token(&args, &config).await?;

    // Extract controller, kv_store, and primary_raft_node from node_mode
    let (controller, kv_store, primary_raft_node, network_factory) = extract_node_components(&config, &node_mode)?;

    // Initialize blob replication if enabled (non-sharded mode only)
    // This sets up event-driven replication to maintain blob copies across cluster nodes
    if node_mode.blob_replication_mut().is_some() {
        // Extract values before mutable borrow
        let blob_store = node_mode.blob_store().cloned();
        let endpoint = Some(node_mode.iroh_manager().endpoint().clone());
        let shutdown_token = node_mode.shutdown_token();
        let node_config = node_mode.config().clone();

        // Get blob events from broadcaster if available
        let blob_events = blob_store.as_ref().and_then(|bs| bs.broadcaster()).map(|b| b.subscribe());

        let replication_resources = initialize_blob_replication(
            &node_config,
            blob_store,
            endpoint,
            primary_raft_node.clone(), // Use concrete RaftNode, not trait object
            blob_events,
            shutdown_token,
        )
        .await;

        // Wire up topology watcher to track Raft membership changes
        let has_manager = replication_resources.replication_manager.is_some();
        if let Some(blob_replication) = node_mode.blob_replication_mut() {
            *blob_replication = replication_resources;

            if has_manager {
                let metrics_rx = primary_raft_node.raft().metrics();
                let extractor: aspen_blob::replication::topology_watcher::NodeInfoExtractor<_> =
                    Box::new(|metrics: &openraft::RaftMetrics<aspen_raft::types::AppTypeConfig>| {
                        metrics
                            .membership_config
                            .membership()
                            .nodes()
                            .map(|(node_id, node)| {
                                aspen_blob::replication::NodeInfo::new(u64::from(*node_id), node.iroh_addr.id)
                            })
                            .collect()
                    });
                blob_replication.wire_topology_watcher(metrics_rx, extractor);
            }
        }
    }

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
    let router = setup_router(&config, &node_mode, client_handler, watch_registry, kv_store.clone());

    // Get fresh endpoint address (may have discovered more addresses since startup)
    let endpoint_addr = node_mode.iroh_manager().endpoint().addr();
    info!(
        endpoint_id = %endpoint_addr.id,
        sharding = config.sharding.enabled,
        "Iroh Router spawned - all client API available via Iroh Client RPC (ALPN: aspen-tui)"
    );

    // Start DNS protocol server if enabled
    start_dns_server(&config).await;

    // Generate and print cluster ticket (V2 with direct addresses)
    print_cluster_ticket(&config, &endpoint_addr);

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
                sharded_handle.base.network.iroh_manager.endpoint().secret_key(),
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

        Ok(NodeMode::Sharded(Box::new(sharded_handle)))
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

        Ok(NodeMode::Single(Box::new(handle)))
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

/// Load secrets from SOPS-encrypted file if configured.
///
/// Returns the SecretsManager which provides trusted roots and pre-built tokens.
#[cfg(feature = "secrets")]
async fn load_secrets(config: &NodeConfig, args: &Args) -> Result<Option<Arc<aspen_secrets::SecretsManager>>> {
    use aspen_secrets::SecretsManager;
    use aspen_secrets::decrypt_secrets_file;
    use aspen_secrets::load_age_identity;

    if !config.secrets.enabled {
        return Ok(None);
    }

    // Determine secrets file path from args or config
    let secrets_file = args
        .secrets_file
        .as_ref()
        .or(config.secrets.secrets_file.as_ref())
        .ok_or_else(|| anyhow::anyhow!("secrets enabled but no secrets_file specified"))?;

    // Determine identity file path from args or config
    let identity_file = args.age_identity_file.as_ref().or(config.secrets.age_identity_file.as_ref());

    info!(
        secrets_file = %secrets_file.display(),
        identity_file = ?identity_file.map(|p| p.display().to_string()),
        "loading SOPS-encrypted secrets"
    );

    // Load age identity
    let identity = load_age_identity(identity_file.map(|p| p.as_path()), &config.secrets.age_identity_env)
        .await
        .context("failed to load age identity for secrets decryption")?;

    // Decrypt secrets file
    let secrets_file_data =
        decrypt_secrets_file(secrets_file, &identity).await.context("failed to decrypt secrets file")?;

    // Create secrets manager
    let manager = SecretsManager::new(config.secrets.clone(), secrets_file_data)
        .context("failed to initialize secrets manager")?;

    info!(trusted_roots = manager.trusted_root_count(), "secrets loaded successfully");

    Ok(Some(Arc::new(manager)))
}

/// Load or generate cluster identity for federation.
///
/// Loads the cluster secret key from:
/// 1. `cluster_key` config field (hex-encoded 32 bytes)
/// 2. `cluster_key_path` file (hex-encoded 32 bytes)
/// 3. Generated and stored in `data_dir/federation/cluster_key`
#[cfg(feature = "forge")]
fn load_federation_identity(config: &NodeConfig) -> Result<aspen_cluster::federation::ClusterIdentity> {
    use aspen_cluster::federation::ClusterIdentity;

    // Try loading from inline cluster_key config
    if let Some(ref hex_key) = config.federation.cluster_key {
        return ClusterIdentity::from_hex_key(hex_key, config.federation.cluster_name.clone())
            .map_err(|e| anyhow::anyhow!("invalid federation cluster_key: {}", e));
    }

    // Try loading from cluster_key_path file
    if let Some(ref key_path) = config.federation.cluster_key_path {
        let hex_key = std::fs::read_to_string(key_path)
            .with_context(|| format!("failed to read federation key from {}", key_path.display()))?;
        let hex_key = hex_key.trim();
        return ClusterIdentity::from_hex_key(hex_key, config.federation.cluster_name.clone())
            .map_err(|e| anyhow::anyhow!("invalid federation cluster_key in {}: {}", key_path.display(), e));
    }

    // Generate new key and store in data_dir/federation/cluster_key
    let data_dir = config
        .data_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("federation enabled but no data_dir configured for key storage"))?;

    let federation_dir = data_dir.join("federation");
    let key_path = federation_dir.join("cluster_key");

    // Check if key already exists
    if key_path.exists() {
        let hex_key = std::fs::read_to_string(&key_path)
            .with_context(|| format!("failed to read federation key from {}", key_path.display()))?;
        let hex_key = hex_key.trim();
        return ClusterIdentity::from_hex_key(hex_key, config.federation.cluster_name.clone())
            .map_err(|e| anyhow::anyhow!("invalid stored federation cluster_key: {}", e));
    }

    // Generate new identity and persist the key
    info!("generating new federation cluster identity");
    let identity = ClusterIdentity::generate(config.federation.cluster_name.clone());

    // Create federation directory if needed
    std::fs::create_dir_all(&federation_dir)
        .with_context(|| format!("failed to create federation directory: {}", federation_dir.display()))?;

    // Write secret key as hex
    let hex_key = hex::encode(identity.secret_key().to_bytes());
    std::fs::write(&key_path, &hex_key)
        .with_context(|| format!("failed to write federation key to {}", key_path.display()))?;

    info!(
        key_path = %key_path.display(),
        public_key = %identity.public_key(),
        "generated and stored new federation cluster key"
    );

    Ok(identity)
}

/// Parse trusted cluster public keys from config strings.
#[cfg(feature = "forge")]
fn parse_trusted_cluster_keys(keys: &[String]) -> Result<Vec<iroh::PublicKey>> {
    let mut parsed = Vec::with_capacity(keys.len());
    for key_str in keys {
        let key: iroh::PublicKey =
            key_str.parse().with_context(|| format!("invalid trusted cluster public key: {}", key_str))?;
        parsed.push(key);
    }
    Ok(parsed)
}

/// Load Nix cache signer from Transit secrets engine if configured.
///
/// Returns the signer which enables automatic narinfo signing for the Nix binary cache gateway.
/// Uses Ed25519 keys stored in the Transit secrets engine for secure remote signing.
/// Also stores the public key in the Raft KV store for cluster-wide distribution.
#[cfg(all(feature = "secrets", feature = "nix-cache-gateway"))]
async fn load_nix_cache_signer(
    config: &NodeConfig,
    secrets_manager: Option<&Arc<aspen_secrets::SecretsManager>>,
    kv_store: &Arc<dyn aspen_core::KeyValueStore>,
) -> Result<Option<Arc<dyn aspen_nix_cache_gateway::NarinfoSigningProvider>>> {
    use aspen_nix_cache_gateway::NarinfoSigner;

    // Only load signer if Nix cache is enabled and signing is configured
    if !config.nix_cache.enabled {
        return Ok(None);
    }

    let (cache_name, signing_key_name) =
        match (config.nix_cache.cache_name.as_ref(), config.nix_cache.signing_key_name.as_ref()) {
            (Some(cache_name), Some(signing_key_name)) => (cache_name, signing_key_name),
            _ => {
                debug!("Nix cache signing not configured (missing cache_name or signing_key_name)");
                return Ok(None);
            }
        };

    let secrets_manager = match secrets_manager {
        Some(manager) => manager,
        None => {
            warn!("Nix cache signing configured but secrets not available");
            return Ok(None);
        }
    };

    // Get the Transit store
    let transit_store = secrets_manager.get_transit_store(&config.nix_cache.transit_mount).await?;

    // Create Transit-backed signer
    match NarinfoSigner::from_transit(cache_name.clone(), transit_store, signing_key_name.clone()).await {
        Ok(signer) => {
            let signer_arc = Arc::new(signer) as Arc<dyn aspen_nix_cache_gateway::NarinfoSigningProvider>;

            // Store public key in Raft KV for cluster-wide distribution
            match signer_arc.public_key().await {
                Ok(public_key_base64) => {
                    // Format: {cache_name}:{base64_public_key} (Nix-compatible)
                    let nix_public_key_format = format!("{}:{}", cache_name, public_key_base64);

                    // Store at _system:nix-cache:public-key
                    let write_request = aspen_core::WriteRequest {
                        command: aspen_core::WriteCommand::Set {
                            key: "_system:nix-cache:public-key".to_string(),
                            value: nix_public_key_format,
                        },
                    };
                    if let Err(e) = kv_store.write(write_request).await {
                        warn!(
                            error = %e,
                            cache_name = %cache_name,
                            "Failed to store Nix cache public key in KV store, signing will still work locally"
                        );
                    } else {
                        info!(
                            cache_name = %cache_name,
                            "Nix cache public key distributed to cluster via Raft KV"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        cache_name = %cache_name,
                        "Failed to get public key from signer, cannot distribute to cluster"
                    );
                }
            }

            info!(
                cache_name = %cache_name,
                signing_key_name = %signing_key_name,
                transit_mount = %config.nix_cache.transit_mount,
                "Nix cache signer loaded from Transit secrets engine"
            );
            Ok(Some(signer_arc))
        }
        Err(e) => {
            warn!(
                cache_name = %cache_name,
                signing_key_name = %signing_key_name,
                transit_mount = %config.nix_cache.transit_mount,
                error = %e,
                "Failed to load Nix cache signer from Transit, continuing without signing"
            );
            Ok(None)
        }
    }
}

/// Read Nix cache public key from Raft KV store.
///
/// Returns the public key string in Nix format: "{cache_name}:{base64_key}"
/// Returns `None` if the key is not stored in KV (cache signing not configured).
///
/// This is used by `NixBuildWorker` to set `--trusted-public-keys` for cache substitution.
#[cfg(feature = "ci")]
async fn read_nix_cache_public_key(kv_store: &Arc<dyn aspen_core::KeyValueStore>) -> Option<String> {
    use aspen_core::ReadRequest;

    let read_request = ReadRequest::new("_system:nix-cache:public-key");
    match kv_store.read(read_request).await {
        Ok(read_result) => {
            if let Some(kv) = read_result.kv {
                debug!(
                    public_key = %kv.value,
                    "Retrieved Nix cache public key from KV store for CI substituter"
                );
                Some(kv.value)
            } else {
                debug!("No Nix cache public key found in KV store");
                None
            }
        }
        Err(e) => {
            // This is expected when cluster is not yet initialized or key not set
            debug!(error = %e, "Failed to read Nix cache public key from KV store");
            None
        }
    }
}

/// Setup token authentication if enabled.
///
/// If secrets feature is enabled and configured, uses trusted roots from SOPS secrets.
/// Otherwise falls back to CLI args or node's own key.
#[cfg(feature = "secrets")]
async fn setup_token_authentication(
    args: &Args,
    node_mode: &NodeMode,
    secrets_manager: Option<&Arc<aspen_secrets::SecretsManager>>,
) -> Result<Option<TokenVerifier>> {
    if !args.enable_token_auth {
        return Ok(None);
    }

    // If we have a secrets manager, use its pre-built verifier
    if let Some(manager) = secrets_manager {
        let verifier = manager.build_token_verifier();
        info!(
            trusted_roots = manager.trusted_root_count(),
            "Token auth enabled with trusted roots from SOPS secrets"
        );
        return Ok(Some(verifier));
    }

    // Fall back to CLI args or node's own key
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

/// Setup token authentication if enabled (non-secrets version).
#[cfg(not(feature = "secrets"))]
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
            let kv_store: Arc<dyn KeyValueStore> = handle.sharding.sharded_kv.clone();
            let primary_shard = handle
                .primary_shard()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("shard 0 must be present in sharded mode"))?;
            let controller: Arc<dyn ClusterController> = primary_shard.clone();
            let network_factory = handle.base.network.network_factory.clone();
            Ok((controller, kv_store, primary_shard, network_factory))
        }
    }
}

/// Initialize job manager and optionally start worker service.
#[allow(unused_variables)] // token_verifier only used with shell-worker feature
async fn initialize_job_system(
    config: &NodeConfig,
    node_mode: &NodeMode,
    kv_store: Arc<dyn KeyValueStore>,
    token_verifier: Option<Arc<TokenVerifier>>,
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

        // Register Nix build worker for CI/CD Nix flake builds
        #[cfg(feature = "ci")]
        {
            use aspen_cache::CacheIndex;
            use aspen_cache::KvCacheIndex;
            use aspen_ci::NixBuildWorker;
            use aspen_ci::NixBuildWorkerConfig;

            // Create cache index backed by the KV store
            let cache_index: Option<Arc<dyn CacheIndex>> =
                Some(Arc::new(KvCacheIndex::new(kv_store.clone())) as Arc<dyn CacheIndex>);

            // Create SNIX services when enabled in config
            #[cfg(feature = "snix")]
            let (snix_blob_service, snix_directory_service, snix_pathinfo_service) = if config.snix.enabled {
                use aspen_snix::IrohBlobService;
                use aspen_snix::RaftDirectoryService;
                use aspen_snix::RaftPathInfoService;

                let blob_svc: Option<Arc<dyn snix_castore::blobservice::BlobService>> =
                    node_mode.blob_store().map(|bs| {
                        Arc::new(IrohBlobService::from_arc(bs.clone()))
                            as Arc<dyn snix_castore::blobservice::BlobService>
                    });
                let dir_svc: Option<Arc<dyn snix_castore::directoryservice::DirectoryService>> =
                    Some(Arc::new(RaftDirectoryService::from_arc(kv_store.clone()))
                        as Arc<dyn snix_castore::directoryservice::DirectoryService>);
                let pathinfo_svc: Option<Arc<dyn snix_store::pathinfoservice::PathInfoService>> =
                    Some(Arc::new(RaftPathInfoService::from_arc(kv_store.clone()))
                        as Arc<dyn snix_store::pathinfoservice::PathInfoService>);

                info!(
                    directory_prefix = %config.snix.directory_prefix,
                    pathinfo_prefix = %config.snix.pathinfo_prefix,
                    "SNIX services enabled for decomposed content-addressed storage"
                );
                (blob_svc, dir_svc, pathinfo_svc)
            } else {
                (None, None, None)
            };

            #[cfg(not(feature = "snix"))]
            let (snix_blob_service, snix_directory_service, snix_pathinfo_service) = (None, None, None);

            // Determine cache substituter configuration:
            // Enable CI cache substitution when:
            // 1. nix_cache.enabled (gateway is running on this node)
            // 2. nix_cache.enable_ci_substituter (operator hasn't disabled it)
            let use_cluster_cache = config.nix_cache.enabled && config.nix_cache.enable_ci_substituter;

            // Get iroh_endpoint from node_mode's IrohEndpointManager
            let iroh_endpoint = if use_cluster_cache {
                Some(Arc::new(node_mode.iroh_manager().endpoint().clone()))
            } else {
                None
            };

            // Gateway node is this node's own endpoint ID when nix_cache.enabled
            // (the cache gateway runs locally on the same node via self-connect)
            let gateway_node = if use_cluster_cache {
                Some(node_mode.iroh_manager().endpoint().id())
            } else {
                None
            };

            // Read public key from KV store (may be None if not yet stored)
            let cache_public_key = if use_cluster_cache {
                read_nix_cache_public_key(&kv_store).await
            } else {
                None
            };

            // Log cache substituter status
            if use_cluster_cache {
                if cache_public_key.is_some() {
                    let gateway_short = gateway_node
                        .as_ref()
                        .map(|g| g.fmt_short().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    info!(
                        gateway_node = %gateway_short,
                        "Nix cache substituter enabled for CI workers"
                    );
                } else {
                    warn!(
                        "Nix cache substituter requested but public key not available in KV store. \
                         Cache substitution will be disabled until key is stored at _system:nix-cache:public-key"
                    );
                }
            }

            let nix_config = NixBuildWorkerConfig {
                node_id: config.node_id,
                cluster_id: config.cookie.clone(),
                blob_store: node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>),
                cache_index,
                snix_blob_service,
                snix_directory_service,
                snix_pathinfo_service,
                output_dir: std::path::PathBuf::from("/tmp/aspen-ci/builds"),
                nix_binary: "nix".to_string(),
                verbose: false,
                use_cluster_cache,
                iroh_endpoint,
                gateway_node,
                cache_public_key,
            };

            // Validate worker config and log warnings for missing services
            let all_services_available = nix_config.validate();
            if !all_services_available {
                warn!(
                    node_id = config.node_id,
                    "NixBuildWorker will operate with reduced functionality due to missing services"
                );
            }

            let nix_worker = NixBuildWorker::new(nix_config);
            worker_service
                .register_handler("ci_nix_build", nix_worker)
                .await
                .context("failed to register Nix build worker")?;
            info!(
                cluster_id = %config.cookie,
                node_id = config.node_id,
                snix_enabled = config.snix.enabled,
                "Nix build worker registered for CI/CD flake builds"
            );

            // Register CI worker for job execution (Linux only)
            // Two modes available:
            // 1. LocalExecutorWorker (ASPEN_CI_LOCAL_EXECUTOR=1): Direct process execution, suitable when
            //    already running in an isolated environment (e.g., a VM)
            // 2. CloudHypervisorWorker (ASPEN_CI_KERNEL_PATH set): Full VM isolation via nested Cloud
            //    Hypervisor microVMs
            #[cfg(target_os = "linux")]
            {
                let use_local_executor = std::env::var("ASPEN_CI_LOCAL_EXECUTOR")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);

                if use_local_executor {
                    // Use LocalExecutorWorker for direct process execution
                    use aspen_ci::LocalExecutorWorker;
                    use aspen_ci::LocalExecutorWorkerConfig;

                    let workspace_dir = std::env::var("ASPEN_CI_WORKSPACE_DIR")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| std::path::PathBuf::from("/workspace"));

                    let local_config = LocalExecutorWorkerConfig {
                        workspace_dir: workspace_dir.clone(),
                        cleanup_workspaces: true,
                    };

                    let blob_store_opt = node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>);

                    let local_worker = if let Some(blob_store) = blob_store_opt {
                        LocalExecutorWorker::with_blob_store(local_config, blob_store)
                    } else {
                        LocalExecutorWorker::new(local_config)
                    };

                    worker_service
                        .register_handler("ci_vm", local_worker)
                        .await
                        .context("failed to register Local Executor worker")?;
                    info!(
                        workspace_dir = %workspace_dir.display(),
                        "Local Executor worker registered for CI jobs (no VM isolation)"
                    );
                } else {
                    // Use CloudHypervisorWorker for VM-isolated CI jobs
                    use aspen_ci::CloudHypervisorWorker;
                    use aspen_ci::CloudHypervisorWorkerConfig;

                    // Build Cloud Hypervisor worker config
                    // Paths are expected to be provided by the Nix-built CI environment
                    let ch_state_dir = config
                        .data_dir
                        .as_ref()
                        .map(|d| d.join("ci").join("vms"))
                        .unwrap_or_else(|| std::path::PathBuf::from("/var/lib/aspen/ci/vms"));

                    let ch_config = CloudHypervisorWorkerConfig {
                        node_id: config.node_id,
                        state_dir: ch_state_dir.clone(),
                        // kernel/initrd/toplevel paths are discovered from environment or config
                        kernel_path: std::env::var("ASPEN_CI_KERNEL_PATH")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_default(),
                        initrd_path: std::env::var("ASPEN_CI_INITRD_PATH")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_default(),
                        // toplevel is the NixOS system with init script (kernel cmdline needs ${toplevel}/init)
                        toplevel_path: std::env::var("ASPEN_CI_TOPLEVEL_PATH")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_default(),
                        // Use environment for cloud-hypervisor binary discovery
                        cloud_hypervisor_path: std::env::var("CLOUD_HYPERVISOR_PATH")
                            .map(std::path::PathBuf::from)
                            .ok(),
                        virtiofsd_path: std::env::var("VIRTIOFSD_PATH").map(std::path::PathBuf::from).ok(),
                        ..Default::default()
                    };

                    // Only register if kernel path is configured (indicates CI VM support enabled)
                    if !ch_config.kernel_path.as_os_str().is_empty() {
                        let blob_store_opt =
                            node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>);

                        match CloudHypervisorWorker::with_blob_store(ch_config.clone(), blob_store_opt) {
                            Ok(ch_worker) => {
                                // Register for both job type names
                                worker_service
                                    .register_handler("ci_vm", ch_worker)
                                    .await
                                    .context("failed to register Cloud Hypervisor worker")?;
                                info!(
                                    state_dir = ?ch_state_dir,
                                    pool_size = ch_config.pool_size,
                                    max_vms = ch_config.max_vms,
                                    "Cloud Hypervisor worker registered for VM-isolated CI jobs"
                                );
                            }
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    "Failed to create CloudHypervisorWorker. VM-isolated CI jobs disabled."
                                );
                            }
                        }
                    } else {
                        debug!(
                            "Cloud Hypervisor worker not registered: ASPEN_CI_KERNEL_PATH not set. \
                             Set this environment variable to enable VM-isolated CI jobs, \
                             or set ASPEN_CI_LOCAL_EXECUTOR=1 for direct process execution."
                        );
                    }
                }
            }
        }

        // Register shell command worker for executing system commands
        #[cfg(feature = "shell-worker")]
        {
            use aspen_jobs::workers::shell_command::ShellCommandWorker;
            use aspen_jobs::workers::shell_command::ShellCommandWorkerConfig;

            let has_auth = token_verifier.is_some();

            // Capture essential environment variables from parent process.
            // This allows shell workers to find binaries in nix store paths
            // when the node is running inside a nix develop shell.
            let mut default_env = std::collections::HashMap::new();
            for var in ["PATH", "HOME", "USER", "SHELL", "TERM", "LANG", "LC_ALL"] {
                if let Ok(value) = std::env::var(var) {
                    default_env.insert(var.to_string(), value);
                }
            }
            // Also capture Nix-specific variables for proper nix/cargo operation
            for var in [
                "NIX_PATH",
                "NIX_SSL_CERT_FILE",
                "SSL_CERT_FILE",
                "CARGO_HOME",
                "RUSTUP_HOME",
            ] {
                if let Ok(value) = std::env::var(var) {
                    default_env.insert(var.to_string(), value);
                }
            }

            let shell_config = ShellCommandWorkerConfig {
                node_id: config.node_id,
                token_verifier: token_verifier.clone(),
                blob_store: node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>),
                default_working_dir: std::env::temp_dir(),
                default_env,
            };
            info!(
                path_length = shell_config.default_env.get("PATH").map(|p| p.len()).unwrap_or(0),
                env_vars = shell_config.default_env.len(),
                "shell command worker configured with inherited environment"
            );
            let shell_worker = ShellCommandWorker::new(shell_config);
            worker_service
                .register_handler("shell_command", shell_worker)
                .await
                .context("failed to register shell command worker")?;
            if has_auth {
                info!("shell command worker registered (auth enabled)");
            } else {
                warn!("shell command worker registered WITHOUT auth (dev mode only)");
            }
        }

        // Register hook job worker if hooks are enabled
        if config.hooks.enabled {
            if let Some(hook_service) = node_mode.hook_service() {
                use aspen_hooks::constants::HOOK_JOB_TYPE;
                use aspen_hooks::worker::job_worker::HookWorkerImpl;

                let hook_worker = HookWorkerImpl::new(hook_service.registry());
                worker_service
                    .register_handler(HOOK_JOB_TYPE, hook_worker)
                    .await
                    .context("failed to register hook job worker")?;
                info!("hook job worker registered for job-mode handler execution");
            } else {
                debug!("hook job worker not registered: hook service not available");
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
    // Load secrets if configured
    #[cfg(feature = "secrets")]
    let secrets_manager = load_secrets(config, args).await?;

    // Create token verifier if authentication is enabled
    #[cfg(feature = "secrets")]
    let token_verifier_arc = setup_token_authentication(args, node_mode, secrets_manager.as_ref()).await?.map(Arc::new);
    #[cfg(not(feature = "secrets"))]
    let token_verifier_arc = setup_token_authentication(args, node_mode).await?.map(Arc::new);

    // Load Nix cache signer if configured
    #[cfg(all(feature = "secrets", feature = "nix-cache-gateway"))]
    let nix_cache_signer = load_nix_cache_signer(config, secrets_manager.as_ref(), kv_store).await?;

    // Create Client protocol context and handler
    // Create adapter for docs_sync if available
    let docs_sync_arc: Option<Arc<dyn aspen::api::DocsSyncProvider>> = node_mode.docs_sync().map(|ds| {
        Arc::new(aspen::protocol_adapters::DocsSyncProviderAdapter::new(ds.clone(), node_mode.blob_store().cloned()))
            as Arc<dyn aspen::api::DocsSyncProvider>
    });
    // Create adapter for peer_manager if available
    let peer_manager_arc: Option<Arc<dyn aspen::api::PeerManager>> = node_mode.peer_manager().map(|pm| {
        Arc::new(aspen::protocol_adapters::PeerManagerAdapter::new(pm.clone())) as Arc<dyn aspen::api::PeerManager>
    });

    // Create adapter for endpoint manager
    let endpoint_manager_adapter =
        Arc::new(aspen::protocol_adapters::EndpointProviderAdapter::new(node_mode.iroh_manager().clone()))
            as Arc<dyn aspen::api::EndpointProvider>;

    // Use the real IrpcRaftNetworkFactory which implements aspen_core::NetworkFactory
    let network_factory_arc: Arc<dyn aspen::api::NetworkFactory> = network_factory.clone();

    // Initialize ForgeNode if blob_store is available
    // Note: We create the Arc initially, then use Arc::get_mut to enable gossip
    // before the Arc is shared with other components
    #[cfg(feature = "forge")]
    let mut forge_node = node_mode.blob_store().map(|blob_store| {
        let secret_key = node_mode.iroh_manager().secret_key().clone();
        Arc::new(aspen::forge::ForgeNode::new(blob_store.clone(), kv_store.clone(), secret_key))
    });

    // Initialize PijulStore if blob_store and data_dir are available
    #[cfg(feature = "pijul")]
    let pijul_store = node_mode.blob_store().and_then(|blob_store| {
        config.data_dir.as_ref().map(|data_dir| {
            let store = Arc::new(aspen::pijul::PijulStore::new(blob_store.clone(), kv_store.clone(), data_dir.clone()));
            info!("Pijul store initialized at {:?}", data_dir);
            store
        })
    });

    // Initialize JobManager and start worker service if enabled
    let (job_manager, worker_service_handle, _worker_service_cancel) =
        initialize_job_system(config, node_mode, kv_store.clone(), token_verifier_arc.clone()).await?;

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

    // Initialize secrets service if secrets feature is enabled
    #[cfg(feature = "secrets")]
    let secrets_service = {
        // Create mount registry for dynamic multi-mount support
        // The registry creates stores on-demand with mount-specific storage prefixes
        let mount_registry =
            Arc::new(aspen_secrets::MountRegistry::new(kv_store.clone() as Arc<dyn aspen_core::KeyValueStore>));

        info!("Secrets service initialized with multi-mount support");
        Some(Arc::new(SecretsService::new(mount_registry)))
    };

    // Enable Forge gossip BEFORE creating CI trigger service to avoid Arc::get_mut failure
    // We pass None as the handler initially and set it later after the trigger service is created
    #[cfg(feature = "forge")]
    if config.forge.enable_gossip
        && let Some(ref mut forge_arc) = forge_node
        && let Some(gossip) = node_mode.iroh_manager().gossip()
    {
        // Get mutable reference to ForgeNode via Arc::get_mut
        // This is safe because the Arc hasn't been cloned yet (we do this BEFORE CI trigger service)
        if let Some(forge) = Arc::get_mut(forge_arc) {
            forge.enable_gossip(gossip.clone(), None).await.context("failed to enable forge gossip")?;
            info!("Forge gossip enabled (handler will be set after CI trigger service init)");
        } else {
            // This should never happen since we haven't cloned the Arc yet
            warn!("forge.enable_gossip: Arc has multiple owners, cannot enable gossip");
        }
    } else if config.forge.enable_gossip && forge_node.is_some() {
        #[cfg(feature = "forge")]
        warn!("forge.enable_gossip is true but gossip service not available");
    }

    // Initialize CI orchestrator if CI is enabled
    #[cfg(feature = "ci")]
    let ci_orchestrator = if config.ci.enabled {
        use std::time::Duration;

        use aspen_ci::PipelineOrchestrator;
        use aspen_ci::orchestrator::PipelineOrchestratorConfig;
        use aspen_jobs::WorkflowManager;

        // Create workflow manager for pipeline execution
        let workflow_manager = Arc::new(WorkflowManager::new(job_manager.clone(), kv_store.clone()));

        // Create orchestrator config from cluster config
        let orchestrator_config = PipelineOrchestratorConfig {
            max_runs_per_repo: config.ci.max_concurrent_runs.min(10),
            max_total_runs: 50, // Tiger Style: bounded total runs
            default_step_timeout: Duration::from_secs(config.ci.pipeline_timeout_secs.min(86400)),
            avoid_leader: config.ci.avoid_leader,
            resource_isolation: config.ci.resource_isolation,
            max_job_memory_bytes: config.ci.max_job_memory_bytes,
        };

        // Get optional blob store for artifact storage
        #[cfg(feature = "blob")]
        let blob_store_opt = node_mode.blob_store().map(|b| b.clone() as Arc<dyn aspen_blob::BlobStore>);
        #[cfg(not(feature = "blob"))]
        let blob_store_opt: Option<Arc<dyn aspen_blob::BlobStore>> = None;

        let orchestrator = Arc::new(PipelineOrchestrator::new(
            orchestrator_config,
            workflow_manager,
            job_manager.clone(),
            blob_store_opt,
            kv_store.clone(),
        ));

        // Initialize the orchestrator to set up job completion callbacks
        orchestrator.init().await;

        info!(
            max_concurrent = config.ci.max_concurrent_runs,
            auto_trigger = config.ci.auto_trigger,
            "CI pipeline orchestrator initialized"
        );

        Some(orchestrator)
    } else {
        None
    };

    // Initialize CI trigger service when auto_trigger is enabled
    #[cfg(all(feature = "ci", feature = "forge"))]
    let ci_trigger_service: Option<Arc<aspen_ci::TriggerService>> = if config.ci.enabled && config.ci.auto_trigger {
        use aspen_ci::ForgeConfigFetcher;
        use aspen_ci::OrchestratorPipelineStarter;
        use aspen_ci::TriggerService;
        use aspen_ci::TriggerServiceConfig;

        // Need both forge_node and orchestrator for auto-triggering
        match (&forge_node, &ci_orchestrator) {
            (Some(forge), Some(orchestrator)) => {
                // Create config fetcher backed by forge
                let config_fetcher = Arc::new(ForgeConfigFetcher::new(forge.clone()));

                // Create pipeline starter backed by orchestrator (with forge for repo checkout)
                let pipeline_starter = Arc::new(OrchestratorPipelineStarter::new(orchestrator.clone(), forge.clone()));

                // Create trigger service config
                let trigger_config = TriggerServiceConfig::default();

                // Create and return the trigger service
                let service = TriggerService::new(trigger_config, config_fetcher, pipeline_starter);

                // Watch any configured repositories for CI triggers
                if !config.ci.watched_repos.is_empty() {
                    for repo_id_hex in &config.ci.watched_repos {
                        match aspen_forge::identity::RepoId::from_hex(repo_id_hex) {
                            Ok(repo_id) => {
                                if let Err(e) = service.watch_repo(repo_id).await {
                                    warn!(
                                        repo_id = %repo_id_hex,
                                        error = %e,
                                        "Failed to watch repository for CI triggers"
                                    );
                                } else {
                                    info!(
                                        repo_id = %repo_id_hex,
                                        "Watching repository for CI triggers"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    repo_id = %repo_id_hex,
                                    error = %e,
                                    "Invalid repo ID in watched_repos config - skipping"
                                );
                            }
                        }
                    }
                }

                info!(
                    auto_trigger = true,
                    watched_repos = config.ci.watched_repos.len(),
                    "CI trigger service initialized - will auto-trigger on ref updates"
                );

                Some(service)
            }
            _ => {
                warn!("CI auto_trigger enabled but forge or orchestrator not available - disabling auto-trigger");
                None
            }
        }
    } else {
        None
    };

    #[cfg(all(feature = "ci", not(feature = "forge")))]
    let ci_trigger_service: Option<Arc<aspen_ci::TriggerService>> = None;

    // Set the CI trigger handler on the gossip service (gossip was enabled earlier)
    // This wires up the CI trigger handler to receive ref update announcements
    #[cfg(all(feature = "forge", feature = "ci"))]
    if config.forge.enable_gossip
        && config.ci.auto_trigger
        && let Some(ref forge_arc) = forge_node
        && let Some(ref trigger_service) = ci_trigger_service
    {
        let ci_handler: Arc<dyn aspen::forge::gossip::AnnouncementCallback> =
            Arc::new(aspen_ci::CiTriggerHandler::new(trigger_service.clone()));

        if let Err(e) = forge_arc.set_gossip_handler(Some(ci_handler)).await {
            warn!("Failed to set CI trigger handler on gossip: {}", e);
        } else {
            info!("CI trigger handler registered with forge gossip");
        }
    }

    // Initialize federation identity and trust manager if federation is enabled
    #[cfg(feature = "forge")]
    let (federation_identity, federation_trust_manager) = if config.federation.enabled {
        // Load or generate cluster identity
        let cluster_identity = load_federation_identity(config)?;

        // Create signed identity for sharing with other clusters
        let signed_identity = Arc::new(cluster_identity.to_signed());

        // Parse trusted clusters from config
        let trusted_keys = parse_trusted_cluster_keys(&config.federation.trusted_clusters)?;

        // Create trust manager with initial trusted clusters from config
        let trust_manager = Arc::new(aspen_cluster::federation::TrustManager::with_trusted(trusted_keys));

        info!(
            cluster_name = %signed_identity.name(),
            cluster_key = %signed_identity.public_key(),
            trusted_clusters = config.federation.trusted_clusters.len(),
            "federation identity initialized"
        );

        (Some(signed_identity), Some(trust_manager))
    } else {
        (None, None)
    };

    let client_context = ClientProtocolContext {
        node_id: config.node_id,
        controller: controller.clone(),
        kv_store: kv_store.clone(),
        #[cfg(feature = "sql")]
        sql_executor: primary_raft_node.clone(),
        state_machine: Some(primary_raft_node.state_machine().clone()),
        endpoint_manager: endpoint_manager_adapter,
        #[cfg(feature = "blob")]
        blob_store: node_mode.blob_store().cloned(),
        #[cfg(feature = "blob")]
        blob_replication_manager: node_mode.blob_replication_manager(),
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
        #[cfg(feature = "forge")]
        forge_node,
        #[cfg(feature = "pijul")]
        pijul_store,
        job_manager: Some(job_manager),
        worker_service: worker_service_handle.clone(),
        worker_coordinator: Some(worker_coordinator),
        watch_registry: Some(watch_registry),
        hook_service: node_mode.hook_service(),
        hooks_config: node_mode.hooks_config(),
        #[cfg(feature = "secrets")]
        secrets_service,
        // Federation fields - initialized when federation is enabled in config
        #[cfg(feature = "forge")]
        federation_identity,
        #[cfg(feature = "forge")]
        federation_trust_manager,
        // Federation discovery service for discovering other clusters via DHT
        // Currently initialized as None; will be populated when federation config
        // specifies a cluster identity and enables DHT discovery
        #[cfg(all(feature = "forge", feature = "global-discovery"))]
        federation_discovery: None,
        // CI/CD services - initialized above when ci.enabled is true
        #[cfg(feature = "ci")]
        ci_orchestrator,
        #[cfg(feature = "ci")]
        ci_trigger_service,
        // Nix cache signer - initialized above when nix_cache.enabled and signing configured
        #[cfg(feature = "nix-cache-gateway")]
        nix_cache_signer,
    };

    Ok((token_verifier_arc, client_context, worker_service_handle, coordinator_for_shutdown))
}

/// Setup the Iroh Router with all protocol handlers.
#[allow(unused_variables)] // kv_store used only with nix-cache-gateway feature
fn setup_router(
    config: &NodeConfig,
    node_mode: &NodeMode,
    client_handler: ClientProtocolHandler,
    watch_registry: Arc<dyn WatchRegistry>,
    kv_store: Arc<dyn KeyValueStore>,
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
            // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are
            // structurally identical (both use the same types from aspen-raft-types: AppRequest,
            // AppResponse, NodeId, RaftMemberInfo). This transmute is verified safe at compile time
            // by static_assertions in aspen_raft::types::_transmute_safety_static_checks. If the
            // types ever diverge, compilation will fail.
            let transport_raft: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
                unsafe { std::mem::transmute(handle.storage.raft_node.raft().as_ref().clone()) };
            let raft_handler = RaftProtocolHandler::new(transport_raft);
            builder = builder.accept(RAFT_ALPN, raft_handler);
        }
        NodeMode::Sharded(handle) => {
            // Sharded mode: register sharded Raft handler for multi-shard routing
            builder = builder.accept(RAFT_SHARDED_ALPN, handle.sharding.sharded_handler.clone());

            // Also register legacy ALPN routing to shard 0 for backward compatibility
            if let Some(shard_0) = handle.primary_shard() {
                // SAFETY: aspen_raft::types::AppTypeConfig and aspen_transport::rpc::AppTypeConfig are
                // structurally identical (both use the same types from aspen-raft-types: AppRequest,
                // AppResponse, NodeId, RaftMemberInfo). This transmute is verified safe at compile time
                // by static_assertions in aspen_raft::types::_transmute_safety_static_checks. If the
                // types ever diverge, compilation will fail.
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

    // Add Nix binary cache HTTP/3 gateway if enabled in config
    #[cfg(all(feature = "blob", feature = "nix-cache-gateway"))]
    if config.nix_cache.enabled {
        use aspen_cache::KvCacheIndex;
        use aspen_nix_cache_gateway::NIX_CACHE_H3_ALPN;
        use aspen_nix_cache_gateway::NixCacheGatewayConfig;
        use aspen_nix_cache_gateway::NixCacheProtocolHandler;

        if let Some(blob_store) = node_mode.blob_store() {
            // Create cache index backed by KV store
            let cache_index = Arc::new(KvCacheIndex::new(kv_store.clone()));

            // Build gateway config from NodeConfig
            let gateway_config = NixCacheGatewayConfig {
                store_dir: config.nix_cache.store_dir.to_string_lossy().to_string(),
                priority: config.nix_cache.priority,
                want_mass_query: config.nix_cache.want_mass_query,
                cache_name: config.nix_cache.cache_name.clone(),
                ..NixCacheGatewayConfig::default()
            };

            let handler = NixCacheProtocolHandler::new(
                gateway_config,
                cache_index,
                blob_store.clone(),
                client_context.nix_cache_signer.clone(),
            );
            builder = builder.accept(NIX_CACHE_H3_ALPN, handler);
            info!(
                priority = config.nix_cache.priority,
                want_mass_query = config.nix_cache.want_mass_query,
                cache_name = ?config.nix_cache.cache_name,
                "Nix cache gateway protocol handler registered (ALPN: {})",
                std::str::from_utf8(NIX_CACHE_H3_ALPN).unwrap_or("iroh+h3")
            );
        } else {
            warn!("nix_cache.enabled is true but blob store not available - cache gateway disabled");
        }
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
///
/// Generates a V2 ticket with direct addresses included, enabling connection
/// without discovery mechanisms. This is essential for:
/// - VM networking where mDNS doesn't traverse the host-VM boundary
/// - Relay-disabled environments (local testing)
/// - Air-gapped deployments
///
/// The ticket is written to multiple locations for discoverability:
/// 1. Logged via tracing (captured by log aggregators)
/// 2. Printed to stdout (for interactive use)
/// 3. Written to `{data_dir}/cluster-ticket.txt` or `/tmp/aspen-node-{node_id}-ticket.txt` (for
///    automated scripts like dogfood-vm.sh)
fn print_cluster_ticket(config: &NodeConfig, endpoint_addr: &iroh::EndpointAddr) {
    use std::io::Write;

    use aspen::cluster::ticket::AspenClusterTicketV2;
    use iroh_gossip::proto::TopicId;

    let hash = blake3::hash(config.cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    // Use V2 ticket format with direct addresses for relay-less connectivity
    let cluster_ticket = AspenClusterTicketV2::with_bootstrap_addr(topic_id, config.cookie.clone(), endpoint_addr);

    let ticket_str = cluster_ticket.serialize();
    let direct_addrs: Vec<_> = endpoint_addr
        .addrs
        .iter()
        .filter_map(|a| match a {
            iroh::TransportAddr::Ip(addr) => Some(addr),
            _ => None,
        })
        .collect();
    info!(
        ticket = %ticket_str,
        endpoint_id = %endpoint_addr.id,
        direct_addrs = ?direct_addrs,
        "cluster ticket generated (V2 with direct addresses)"
    );

    // Write ticket to a well-known file location for automated scripts
    // This is more reliable than parsing log files for the ticket string
    let ticket_file = if let Some(ref data_dir) = config.data_dir {
        data_dir.join("cluster-ticket.txt")
    } else {
        std::path::PathBuf::from(format!("/tmp/aspen-node-{}-ticket.txt", config.node_id))
    };

    match std::fs::File::create(&ticket_file).and_then(|mut f| {
        f.write_all(ticket_str.as_bytes())?;
        f.sync_all()
    }) {
        Ok(()) => info!(path = %ticket_file.display(), "cluster ticket written to file"),
        Err(e) => warn!(
            path = %ticket_file.display(),
            error = %e,
            "failed to write cluster ticket to file"
        ),
    }

    println!();
    println!("Connect with TUI:");
    println!("  aspen-tui --ticket {}", ticket_str);
    println!();
}
