use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use anyhow::Result;
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ControlPlaneError,
    DeterministicClusterController, DeterministicKeyValueStore, InitRequest, KeyValueStore,
    KeyValueStoreError, ReadRequest, WriteRequest,
};
use aspen::cluster::bootstrap::{bootstrap_node, load_config, BootstrapHandle};
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::learner_promotion::{LearnerPromotionCoordinator, PromotionRequest};
use aspen::raft::network::IrpcRaftNetworkFactory;
use aspen::raft::supervision::HealthMonitor;
use aspen::raft::{RaftActorMessage, RaftControlClient};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use ractor::{ActorRef, call_t};
use serde::Serialize;
use serde_json::json;
use tokio::signal;
use tracing::{info, instrument, warn};
use tracing_subscriber::EnvFilter;

/// Default HTTP server address (127.0.0.1:8080).
///
/// Tiger Style: Compile-time constant to avoid runtime parsing.
const DEFAULT_HTTP_ADDR: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    8080,
);

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
    /// Options: "inmemory" (default), "redb"
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

    /// Hostname recorded in the NodeServer's identity (informational).
    #[arg(long)]
    host: Option<String>,

    /// Port for the Ractor node listener. Use 0 to request an OS-assigned port.
    #[arg(long, alias = "ractor-port")]
    port: Option<u16>,

    /// Shared cookie for authenticating Ractor nodes.
    #[arg(long)]
    cookie: Option<String>,

    /// Address for the HTTP control API.
    #[arg(long)]
    http_addr: Option<SocketAddr>,

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

    /// Optional Iroh relay server URL.
    #[arg(long)]
    iroh_relay_url: Option<String>,

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

    /// Custom Pkarr relay URL.
    /// Only relevant when --enable-pkarr is set.
    #[arg(long)]
    pkarr_relay_url: Option<String>,

    /// Peer node addresses in format: node_id@addr. Example: "1@<node-id>:<relay-url>:<direct-addrs>"
    /// Can be specified multiple times for multiple peers.
    #[arg(long)]
    peers: Vec<String>,
}

type ClusterControllerHandle = Arc<dyn ClusterController>;
type KeyValueStoreHandle = Arc<dyn KeyValueStore>;

/// Global metrics collector for errors and latencies.
///
/// Uses atomic counters for thread-safe, lock-free metrics collection.
/// Tiger Style: Fixed-size buckets, bounded memory usage.
struct MetricsCollector {
    // Error counters
    storage_errors: AtomicU64,
    network_errors: AtomicU64,
    rpc_errors: AtomicU64,

    // Write operation latency buckets (microseconds)
    // Buckets: <1ms, <10ms, <100ms, <1s, >=1s
    write_latency_us_bucket_1ms: AtomicU64,
    write_latency_us_bucket_10ms: AtomicU64,
    write_latency_us_bucket_100ms: AtomicU64,
    write_latency_us_bucket_1s: AtomicU64,
    write_latency_us_bucket_inf: AtomicU64,
    write_count: AtomicU64,
    write_total_us: AtomicU64,

    // Read operation latency buckets (microseconds)
    read_latency_us_bucket_1ms: AtomicU64,
    read_latency_us_bucket_10ms: AtomicU64,
    read_latency_us_bucket_100ms: AtomicU64,
    read_latency_us_bucket_1s: AtomicU64,
    read_latency_us_bucket_inf: AtomicU64,
    read_count: AtomicU64,
    read_total_us: AtomicU64,
}

impl MetricsCollector {
    const fn new() -> Self {
        Self {
            storage_errors: AtomicU64::new(0),
            network_errors: AtomicU64::new(0),
            rpc_errors: AtomicU64::new(0),

            write_latency_us_bucket_1ms: AtomicU64::new(0),
            write_latency_us_bucket_10ms: AtomicU64::new(0),
            write_latency_us_bucket_100ms: AtomicU64::new(0),
            write_latency_us_bucket_1s: AtomicU64::new(0),
            write_latency_us_bucket_inf: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            write_total_us: AtomicU64::new(0),

            read_latency_us_bucket_1ms: AtomicU64::new(0),
            read_latency_us_bucket_10ms: AtomicU64::new(0),
            read_latency_us_bucket_100ms: AtomicU64::new(0),
            read_latency_us_bucket_1s: AtomicU64::new(0),
            read_latency_us_bucket_inf: AtomicU64::new(0),
            read_count: AtomicU64::new(0),
            read_total_us: AtomicU64::new(0),
        }
    }

