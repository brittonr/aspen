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

use anyhow::{Context, Result};
use aspen::api::{
    ClusterController, DeterministicClusterController, DeterministicKeyValueStore, KeyValueStore,
};
use aspen::auth::TokenVerifier;
use aspen::cluster::bootstrap::{NodeHandle, bootstrap_node, load_config};
use aspen::cluster::config::{ControlBackend, IrohConfig, NodeConfig};
use aspen::protocol_handlers::{
    ClientProtocolContext, ClientProtocolHandler, LOG_SUBSCRIBER_ALPN,
    LogSubscriberProtocolHandler, RaftProtocolHandler,
};
use clap::Parser;
use iroh::PublicKey;
use tokio::signal;
use tracing::{info, warn};
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
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

/// Build cluster configuration from CLI arguments.
///
/// Tiger Style: Focused function for config construction (single responsibility).
fn build_cluster_config(args: &Args) -> NodeConfig {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let mut config = NodeConfig::from_env();
    config.node_id = args.node_id.unwrap_or(0);
    config.data_dir = args.data_dir.clone();
    config.storage_backend = args
        .storage_backend
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();
    config.redb_log_path = args.redb_log_path.clone();
    config.redb_sm_path = args.redb_sm_path.clone();
    config.sqlite_log_path = None;
    config.sqlite_sm_path = None;
    config.host = args.host.clone().unwrap_or_else(|| "127.0.0.1".into());
    config.cookie = args.cookie.clone().unwrap_or_else(|| "aspen-cookie".into());
    // HTTP API removed - set to localhost:0 (unused)
    config.http_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    config.control_backend = args.control_backend.unwrap_or_default();
    config.heartbeat_interval_ms = args.heartbeat_interval_ms.unwrap_or(500);
    config.election_timeout_min_ms = args.election_timeout_min_ms.unwrap_or(1500);
    config.election_timeout_max_ms = args.election_timeout_max_ms.unwrap_or(3000);
    config.iroh = IrohConfig {
        secret_key: args.iroh_secret_key.clone(),
        enable_gossip: !args.disable_gossip,
        gossip_ticket: args.ticket.clone(),
        enable_mdns: !args.disable_mdns,
        enable_dns_discovery: args.enable_dns_discovery,
        dns_discovery_url: args.dns_discovery_url.clone(),
        enable_pkarr: args.enable_pkarr,
        enable_raft_auth: args.enable_raft_auth,
    };
    config.peers = args.peers.clone();
    config
}

