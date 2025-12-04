use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ControlPlaneError,
    DeterministicClusterController, DeterministicKeyValueStore, InitRequest, KeyValueStore,
    KeyValueStoreError, ReadRequest, WriteRequest,
};
use aspen::cluster::bootstrap::{bootstrap_node, load_config};
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::network::IrpcRaftNetworkFactory;
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

#[derive(Serialize)]
struct HealthResponse {
    node_id: u64,
    raft_node_id: u64,
    ractor_port: u16,
}

#[derive(Clone)]
struct AppState {
    node_id: u64,
    raft_actor: ActorRef<RaftActorMessage>,
    // Note: raft_core removed - all Raft operations go through RaftActor for proper actor supervision
    ractor_port: u16,
    controller: ClusterControllerHandle,
    kv: KeyValueStoreHandle,
    network_factory: Arc<IrpcRaftNetworkFactory>,
    iroh_manager: Arc<aspen::cluster::IrohEndpointManager>,
    cluster_cookie: String,
    data_dir: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    let args = Args::parse();

    // Build configuration from CLI args
    let cli_config = ClusterBootstrapConfig {
        node_id: args.node_id.unwrap_or(0),
        data_dir: args.data_dir,
        storage_backend: args
            .storage_backend
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        redb_log_path: args.redb_log_path,
        redb_sm_path: args.redb_sm_path,
        host: args.host.unwrap_or_else(|| "127.0.0.1".into()),
        ractor_port: args.port.unwrap_or(26000),
        cookie: args.cookie.unwrap_or_else(|| "aspen-cookie".into()),
        http_addr: args.http_addr.unwrap_or_else(|| {
            "127.0.0.1:8080"
                .parse()
                .expect("hardcoded default address is valid")
        }),
        control_backend: args.control_backend.unwrap_or_default(),
        heartbeat_interval_ms: args.heartbeat_interval_ms.unwrap_or(500),
        election_timeout_min_ms: args.election_timeout_min_ms.unwrap_or(1500),
        election_timeout_max_ms: args.election_timeout_max_ms.unwrap_or(3000),
        iroh: IrohConfig {
            secret_key: args.iroh_secret_key,
            relay_url: args.iroh_relay_url,
            enable_gossip: !args.disable_gossip,
            gossip_ticket: args.ticket,
            enable_mdns: !args.disable_mdns,
            enable_dns_discovery: args.enable_dns_discovery,
            dns_discovery_url: args.dns_discovery_url,
            enable_pkarr: args.enable_pkarr,
            pkarr_relay_url: args.pkarr_relay_url,
        },
        peers: args.peers,
    };

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
    let (controller, kv_store): (ClusterControllerHandle, KeyValueStoreHandle) =
        match config.control_backend {
            ControlBackend::Deterministic => (
                DeterministicClusterController::new(),
                DeterministicKeyValueStore::new(),
            ),
            ControlBackend::RaftActor => {
                let cluster_client = Arc::new(RaftControlClient::new(handle.raft_actor.clone()));
                let kv_client = Arc::new(KvClient::new(handle.raft_actor.clone()));
                (cluster_client, kv_client)
            }
        };

    let app_state = AppState {
        node_id: config.node_id,
        raft_actor: handle.raft_actor.clone(),
        ractor_port: config.ractor_port,
        controller,
        kv: kv_store,
        network_factory: handle.network_factory.clone(),
        iroh_manager: handle.iroh_manager.clone(),
        cluster_cookie: config.cookie.clone(),
        data_dir: config.data_dir.clone(),
    };

    let app = Router::new()
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
        .with_state(app_state);

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

async fn health(State(ctx): State<AppState>) -> impl IntoResponse {
    match call_t!(ctx.raft_actor, RaftActorMessage::GetNodeId, 25) {
        Ok(raft_node_id) => {
            let payload = HealthResponse {
                node_id: ctx.node_id,
                raft_node_id,
                ractor_port: ctx.ractor_port,
            };
            (StatusCode::OK, Json(payload)).into_response()
        }
        Err(err) => {
            warn!(error = ?err, "raft RPC transport failure");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn metrics(State(ctx): State<AppState>) -> impl IntoResponse {
    let mut body = format!(
        "# TYPE aspen_node_info gauge\naspen_node_info{{node_id=\"{}\"}} 1\n",
        ctx.node_id
    );

    // Add Raft metrics if available
    if let Ok(raft_metrics) = ctx.controller.get_metrics().await {
        // Current leader
        body.push_str("# TYPE aspen_current_leader gauge\n");
        if let Some(leader) = raft_metrics.current_leader {
            body.push_str(&format!(
                "aspen_current_leader{{node_id=\"{}\"}} {}\n",
                ctx.node_id, leader
            ));
        }

        // Current term
        body.push_str("# TYPE aspen_current_term gauge\n");
        body.push_str(&format!(
            "aspen_current_term{{node_id=\"{}\"}} {}\n",
            ctx.node_id, raft_metrics.current_term
        ));

        // Server state (as string label)
        body.push_str("# TYPE aspen_state gauge\n");
        let state_str = format!("{:?}", raft_metrics.state);
        body.push_str(&format!(
            "aspen_state{{node_id=\"{}\",state=\"{}\"}} 1\n",
            ctx.node_id, state_str
        ));

        // Last log index
        if let Some(last_log_index) = raft_metrics.last_log_index {
            body.push_str("# TYPE aspen_last_log_index gauge\n");
            body.push_str(&format!(
                "aspen_last_log_index{{node_id=\"{}\"}} {}\n",
                ctx.node_id, last_log_index
            ));
        }

        // Last applied
        if let Some(ref last_applied) = raft_metrics.last_applied {
            body.push_str("# TYPE aspen_last_applied_index gauge\n");
            body.push_str(&format!(
                "aspen_last_applied_index{{node_id=\"{}\"}} {}\n",
                ctx.node_id, last_applied.index
            ));
        }
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

type ApiResult<T> = Result<T, ApiError>;

async fn init_cluster(
    State(state): State<AppState>,
    Json(request): Json<InitRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.controller.init(request).await?;
    Ok(Json(result))
}

async fn add_learner(
    State(state): State<AppState>,
    Json(request): Json<AddLearnerRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.controller.add_learner(request).await?;
    Ok(Json(result))
}

async fn change_membership(
    State(state): State<AppState>,
    Json(request): Json<ChangeMembershipRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.controller.change_membership(request).await?;
    Ok(Json(result))
}

async fn write_value(
    State(state): State<AppState>,
    Json(request): Json<WriteRequest>,
) -> ApiResult<impl IntoResponse> {
    // Tiger Style: Check disk space before writes to fail fast on resource exhaustion
    if let Some(ref data_dir) = state.data_dir {
        aspen::utils::ensure_disk_space_available(data_dir)
            .map_err(|e| anyhow::anyhow!("disk space check failed: {}", e))?;
    }

    let result = state.kv.write(request).await?;
    Ok(Json(result))
}

async fn read_value(
    State(state): State<AppState>,
    Json(request): Json<ReadRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.kv.read(request).await?;
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
    ctx.network_factory.add_peer(req.node_id, req.endpoint_addr);
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