    fn record_storage_error(&self) {
        self.storage_errors.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn record_network_error(&self) {
        self.network_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn record_rpc_error(&self) {
        self.rpc_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn record_write_latency(&self, latency_us: u64) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.write_total_us.fetch_add(latency_us, Ordering::Relaxed);

        // Bucket the latency (Tiger Style: fixed buckets)
        if latency_us < 1_000 {
            // < 1ms
            self.write_latency_us_bucket_1ms.fetch_add(1, Ordering::Relaxed);
        } else if latency_us < 10_000 {
            // < 10ms
            self.write_latency_us_bucket_10ms.fetch_add(1, Ordering::Relaxed);
        } else if latency_us < 100_000 {
            // < 100ms
            self.write_latency_us_bucket_100ms.fetch_add(1, Ordering::Relaxed);
        } else if latency_us < 1_000_000 {
            // < 1s
            self.write_latency_us_bucket_1s.fetch_add(1, Ordering::Relaxed);
        } else {
            // >= 1s
            self.write_latency_us_bucket_inf.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn record_read_latency(&self, latency_us: u64) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.read_total_us.fetch_add(latency_us, Ordering::Relaxed);

        // Bucket the latency (Tiger Style: fixed buckets)
        if latency_us < 1_000 {
            // < 1ms
            self.read_latency_us_bucket_1ms.fetch_add(1, Ordering::Relaxed);
        } else if latency_us < 10_000 {
            // < 10ms
            self.read_latency_us_bucket_10ms.fetch_add(1, Ordering::Relaxed);
        } else if latency_us < 100_000 {
            // < 100ms
            self.read_latency_us_bucket_100ms.fetch_add(1, Ordering::Relaxed);
        } else if latency_us < 1_000_000 {
            // < 1s
            self.read_latency_us_bucket_1s.fetch_add(1, Ordering::Relaxed);
        } else {
            // >= 1s
            self.read_latency_us_bucket_inf.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Global metrics instance.
/// Tiger Style: Static initialization with const constructor.
static METRICS: OnceLock<MetricsCollector> = OnceLock::new();

fn metrics_collector() -> &'static MetricsCollector {
    METRICS.get_or_init(MetricsCollector::new)
}

/// Detailed health check response with individual component status.
#[derive(Serialize)]
struct DetailedHealthResponse {
    /// Overall health status: "healthy", "degraded", or "unhealthy"
    status: String,
    /// Individual health checks
    checks: HealthChecks,
    /// Node identification
    node_id: u64,
    /// Raft node ID
    raft_node_id: Option<u64>,
    /// Uptime in seconds
    uptime_seconds: u64,
    /// Supervision health monitoring (if enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    supervision: Option<SupervisionHealth>,
}

#[derive(Serialize)]
struct SupervisionHealth {
    /// Health monitor status: "healthy", "degraded", or "unhealthy"
    status: String,
    /// Number of consecutive health check failures
    consecutive_failures: u32,
    /// Whether supervision is enabled
    enabled: bool,
}

#[derive(Serialize)]
struct HealthChecks {
    /// Raft actor is responsive
    raft_actor: HealthCheckStatus,
    /// Has Raft cluster leader (self or other)
    raft_cluster: HealthCheckStatus,
    /// Disk space is available (< 95%)
    disk_space: HealthCheckStatus,
    /// Storage is writable
    storage: HealthCheckStatus,
}

#[derive(Serialize)]
struct HealthCheckStatus {
    /// "ok", "warning", or "error"
    status: String,
    /// Optional message with details
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Clone)]
struct AppState {
    node_id: u64,
    raft_actor: ActorRef<RaftActorMessage>,
    // Note: raft_core removed - all Raft operations go through RaftActor for proper actor supervision
    _ractor_port: u16,
    controller: ClusterControllerHandle,
    kv: KeyValueStoreHandle,
    network_factory: Arc<IrpcRaftNetworkFactory>,
    iroh_manager: Arc<aspen::cluster::IrohEndpointManager>,
    cluster_cookie: String,
    data_dir: Option<std::path::PathBuf>,
    promotion_coordinator: Option<Arc<LearnerPromotionCoordinator<RaftControlClient>>>,
    health_monitor: Option<Arc<HealthMonitor>>,
    start_time: Instant,
}

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
fn build_cluster_config(args: &Args) -> ClusterBootstrapConfig {
    ClusterBootstrapConfig {
        node_id: args.node_id.unwrap_or(0),
        data_dir: args.data_dir.clone(),
        storage_backend: args
            .storage_backend
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        redb_log_path: args.redb_log_path.clone(),
        redb_sm_path: args.redb_sm_path.clone(),
        sqlite_log_path: None,
        sqlite_sm_path: None,
        host: args.host.clone().unwrap_or_else(|| "127.0.0.1".into()),
        ractor_port: args.port.unwrap_or(26000),
        cookie: args.cookie.clone().unwrap_or_else(|| "aspen-cookie".into()),
        http_addr: args.http_addr.unwrap_or(DEFAULT_HTTP_ADDR),
        control_backend: args.control_backend.unwrap_or_default(),
        heartbeat_interval_ms: args.heartbeat_interval_ms.unwrap_or(500),
        election_timeout_min_ms: args.election_timeout_min_ms.unwrap_or(1500),
        election_timeout_max_ms: args.election_timeout_max_ms.unwrap_or(3000),
        iroh: IrohConfig {
            secret_key: args.iroh_secret_key.clone(),
            relay_url: args.iroh_relay_url.clone(),
            enable_gossip: !args.disable_gossip,
            gossip_ticket: args.ticket.clone(),
            enable_mdns: !args.disable_mdns,
            enable_dns_discovery: args.enable_dns_discovery,
            dns_discovery_url: args.dns_discovery_url.clone(),
            enable_pkarr: args.enable_pkarr,
            pkarr_relay_url: args.pkarr_relay_url.clone(),
        },
        peers: args.peers.clone(),
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    }
}

/// Setup controllers based on control backend configuration.
///
/// Returns tuple of (controller, kv_store, optional promotion coordinator).
///
/// Tiger Style: Single responsibility for controller initialization logic.
fn setup_controllers(
    config: &ClusterBootstrapConfig,
    handle: &BootstrapHandle,
) -> (
    ClusterControllerHandle,
    KeyValueStoreHandle,
    Option<Arc<LearnerPromotionCoordinator<RaftControlClient>>>,
) {
    match config.control_backend {
        ControlBackend::Deterministic => (
            DeterministicClusterController::new(),
            DeterministicKeyValueStore::new(),
            None,
        ),
        ControlBackend::RaftActor => {
            let cluster_client = Arc::new(RaftControlClient::new_with_capacity(
                handle.raft_actor.clone(),
                config.raft_mailbox_capacity,
                config.node_id,
            ));
            let kv_client = Arc::new(KvClient::new(handle.raft_actor.clone()));
            let coordinator = Arc::new(LearnerPromotionCoordinator::with_failure_detector(
                cluster_client.clone(),
                handle.network_factory.failure_detector().clone(),
            ));
            (cluster_client, kv_client, Some(coordinator))
        }
    }
}

/// Create application state from configuration and handles.
///
/// Tiger Style: Focused state construction function.
fn create_app_state(
    config: &ClusterBootstrapConfig,
    handle: &BootstrapHandle,
    controller: ClusterControllerHandle,
    kv_store: KeyValueStoreHandle,
    promotion_coordinator: Option<Arc<LearnerPromotionCoordinator<RaftControlClient>>>,
) -> AppState {
    AppState {
        node_id: config.node_id,
        raft_actor: handle.raft_actor.clone(),
        _ractor_port: config.ractor_port,
        controller,
        kv: kv_store,
        network_factory: handle.network_factory.clone(),
        iroh_manager: handle.iroh_manager.clone(),
        cluster_cookie: config.cookie.clone(),
        data_dir: config.data_dir.clone(),
        promotion_coordinator,
        health_monitor: handle.health_monitor.clone(),
        start_time: Instant::now(),
    }
}

/// Build Axum router with all API endpoints.
///
/// Tiger Style: Focused function for route configuration.
fn build_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/iroh-metrics", get(iroh_metrics))
        .route("/node-info", get(node_info))
        .route("/cluster-ticket", get(cluster_ticket))
        .route("/init", post(init_cluster))
        .route("/add-learner", post(add_learner))
        .route("/change-membership", post(change_membership))
        .route("/add-peer", post(add_peer))
        .route("/write", post(write_value))
        .route("/read", post(read_value))
        .route("/raft-metrics", get(raft_metrics))
        .route("/leader", get(get_leader))
        .route("/trigger-snapshot", post(trigger_snapshot))
        .route("/admin/promote-learner", post(promote_learner))
        .with_state(app_state)
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
        http_addr = %config.http_addr,
        control_backend = ?config.control_backend,
        "starting aspen node"
    );

    // Bootstrap the node
    let handle = bootstrap_node(config.clone()).await?;

    // Build controller and KV store based on control backend
    let (controller, kv_store, promotion_coordinator) = setup_controllers(&config, &handle);

    // Create application state
    let app_state = create_app_state(&config, &handle, controller, kv_store, promotion_coordinator);

    // Build router with all API endpoints
    let app = build_router(app_state);

    // Start HTTP server
    let listener = tokio::net::TcpListener::bind(config.http_addr).await?;
    info!(addr = %config.http_addr, "http server listening");

    let http = axum::serve(listener, app.into_make_service()).with_graceful_shutdown(async {
        let _ = signal::ctrl_c().await;
    });

    tokio::select! {
        result = http => {
            if let Err(err) = result {
                warn!(error = %err, "http server exited with error");
            }
        }
    }

    // Gracefully shutdown
    handle.shutdown().await?;

    Ok(())
}

/// Check Raft actor responsiveness.
///
/// Tiger Style: Focused function for a single health check.
async fn check_raft_actor_health(
    raft_actor: &ActorRef<RaftActorMessage>,
) -> (HealthCheckStatus, Option<u64>) {
    match call_t!(raft_actor, RaftActorMessage::GetNodeId, 25) {
        Ok(id) => (
            HealthCheckStatus {
                status: "ok".to_string(),
                message: None,
            },
            Some(id),
        ),
        Err(err) => (
            HealthCheckStatus {
                status: "error".to_string(),
                message: Some(format!("Raft actor not responding: {}", err)),
            },
            None,
        ),
    }
}

/// Check if Raft cluster has an elected leader.
///
/// Tiger Style: Focused function for a single health check.
async fn check_raft_cluster_health(controller: &ClusterControllerHandle) -> HealthCheckStatus {
    match controller.get_metrics().await {
        Ok(metrics) => {
            if metrics.current_leader.is_some() {
                HealthCheckStatus {
                    status: "ok".to_string(),
                    message: Some(format!("Leader: {:?}", metrics.current_leader)),
                }
            } else {
                HealthCheckStatus {
                    status: "warning".to_string(),
                    message: Some("No leader elected".to_string()),
                }
            }
        }
        Err(_) => HealthCheckStatus {
            status: "warning".to_string(),
            message: Some("Unable to get Raft metrics".to_string()),
        },
    }
}

/// Check disk space availability.
///
/// Tiger Style: Focused function for a single health check.
fn check_disk_space_health(data_dir: &Option<PathBuf>) -> HealthCheckStatus {
    if let Some(dir) = data_dir {
        match aspen::utils::check_disk_space(dir) {
            Ok(disk_space) => {
                if disk_space.usage_percent >= aspen::utils::DISK_USAGE_THRESHOLD_PERCENT {
                    HealthCheckStatus {
                        status: "error".to_string(),
                        message: Some(format!(
                            "Disk usage critical: {}% (threshold: {}%)",
                            disk_space.usage_percent,
                            aspen::utils::DISK_USAGE_THRESHOLD_PERCENT
                        )),
                    }
                } else if disk_space.usage_percent >= 80 {
                    HealthCheckStatus {
                        status: "warning".to_string(),
                        message: Some(format!("Disk usage high: {}%", disk_space.usage_percent)),
                    }
                } else {
                    HealthCheckStatus {
                        status: "ok".to_string(),
                        message: Some(format!("Disk usage: {}%", disk_space.usage_percent)),
                    }
                }
            }
            Err(e) => HealthCheckStatus {
                status: "warning".to_string(),
                message: Some(format!("Unable to check disk space: {}", e)),
            },
        }
    } else {
        HealthCheckStatus {
            status: "ok".to_string(),
            message: Some("In-memory storage (no disk check needed)".to_string()),
        }
    }
}

/// Check storage writability.
///
/// Tiger Style: Focused function for a single health check.
async fn check_storage_health(controller: &ClusterControllerHandle) -> HealthCheckStatus {
    match controller.get_metrics().await {
        Ok(_) => HealthCheckStatus {
            status: "ok".to_string(),
            message: None,
        },
        Err(e) => HealthCheckStatus {
            status: "warning".to_string(),
            message: Some(format!("Storage health check failed: {}", e)),
        },
    }
}

/// Check supervision health monitoring status.
///
/// Tiger Style: Focused function for a single health check.
async fn check_supervision_health(
    health_monitor: &Option<Arc<HealthMonitor>>,
) -> Option<SupervisionHealth> {
    if let Some(monitor) = health_monitor {
        use aspen::raft::supervision::HealthStatus;
        let status = monitor.get_status().await;
        let consecutive_failures = monitor.get_consecutive_failures();

        let status_str = match status {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        };

        Some(SupervisionHealth {
            status: status_str.to_string(),
            consecutive_failures,
            enabled: true,
        })
    } else {
        None
    }
}

/// Build health response from individual check results.
///
/// Tiger Style: Focused function for response construction.
#[allow(clippy::too_many_arguments)]
fn build_health_response(
    raft_actor_check: HealthCheckStatus,
    raft_cluster_check: HealthCheckStatus,
    disk_space_check: HealthCheckStatus,
    storage_check: HealthCheckStatus,
    supervision_health: Option<SupervisionHealth>,
    node_id: u64,
    raft_node_id: Option<u64>,
    uptime_seconds: u64,
) -> (StatusCode, Json<DetailedHealthResponse>) {
    let supervision_unhealthy = supervision_health
        .as_ref()
        .is_some_and(|s| s.status == "unhealthy");
    let supervision_degraded = supervision_health
        .as_ref()
        .is_some_and(|s| s.status == "degraded");

    let is_critical_failure = raft_actor_check.status == "error"
        || disk_space_check.status == "error"
        || supervision_unhealthy;
    let has_warnings = raft_cluster_check.status == "warning"
        || disk_space_check.status == "warning"
        || storage_check.status == "warning"
        || supervision_degraded;

    let (overall_status, http_status) = if is_critical_failure {
        ("unhealthy".to_string(), StatusCode::SERVICE_UNAVAILABLE)
    } else if has_warnings {
        ("degraded".to_string(), StatusCode::OK)
    } else {
        ("healthy".to_string(), StatusCode::OK)
    };

    let response = DetailedHealthResponse {
        status: overall_status,
        checks: HealthChecks {
            raft_actor: raft_actor_check,
            raft_cluster: raft_cluster_check,
            disk_space: disk_space_check,
            storage: storage_check,
        },
        node_id,
        raft_node_id,
        uptime_seconds,
        supervision: supervision_health,
    };

    (http_status, Json(response))
}

/// Health check endpoint with detailed component status.
///
/// Returns:
/// - 200 OK if all checks pass
/// - 503 SERVICE_UNAVAILABLE if any critical check fails
///
/// Checks performed:
/// 1. Raft actor responsiveness (critical)
/// 2. Raft cluster has leader (critical)
/// 3. Disk space availability (<95% used) (critical)
/// 4. Storage writability (warning only)
async fn health(State(ctx): State<AppState>) -> impl IntoResponse {
    // Perform all health checks
    let (raft_actor_check, raft_node_id) = check_raft_actor_health(&ctx.raft_actor).await;
    let raft_cluster_check = check_raft_cluster_health(&ctx.controller).await;
    let disk_space_check = check_disk_space_health(&ctx.data_dir);
    let storage_check = check_storage_health(&ctx.controller).await;
    let supervision_health = check_supervision_health(&ctx.health_monitor).await;

    // Build and return response
    let (http_status, json_response) = build_health_response(
        raft_actor_check,
        raft_cluster_check,
        disk_space_check,
        storage_check,
        supervision_health,
        ctx.node_id,
        raft_node_id,
        ctx.start_time.elapsed().as_secs(),
    );

    (http_status, json_response).into_response()
}

/// Append Raft core state metrics (leader, term, state, log indices, snapshot).
///
/// Tiger Style: Focused function for basic Raft state metrics.
fn append_raft_state_metrics(
    body: &mut String,
    node_id: u64,
    raft_metrics: &openraft::RaftMetrics<aspen::raft::types::AppTypeConfig>,
) {
    // Current leader
    body.push_str("# TYPE aspen_current_leader gauge\n");
    if let Some(leader) = raft_metrics.current_leader {
        body.push_str(&format!(
            "aspen_current_leader{{node_id=\"{}\"}} {}\n",
            node_id, leader
        ));
    }

    // Current term
    body.push_str("# TYPE aspen_current_term gauge\n");
    body.push_str(&format!(
        "aspen_current_term{{node_id=\"{}\"}} {}\n",
        node_id, raft_metrics.current_term
    ));

    // Server state (as string label)
    body.push_str("# TYPE aspen_state gauge\n");
    let state_str = format!("{:?}", raft_metrics.state);
    body.push_str(&format!(
        "aspen_state{{node_id=\"{}\",state=\"{}\"}} 1\n",
        node_id, state_str
    ));

    // Last log index
    if let Some(last_log_index) = raft_metrics.last_log_index {
        body.push_str("# TYPE aspen_last_log_index gauge\n");
        body.push_str(&format!(
            "aspen_last_log_index{{node_id=\"{}\"}} {}\n",
            node_id, last_log_index
        ));
    }

    // Last applied
    if let Some(ref last_applied) = raft_metrics.last_applied {
        body.push_str("# TYPE aspen_last_applied_index gauge\n");
        body.push_str(&format!(
            "aspen_last_applied_index{{node_id=\"{}\"}} {}\n",
            node_id, last_applied.index
        ));
    }

    // Snapshot index (if present)
    if let Some(ref snapshot) = raft_metrics.snapshot {
        body.push_str("# TYPE aspen_snapshot_index gauge\n");
        body.push_str(&format!(
            "aspen_snapshot_index{{node_id=\"{}\"}} {}\n",
            node_id, snapshot.index
        ));
    }

    // Apply lag (difference between last_log and last_applied)
    if let (Some(last_log), Some(last_applied)) =
        (raft_metrics.last_log_index, &raft_metrics.last_applied)
    {
        body.push_str("# TYPE aspen_apply_lag gauge\n");
        body.push_str("# HELP aspen_apply_lag Number of log entries not yet applied to state machine\n");
        let apply_lag = last_log.saturating_sub(last_applied.index);
        body.push_str(&format!(
            "aspen_apply_lag{{node_id=\"{}\"}} {}\n",
            node_id, apply_lag
        ));
    }
}

/// Append Raft replication metrics (replication lag, heartbeat, quorum).
///
/// Tiger Style: Focused function for leader-only replication metrics.
fn append_raft_leader_metrics(
    body: &mut String,
    node_id: u64,
    raft_metrics: &openraft::RaftMetrics<aspen::raft::types::AppTypeConfig>,
) {
    // Replication lag (leader only)
    if let Some(ref replication) = raft_metrics.replication
        && let Some(leader_last_log) = raft_metrics.last_log_index {
            body.push_str("# TYPE aspen_replication_lag gauge\n");
            body.push_str(
                "# HELP aspen_replication_lag Number of log entries follower is behind leader\n",
            );

            for (follower_id, matched_log_id) in replication.iter() {
                let lag = if let Some(matched) = matched_log_id {
                    leader_last_log.saturating_sub(matched.index)
                } else {
                    leader_last_log
                };

                body.push_str(&format!(
                    "aspen_replication_lag{{node_id=\"{}\",follower_id=\"{}\"}} {}\n",
                    node_id, follower_id, lag
                ));
            }
        }

    // Heartbeat metrics (leader only)
    if let Some(ref heartbeat) = raft_metrics.heartbeat {
        body.push_str("# TYPE aspen_heartbeat_seconds gauge\n");
        body.push_str("# HELP aspen_heartbeat_seconds Seconds since last heartbeat acknowledgment from follower\n");

        for (follower_id, last_ack_time_opt) in heartbeat.iter() {
            if let Some(last_ack_time) = last_ack_time_opt {
                let seconds_since_ack = last_ack_time.elapsed().as_secs_f64();
                body.push_str(&format!(
                    "aspen_heartbeat_seconds{{node_id=\"{}\",follower_id=\"{}\"}} {:.3}\n",
                    node_id, follower_id, seconds_since_ack
                ));
            }
        }
    }

    // Quorum acknowledgment age (leader only)
    if let Some(ref last_quorum_acked) = raft_metrics.last_quorum_acked {
        body.push_str("# TYPE aspen_quorum_acked_seconds gauge\n");
        body.push_str(
            "# HELP aspen_quorum_acked_seconds Seconds since last quorum acknowledgment\n",
        );
        let seconds = last_quorum_acked.elapsed().as_secs_f64();
        body.push_str(&format!(
            "aspen_quorum_acked_seconds{{node_id=\"{}\"}} {:.3}\n",
            node_id, seconds
        ));
    }
}

/// Append error counter metrics (storage, network, RPC errors).
///
/// Tiger Style: Focused function for error tracking metrics.
fn append_error_metrics(body: &mut String, node_id: u64, metrics: &MetricsCollector) {
    body.push_str("# TYPE aspen_errors_total counter\n");
    body.push_str("# HELP aspen_errors_total Total number of errors by type\n");
    body.push_str(&format!(
        "aspen_errors_total{{node_id=\"{}\",type=\"storage\"}} {}\n",
        node_id,
        metrics.storage_errors.load(std::sync::atomic::Ordering::Relaxed)
    ));
    body.push_str(&format!(
        "aspen_errors_total{{node_id=\"{}\",type=\"network\"}} {}\n",
        node_id,
        metrics.network_errors.load(std::sync::atomic::Ordering::Relaxed)
    ));
    body.push_str(&format!(
        "aspen_errors_total{{node_id=\"{}\",type=\"rpc\"}} {}\n",
        node_id,
        metrics.rpc_errors.load(std::sync::atomic::Ordering::Relaxed)
    ));
}

/// Append write latency histogram metrics.
///
/// Tiger Style: Focused function for write performance metrics.
fn append_write_latency_histogram(body: &mut String, node_id: u64, metrics: &MetricsCollector) {
    let write_count = metrics.write_count.load(std::sync::atomic::Ordering::Relaxed);
    if write_count == 0 {
        return;
    }

    body.push_str("# TYPE aspen_write_latency_seconds histogram\n");
    body.push_str("# HELP aspen_write_latency_seconds Write operation latency histogram\n");

    let bucket_1ms = metrics.write_latency_us_bucket_1ms.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_10ms = metrics.write_latency_us_bucket_10ms.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_100ms = metrics.write_latency_us_bucket_100ms.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_1s = metrics.write_latency_us_bucket_1s.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_inf = metrics.write_latency_us_bucket_inf.load(std::sync::atomic::Ordering::Relaxed);

    let mut cumulative = 0u64;
    cumulative += bucket_1ms;
    body.push_str(&format!(
        "aspen_write_latency_seconds_bucket{{node_id=\"{}\",le=\"0.001\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_10ms;
    body.push_str(&format!(
        "aspen_write_latency_seconds_bucket{{node_id=\"{}\",le=\"0.010\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_100ms;
    body.push_str(&format!(
        "aspen_write_latency_seconds_bucket{{node_id=\"{}\",le=\"0.100\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_1s;
    body.push_str(&format!(
        "aspen_write_latency_seconds_bucket{{node_id=\"{}\",le=\"1.000\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_inf;
    body.push_str(&format!(
        "aspen_write_latency_seconds_bucket{{node_id=\"{}\",le=\"+Inf\"}} {}\n",
        node_id, cumulative
    ));

    body.push_str(&format!(
        "aspen_write_latency_seconds_count{{node_id=\"{}\"}} {}\n",
        node_id, write_count
    ));

    let write_total_us = metrics.write_total_us.load(std::sync::atomic::Ordering::Relaxed);
    let write_sum_seconds = write_total_us as f64 / 1_000_000.0;
    body.push_str(&format!(
        "aspen_write_latency_seconds_sum{{node_id=\"{}\"}} {:.6}\n",
        node_id, write_sum_seconds
    ));
}

/// Append read latency histogram metrics.
///
/// Tiger Style: Focused function for read performance metrics.
fn append_read_latency_histogram(body: &mut String, node_id: u64, metrics: &MetricsCollector) {
    let read_count = metrics.read_count.load(std::sync::atomic::Ordering::Relaxed);
    if read_count == 0 {
        return;
    }

    body.push_str("# TYPE aspen_read_latency_seconds histogram\n");
    body.push_str("# HELP aspen_read_latency_seconds Read operation latency histogram\n");

    let bucket_1ms = metrics.read_latency_us_bucket_1ms.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_10ms = metrics.read_latency_us_bucket_10ms.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_100ms = metrics.read_latency_us_bucket_100ms.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_1s = metrics.read_latency_us_bucket_1s.load(std::sync::atomic::Ordering::Relaxed);
    let bucket_inf = metrics.read_latency_us_bucket_inf.load(std::sync::atomic::Ordering::Relaxed);

    let mut cumulative = 0u64;
    cumulative += bucket_1ms;
    body.push_str(&format!(
        "aspen_read_latency_seconds_bucket{{node_id=\"{}\",le=\"0.001\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_10ms;
    body.push_str(&format!(
        "aspen_read_latency_seconds_bucket{{node_id=\"{}\",le=\"0.010\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_100ms;
    body.push_str(&format!(
        "aspen_read_latency_seconds_bucket{{node_id=\"{}\",le=\"0.100\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_1s;
    body.push_str(&format!(
        "aspen_read_latency_seconds_bucket{{node_id=\"{}\",le=\"1.000\"}} {}\n",
        node_id, cumulative
    ));

    cumulative += bucket_inf;
    body.push_str(&format!(
        "aspen_read_latency_seconds_bucket{{node_id=\"{}\",le=\"+Inf\"}} {}\n",
        node_id, cumulative
    ));

    body.push_str(&format!(
        "aspen_read_latency_seconds_count{{node_id=\"{}\"}} {}\n",
        node_id, read_count
    ));

    let read_total_us = metrics.read_total_us.load(std::sync::atomic::Ordering::Relaxed);
    let read_sum_seconds = read_total_us as f64 / 1_000_000.0;
    body.push_str(&format!(
        "aspen_read_latency_seconds_sum{{node_id=\"{}\"}} {:.6}\n",
        node_id, read_sum_seconds
    ));
}

/// Append node failure detection metrics (unreachable nodes, duration).
///
/// Tiger Style: Focused function for failure tracking metrics.
fn append_failure_detection_metrics(
    body: &mut String,
    node_id: u64,
    detector: &aspen::raft::node_failure_detection::NodeFailureDetector,
) {
    // Count of currently unreachable nodes
    body.push_str("# TYPE aspen_unreachable_nodes gauge\n");
    body.push_str("# HELP aspen_unreachable_nodes Number of nodes currently unreachable\n");
    body.push_str(&format!(
        "aspen_unreachable_nodes{{node_id=\"{}\"}} {}\n",
        node_id,
        detector.unreachable_count()
    ));

    // Nodes needing attention (unreachable > threshold)
    let nodes_needing_attention = detector.get_nodes_needing_attention();
    body.push_str("# TYPE aspen_nodes_needing_attention gauge\n");
    body.push_str(
        "# HELP aspen_nodes_needing_attention Number of nodes requiring operator intervention\n",
    );
    body.push_str(&format!(
        "aspen_nodes_needing_attention{{node_id=\"{}\"}} {}\n",
        node_id,
        nodes_needing_attention.len()
    ));

    // Per-node unreachable duration
    body.push_str("# TYPE aspen_node_unreachable_seconds gauge\n");
    body.push_str(
        "# HELP aspen_node_unreachable_seconds Seconds a specific node has been unreachable\n",
    );

    for (peer_id, failure_type, duration) in nodes_needing_attention {
        let failure_type_str = match failure_type {
            aspen::raft::node_failure_detection::FailureType::ActorCrash => "actor_crash",
            aspen::raft::node_failure_detection::FailureType::NodeCrash => "node_crash",
            aspen::raft::node_failure_detection::FailureType::Healthy => "healthy",
        };

        body.push_str(&format!(
            "aspen_node_unreachable_seconds{{node_id=\"{}\",peer_id=\"{}\",failure_type=\"{}\"}} {:.3}\n",
            node_id, peer_id, failure_type_str, duration.as_secs_f64()
        ));
    }
}

/// Append health monitoring metrics (RaftActor health status).
///
/// Tiger Style: Focused function for health tracking metrics.
fn append_health_monitoring_metrics(body: &mut String, node_id: u64) {
    use aspen::raft::supervision::get_health_metrics;
    let (health_status, consecutive_failures, checks_total, failures_total) = get_health_metrics();

    body.push_str("# TYPE aspen_raft_actor_health_status gauge\n");
    body.push_str("# HELP aspen_raft_actor_health_status RaftActor health status (0=unhealthy, 1=degraded, 2=healthy)\n");
    body.push_str(&format!(
        "aspen_raft_actor_health_status{{node_id=\"{}\"}} {}\n",
        node_id, health_status
    ));

    body.push_str("# TYPE aspen_consecutive_health_failures gauge\n");
    body.push_str(
        "# HELP aspen_consecutive_health_failures Number of consecutive health check failures\n",
    );
    body.push_str(&format!(
        "aspen_consecutive_health_failures{{node_id=\"{}\"}} {}\n",
        node_id, consecutive_failures
    ));

    body.push_str("# TYPE aspen_health_checks_total counter\n");
    body.push_str("# HELP aspen_health_checks_total Total number of health checks performed\n");
    body.push_str(&format!(
        "aspen_health_checks_total{{node_id=\"{}\"}} {}\n",
        node_id, checks_total
    ));

    body.push_str("# TYPE aspen_health_check_failures_total counter\n");
    body.push_str(
        "# HELP aspen_health_check_failures_total Total number of health check failures\n",
    );
    body.push_str(&format!(
        "aspen_health_check_failures_total{{node_id=\"{}\"}} {}\n",
        node_id, failures_total
    ));
}

async fn metrics(State(ctx): State<AppState>) -> impl IntoResponse {
    let mut body = format!(
        "# TYPE aspen_node_info gauge\naspen_node_info{{node_id=\"{}\"}} 1\n",
        ctx.node_id
    );

    // Add Raft metrics if available
    if let Ok(raft_metrics) = ctx.controller.get_metrics().await {
        append_raft_state_metrics(&mut body, ctx.node_id, &raft_metrics);
        append_raft_leader_metrics(&mut body, ctx.node_id, &raft_metrics);
    }

    // Error counters
    let metrics = metrics_collector();
    append_error_metrics(&mut body, ctx.node_id, metrics);

    // Write and read latency histograms
    append_write_latency_histogram(&mut body, ctx.node_id, metrics);
    append_read_latency_histogram(&mut body, ctx.node_id, metrics);

    // Node failure detection metrics
    if let Ok(detector) = ctx.network_factory.failure_detector().try_read() {
        append_failure_detection_metrics(&mut body, ctx.node_id, &detector);
    }

    // Health monitoring metrics (if supervision is enabled)
    if ctx.health_monitor.is_some() {
        append_health_monitoring_metrics(&mut body, ctx.node_id);
    }

    (StatusCode::OK, body)
}

async fn iroh_metrics(State(_ctx): State<AppState>) -> impl IntoResponse {
    // Note: We don't have direct access to IrohEndpointManager in AppState anymore.
    // This endpoint would need to be restructured to access it from BootstrapHandle.
    // For now, return NOT_FOUND.
    StatusCode::NOT_FOUND.into_response()
}

async fn node_info(State(ctx): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "node_id": ctx.node_id,
        "endpoint_addr": ctx.iroh_manager.node_addr(),
    }))
}

async fn cluster_ticket(State(ctx): State<AppState>) -> impl IntoResponse {
    use aspen::cluster::ticket::AspenClusterTicket;
    use iroh_gossip::proto::TopicId;

    // Derive topic ID from cluster cookie using blake3
    let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    // Create ticket with this node as bootstrap peer
    let ticket = AspenClusterTicket::with_bootstrap(
        topic_id,
        ctx.cluster_cookie.clone(),
        ctx.iroh_manager.endpoint().id(),
    );

    let ticket_str = ticket.serialize();

    Json(json!({
        "ticket": ticket_str,
        "topic_id": format!("{:?}", topic_id),
        "cluster_id": ctx.cluster_cookie,
        "endpoint_id": ctx.iroh_manager.endpoint().id().to_string(),
    }))
}

/// Request to promote a learner to voter.
#[derive(Debug, serde::Deserialize)]
struct PromoteLearnerRequest {
    learner_id: u64,
    replace_node: Option<u64>,
    #[serde(default)]
    force: bool,
}

/// Response from learner promotion endpoint.
#[derive(serde::Serialize)]
struct PromoteLearnerResponse {
    success: bool,
    learner_id: u64,
    previous_voters: Vec<u64>,
    new_voters: Vec<u64>,
    message: String,
}

/// Promote a learner to voter with safety validation.
///
/// POST /admin/promote-learner
/// Body: { "learner_id": 4, "replace_node": 2, "force": false }
///
/// Safety checks (unless force=true):
/// - Membership cooldown elapsed (300s)
/// - Learner is healthy and reachable
/// - Learner is caught up on log (<100 entries behind)
/// - Cluster maintains quorum after change
#[instrument(skip(state), fields(node_id = state.node_id, learner_id = req.learner_id, replace_node = ?req.replace_node, force = req.force))]
async fn promote_learner(
    State(state): State<AppState>,
    Json(req): Json<PromoteLearnerRequest>,
) -> Result<Json<PromoteLearnerResponse>, (StatusCode, String)> {
    // Check if promotion coordinator is available
    let coordinator = state.promotion_coordinator.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_IMPLEMENTED,
            "Learner promotion not available with deterministic backend".to_string(),
        )
    })?;

    let promotion_request = PromotionRequest {
        learner_id: req.learner_id,
        replace_node: req.replace_node,
        force: req.force,
    };

    match coordinator.promote_learner(promotion_request).await {
        Ok(result) => {
            info!(
                learner_id = result.learner_id,
                previous_voters = ?result.previous_voters,
                new_voters = ?result.new_voters,
                "learner promoted successfully"
            );

            Ok(Json(PromoteLearnerResponse {
                success: true,
                learner_id: result.learner_id,
                previous_voters: result.previous_voters,
                new_voters: result.new_voters,
                message: format!(
                    "Learner {} promoted to voter successfully",
                    result.learner_id
                ),
            }))
        }
        Err(e) => {
            warn!(
                learner_id = req.learner_id,
                error = %e,
                "learner promotion failed"
            );
            Err((StatusCode::BAD_REQUEST, e.to_string()))
        }
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[instrument(skip(state), fields(node_id = state.node_id, members = request.initial_members.len()))]
async fn init_cluster(
    State(state): State<AppState>,
    Json(request): Json<InitRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.controller.init(request).await?;
    Ok(Json(result))
}

#[instrument(skip(state), fields(node_id = state.node_id, learner_id = request.learner.id))]
async fn add_learner(
    State(state): State<AppState>,
    Json(request): Json<AddLearnerRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.controller.add_learner(request).await?;
    Ok(Json(result))
}

#[instrument(skip(state), fields(node_id = state.node_id, new_members = ?request.members))]
async fn change_membership(
    State(state): State<AppState>,
    Json(request): Json<ChangeMembershipRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.controller.change_membership(request).await?;
    Ok(Json(result))
}

#[instrument(skip(state, request), fields(node_id = state.node_id, command = ?request.command))]
async fn write_value(
    State(state): State<AppState>,
    Json(request): Json<WriteRequest>,
) -> ApiResult<impl IntoResponse> {
    let start = Instant::now();

    // Tiger Style: Check disk space before writes to fail fast on resource exhaustion
    if let Some(ref data_dir) = state.data_dir {
        aspen::utils::ensure_disk_space_available(data_dir)
            .map_err(|e| {
                metrics_collector().record_storage_error();
                anyhow::anyhow!("disk space check failed: {}", e)
            })?;
    }

    let result = state.kv.write(request).await.inspect_err(|_e| {
        // Record RPC error for write failures
        metrics_collector().record_rpc_error();
    })?;

    // Record write latency
    let latency_us = start.elapsed().as_micros() as u64;
    metrics_collector().record_write_latency(latency_us);

    Ok(Json(result))
}

#[instrument(skip(state), fields(node_id = state.node_id, key = %request.key))]
async fn read_value(
    State(state): State<AppState>,
    Json(request): Json<ReadRequest>,
) -> ApiResult<impl IntoResponse> {
    let start = Instant::now();

    let result = state.kv.read(request).await.inspect_err(|_e| {
        // Record RPC error for read failures
        metrics_collector().record_rpc_error();
    })?;

    // Record read latency
    let latency_us = start.elapsed().as_micros() as u64;
    metrics_collector().record_read_latency(latency_us);

    Ok(Json(result))
}

#[derive(serde::Deserialize)]
struct AddPeerRequest {
    node_id: u64,
    endpoint_addr: iroh::EndpointAddr,
}

async fn add_peer(
    State(ctx): State<AppState>,
    Json(req): Json<AddPeerRequest>,
) -> impl IntoResponse {
    ctx.network_factory.add_peer(req.node_id, req.endpoint_addr).await;
    StatusCode::OK
}

/// Get detailed Raft metrics as JSON.
///
/// Returns structured metrics including node state, leader, log indices,
/// replication state, and heartbeat information.
async fn raft_metrics(State(ctx): State<AppState>) -> impl IntoResponse {
    match ctx.controller.get_metrics().await {
        Ok(metrics) => {
            // Return full RaftMetrics as JSON
            (StatusCode::OK, Json(json!({
                "node_id": ctx.node_id,
                "state": format!("{:?}", metrics.state),
                "current_leader": metrics.current_leader,
                "current_term": metrics.current_term,
                "last_log_index": metrics.last_log_index,
                "last_applied": metrics.last_applied.as_ref().map(|log_id| json!({
                    "term": log_id.leader_id.term,
                    "index": log_id.index
                })),
                "snapshot": metrics.snapshot.as_ref().map(|log_id| json!({
                    "term": log_id.leader_id.term,
                    "index": log_id.index
                })),
                "replication": metrics.replication.as_ref().map(|repl| {
                    let repl_map: std::collections::BTreeMap<String, Option<serde_json::Value>> = repl.iter().map(|(node_id, log_id_opt)| {
                        (node_id.to_string(), log_id_opt.as_ref().map(|log_id| json!({
                            "term": log_id.leader_id.term,
                            "index": log_id.index
                        })))
                    }).collect();
                    repl_map
                }),
                "millis_since_quorum_ack": metrics.last_quorum_acked.as_ref().map(|t| t.elapsed().as_millis()),
            }))).into_response()
        }
        Err(err) => {
            warn!(error = ?err, "failed to get raft metrics");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response()
        }
    }
}

/// Get the current leader ID.
///
/// Returns JSON: {"leader": 1} or {"leader": null}
async fn get_leader(State(ctx): State<AppState>) -> impl IntoResponse {
    match ctx.controller.get_leader().await {
        Ok(leader) => (StatusCode::OK, Json(json!({ "leader": leader }))).into_response(),
        Err(err) => {
            warn!(error = ?err, "failed to get leader");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response()
        }
    }
}

/// Trigger a snapshot immediately.
///
/// Returns the log ID of the created snapshot.
async fn trigger_snapshot(State(ctx): State<AppState>) -> impl IntoResponse {
    match ctx.controller.trigger_snapshot().await {
        Ok(snapshot_id) => (
            StatusCode::OK,
            Json(
                json!({ "snapshot": snapshot_id.as_ref().map(|log_id| json!({
                "term": log_id.leader_id.term,
                "index": log_id.index
            })) }),
            ),
        )
            .into_response(),
        Err(err) => {
            warn!(error = ?err, "failed to trigger snapshot");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response()
        }
    }
}

#[derive(Debug)]
enum ApiError {
    Control(ControlPlaneError),
    KeyValue(KeyValueStoreError),
    General(anyhow::Error),
}

impl From<ControlPlaneError> for ApiError {
    fn from(value: ControlPlaneError) -> Self {
        Self::Control(value)
    }
}

impl From<KeyValueStoreError> for ApiError {
    fn from(value: KeyValueStoreError) -> Self {
        Self::KeyValue(value)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::General(value)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::General(value.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Control(ControlPlaneError::InvalidRequest { reason }) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
            }
            ApiError::Control(ControlPlaneError::NotInitialized) => (
                StatusCode::PRECONDITION_FAILED,
                Json(json!({ "error": "cluster not initialized" })),
            )
                .into_response(),
            ApiError::Control(ControlPlaneError::Failed { reason }) => {
                (StatusCode::BAD_GATEWAY, Json(json!({ "error": reason }))).into_response()
            }
            ApiError::KeyValue(KeyValueStoreError::NotFound { key }) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("key '{key}' not found") })),
            )
                .into_response(),
            ApiError::KeyValue(KeyValueStoreError::Failed { reason }) => {
                (StatusCode::BAD_GATEWAY, Json(json!({ "error": reason }))).into_response()
            }
            ApiError::General(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response(),
        }
    }
}