/// Setup controllers based on control backend configuration.
///
/// Returns tuple of (controller, kv_store).
///
/// Tiger Style: Single responsibility for controller initialization logic.
fn setup_controllers(
    config: &NodeConfig,
    handle: &NodeHandle,
) -> (ClusterControllerHandle, KeyValueStoreHandle) {
    match config.control_backend {
        ControlBackend::Deterministic => (
            DeterministicClusterController::new(),
            DeterministicKeyValueStore::new(),
        ),
        ControlBackend::RaftActor => {
            // Use RaftNode directly as both controller and KV store
            let raft_node = handle.raft_node.clone();
            (raft_node.clone(), raft_node)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();
    let cli_config = build_cluster_config(&args);

    // Load configuration with proper precedence (env < TOML < CLI)
    let config = load_config(args.config.as_deref(), cli_config)?;

    // Validate configuration on startup (Tiger Style: fail-fast)
    config.validate()?;

    info!(
        node_id = config.node_id,
        control_backend = ?config.control_backend,
        "starting aspen node"
    );

    // Bootstrap the node with simplified architecture
    let mut handle = bootstrap_node(config.clone()).await?;

    // Generate and output root token if requested
    // This should only happen when initializing a NEW cluster
    if let Some(ref token_path) = args.output_root_token {
        // Generate root token using the node's Iroh secret key
        let secret_key = handle.iroh_manager.endpoint().secret_key();
        let token = aspen::auth::generate_root_token(
            secret_key,
            std::time::Duration::from_secs(365 * 24 * 60 * 60),
        )
        .context("failed to generate root token")?;

        // Encode to base64 for text transmission
        let token_base64 = token.to_base64().context("failed to encode root token")?;

        // Write to file
        std::fs::write(token_path, &token_base64)
            .with_context(|| format!("failed to write token to {}", token_path.display()))?;

        let issuer = token.issuer;
        info!(
            token_path = %token_path.display(),
            issuer = %issuer,
            "root token written to file"
        );

        // Store token in handle for programmatic access
        handle.root_token = Some(token);
    }

    // Build controller and KV store based on control backend
    let (controller, kv_store) = setup_controllers(&config, &handle);

    // Spawn Iroh Router with protocol handlers for ALPN-based dispatching
    let raft_handler = RaftProtocolHandler::new(handle.raft_node.raft().as_ref().clone());

    // Create Client protocol context and handler
    // Token auth is configurable via CLI flags
    let token_verifier = if args.enable_token_auth {
        let mut verifier = TokenVerifier::new();

        // Parse and add trusted root keys
        if args.trusted_root_key.is_empty() {
            // Default: use this node's Iroh public key as trusted root
            let node_public_key = handle.iroh_manager.endpoint().id();
            verifier = verifier.with_trusted_root(node_public_key);
            info!(
                trusted_root = %node_public_key,
                "Token auth enabled with node's own key as trusted root"
            );
        } else {
            for key_hex in &args.trusted_root_key {
                let key_bytes =
                    hex::decode(key_hex).context("Invalid hex in --trusted-root-key")?;
                let key = PublicKey::try_from(key_bytes.as_slice())
                    .context("Invalid Ed25519 public key in --trusted-root-key")?;
                verifier = verifier.with_trusted_root(key);
                info!(trusted_root = %key, "Added trusted root key");
            }
        }

        if args.require_token_auth {
            info!("Token auth enabled with strict mode (requests without tokens will be rejected)");
        } else {
            warn!(
                "Token auth enabled in migration mode (requests without tokens will be allowed with warnings)"
            );
        }

        Some(Arc::new(verifier))
    } else {
        info!("Token auth disabled - all client requests are allowed");
        None
    };

    let client_context = ClientProtocolContext {
        node_id: config.node_id,
        controller: controller.clone(),
        kv_store: kv_store.clone(),
        sql_executor: handle.raft_node.clone(),
        state_machine: Some(handle.raft_node.state_machine().clone()),
        endpoint_manager: handle.iroh_manager.clone(),
        blob_store: handle.blob_store.clone(),
        peer_manager: handle.peer_manager.clone(),
        cluster_cookie: config.cookie.clone(),
        start_time: Instant::now(),
        network_factory: Some(handle.network_factory.clone()),
        token_verifier,
        require_auth: args.require_token_auth,
    };
    let client_handler = ClientProtocolHandler::new(client_context);

    // Spawn the Router with all protocol handlers
    let router = {
        use aspen::protocol_handlers::{CLIENT_ALPN, RAFT_ALPN};
        use iroh::protocol::Router;
        use iroh_gossip::ALPN as GOSSIP_ALPN;

        let mut builder = Router::builder(handle.iroh_manager.endpoint().clone());
        builder = builder.accept(RAFT_ALPN, raft_handler);
        builder = builder.accept(CLIENT_ALPN, client_handler);

        // Add gossip handler if enabled
        if let Some(gossip) = handle.iroh_manager.gossip() {
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
        }

        // Add blobs protocol handler if blob store is enabled
        if let Some(ref blob_store) = handle.blob_store {
            let blobs_handler = blob_store.protocol_handler();
            builder = builder.accept(iroh_blobs::ALPN, blobs_handler);
            info!("Blobs protocol handler registered");
        }

        // Add docs sync protocol handler if docs sync is enabled
        if let Some(ref docs_sync) = handle.docs_sync {
            use aspen::docs::DOCS_SYNC_ALPN;
            let docs_handler = docs_sync.protocol_handler();
            builder = builder.accept(DOCS_SYNC_ALPN, docs_handler);
            info!(
                namespace = %docs_sync.namespace_id,
                "Docs sync protocol handler registered"
            );
        }

        // Add log subscriber protocol handler if log broadcast is enabled
        // This enables clients to watch for KV changes via the LOG_SUBSCRIBER_ALPN
        if let Some(ref log_sender) = handle.log_broadcast {
            use std::sync::atomic::AtomicU64;
            // Create committed_index tracker - updated by Raft when commits happen
            // For now, start at 0; the SqliteStateMachine broadcasts entries on commit
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

    let endpoint_id = handle.iroh_manager.endpoint().id();
    info!(
        endpoint_id = %endpoint_id,
        "Iroh Router spawned - all client API available via Iroh Client RPC (ALPN: aspen-tui)"
    );

    // Wait for shutdown signal
    shutdown_signal().await;

    // Gracefully shutdown Iroh Router
    info!("shutting down Iroh Router");
    router.shutdown().await?;

    // Gracefully shutdown bootstrap handle (includes RPC server actor, gossip, etc.)
    handle.shutdown().await?;

    Ok(())
}

/// Wait for shutdown signal (SIGINT or SIGTERM).
///
/// Tiger Style: Handles both signals for graceful shutdown in production
/// (systemd sends SIGTERM) and development (Ctrl-C sends SIGINT).
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
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
