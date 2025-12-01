use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, anyhow};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::{Parser, ValueEnum};
use iroh::{EndpointAddr, EndpointId, SecretKey};
use reqwest::Url;
use serde::Serialize;
use serde_json::json;
use tokio::signal;
use tracing::{error, info, warn};

use aspen::StorageSurface;
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterState, ControlPlaneError,
    ExternalControlPlane, HiqliteBackendConfig, HiqliteControlPlane, InMemoryControlPlane,
    InitRequest, KeyValueStore, KeyValueStoreError, ReadRequest, WriteRequest,
};
use aspen::cluster::{
    DeterministicClusterConfig, IrohClusterConfig, IrohClusterTransport, NodeServerConfig,
};
use aspen::raft::{LocalRaftFactory, RaftActorFactory, RaftActorMessage, RaftNodeSpec};
use aspen::storage::StoragePlan;
use openraft::Config;
use ractor::ActorRef;

const IROH_ONLINE_TIMEOUT_SECS: u64 = 5;

#[derive(Parser, Debug)]
#[command(
    name = "aspen-node",
    about = "Runs an Aspen node with HTTP control/metrics endpoints"
)]
struct Cli {
    /// Unique Raft node identifier.
    #[arg(long)]
    id: u64,

    /// HTTP listener address (e.g. 127.0.0.1:21001).
    #[arg(long, default_value = "127.0.0.1:21001")]
    http_addr: SocketAddr,

    /// Cluster host portion used in NodeServer name and default listener.
    #[arg(long, default_value = "127.0.0.1")]
    cluster_host: String,

    /// Cluster listener port. Set to 0 for OS-assigned.
    #[arg(long, default_value_t = 0)]
    cluster_port: u16,

    /// Magic cookie shared by all peers for authentication.
    #[arg(long, default_value = "aspen-cookie")]
    cookie: String,

    /// Optional data directory for persistent storage.
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Deterministic seed used by in-memory storage.
    #[arg(long)]
    deterministic_seed: Option<u64>,

    /// Cluster namespace used for actor naming.
    #[arg(long, default_value = "aspen::primary")]
    cluster_namespace: String,

    /// Backend used to service control-plane requests.
    #[arg(long, value_enum, default_value = "in-memory")]
    control_backend: ControlBackend,

    /// Base URL for the external backend (required when `--control-backend external`).
    #[arg(long)]
    backend_url: Option<String>,

    /// Timeout (in seconds) for backend HTTP requests.
    #[arg(long, default_value_t = 5)]
    backend_timeout_secs: u64,

    /// Host:port of a Hiqlite node (repeat to provide multiple nodes).
    #[arg(long = "hiqlite-node", value_name = "HOST:PORT")]
    hiqlite_nodes: Vec<String>,

    /// API secret expected by the remote Hiqlite nodes.
    #[arg(long)]
    hiqlite_api_secret: Option<String>,

    /// Enable TLS when talking to Hiqlite nodes.
    #[arg(long, default_value_t = false)]
    hiqlite_tls: bool,

    /// Disable TLS certificate verification for Hiqlite connections.
    #[arg(long, default_value_t = false)]
    hiqlite_tls_no_verify: bool,

    /// Indicates that the provided Hiqlite nodes are proxy instances.
    #[arg(long, default_value_t = false)]
    hiqlite_proxy_mode: bool,

    /// Table used inside Hiqlite for Aspen's key/value store.
    #[arg(long, default_value = "aspen_kv")]
    hiqlite_table: String,

    /// Enable Iroh transport for cluster actor traffic.
    #[arg(long)]
    enable_iroh: bool,

    /// Peer EndpointAddrs (EndpointId or JSON) to connect to when Iroh transport is enabled.
    #[arg(long = "iroh-peer")]
    iroh_peers: Vec<String>,

    /// Hex-encoded 32-byte Iroh secret key (for deterministic EndpointIds).
    #[arg(long)]
    iroh_secret_hex: Option<String>,

    /// File to write the local Iroh endpoint info (JSON) once online.
    #[arg(long)]
    iroh_endpoint_file: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ControlBackend {
    #[value(name = "in-memory")]
    InMemory,
    External,
    Hiqlite,
}

fn parse_endpoint_addr_str(input: &str) -> anyhow::Result<EndpointAddr> {
    if let Ok(id) = EndpointId::from_str(input) {
        return Ok(EndpointAddr::from(id));
    }
    serde_json::from_str(input)
        .map_err(|err| anyhow!("failed to parse EndpointAddr from '{input}': {err}"))
}

fn parse_secret_key_hex(input: &str) -> anyhow::Result<SecretKey> {
    let hex = input.trim_start_matches("0x");
    let bytes =
        hex::decode(hex).map_err(|err| anyhow!("invalid iroh secret hex '{input}': {err}"))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("iroh secret hex '{input}' must decode to 32 bytes"))?;
    Ok(SecretKey::from_bytes(&array))
}

fn write_iroh_endpoint_file(path: &Path, transport: &IrohClusterTransport) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }
    }
    let addr = transport.endpoint_addr();
    let info = json!({
        "endpoint_id": addr.id.to_string(),
        "relay_urls": addr.relay_urls().map(|r| r.to_string()).collect::<Vec<_>>(),
        "ip_addrs": addr.ip_addrs().map(|ip| ip.to_string()).collect::<Vec<_>>(),
    });
    let data = serde_json::to_vec_pretty(&info)?;
    fs::write(path, data).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[derive(Clone)]
struct AppState {
    node_id: u64,
    cluster: String,
    storage: StorageSurface,
    raft: ActorRef<RaftActorMessage>,
    controller: Arc<dyn ClusterController>,
    kv: Arc<dyn KeyValueStore>,
}

#[derive(Serialize)]
struct MetricsResponse {
    node_id: u64,
    cluster: String,
    log_next_index: u64,
    last_applied: u64,
    cluster_state: ClusterState,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    node_id: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let plan = StoragePlan {
        node_id: format!("node-{}", cli.id),
        seed: cli.deterministic_seed,
        data_dir: cli.data_dir.clone(),
    };
    let storage = plan.materialize().context("initialize storage surface")?;

    let mut node_config = NodeServerConfig::new(
        format!("node-{}", cli.id),
        cli.cluster_host.clone(),
        cli.cluster_port,
        cli.cookie.clone(),
    );
    if let Some(seed) = cli.deterministic_seed {
        node_config = node_config.with_determinism(DeterministicClusterConfig {
            simulation_seed: Some(seed),
        });
    }
    let node_server = node_config.launch().await.context("launch NodeServer")?;

    let raft_spec = RaftNodeSpec {
        node_id: cli.id,
        cluster: cli.cluster_namespace.clone(),
    };
    let raft_actor =
        LocalRaftFactory::spawn_actor(raft_spec, storage.clone(), &node_server, Config::default())
            .await
            .context("spawn Raft actor")?;

    let mut iroh_transport: Option<IrohClusterTransport> = None;
    if cli.enable_iroh {
        let mut iroh_cfg = IrohClusterConfig::default();
        if let Some(secret_hex) = &cli.iroh_secret_hex {
            let key = parse_secret_key_hex(secret_hex)?;
            iroh_cfg.secret_key = Some(key);
        }
        let transport = IrohClusterTransport::spawn(&node_server, iroh_cfg)
            .await
            .context("launch iroh cluster transport")?;
        if let Some(path) = &cli.iroh_endpoint_file {
            write_iroh_endpoint_file(path, &transport)?;
        }
        let wait_result = tokio::time::timeout(
            Duration::from_secs(IROH_ONLINE_TIMEOUT_SECS),
            transport.wait_until_online(),
        )
        .await;
        match wait_result {
            Ok(_) => {
                info!(
                    endpoint = ?transport.endpoint_addr(),
                    "iroh transport online"
                );
                if let Some(path) = &cli.iroh_endpoint_file {
                    write_iroh_endpoint_file(path, &transport)?;
                }
            }
            Err(_) => {
                warn!(
                    timeout_secs = IROH_ONLINE_TIMEOUT_SECS,
                    endpoint = ?transport.endpoint_addr(),
                    "iroh transport did not signal online before timeout; continuing without waiting"
                );
            }
        }
        for peer in &cli.iroh_peers {
            match parse_endpoint_addr_str(peer) {
                Ok(addr) => {
                    if let Err(err) = transport.connect(addr).await {
                        warn!(peer = peer, error = %err, "failed to connect to iroh peer");
                    } else {
                        info!(peer = peer, "connected to iroh peer");
                    }
                }
                Err(err) => warn!(peer = peer, error = %err, "invalid iroh peer address"),
            }
        }
        iroh_transport = Some(transport);
    }

    let (controller, kv): (Arc<dyn ClusterController>, Arc<dyn KeyValueStore>) =
        match cli.control_backend {
            ControlBackend::InMemory => {
                let plane = Arc::new(InMemoryControlPlane::new());
                (plane.clone(), plane)
            }
            ControlBackend::External => {
                let raw_url = cli
                    .backend_url
                    .clone()
                    .context("external backend requires --backend-url")?;
                let url = Url::parse(&raw_url).context("invalid backend url")?;
                let plane = Arc::new(
                    ExternalControlPlane::new(url, Duration::from_secs(cli.backend_timeout_secs))
                        .context("build external backend client")?,
                );
                (plane.clone(), plane)
            }
            ControlBackend::Hiqlite => {
                let api_secret = cli
                    .hiqlite_api_secret
                    .clone()
                    .context("hiqlite backend requires --hiqlite-api-secret")?;
                let config = HiqliteBackendConfig {
                    nodes: cli.hiqlite_nodes.clone(),
                    api_secret,
                    use_tls: cli.hiqlite_tls,
                    tls_no_verify: cli.hiqlite_tls_no_verify,
                    proxy_mode: cli.hiqlite_proxy_mode,
                    table: cli.hiqlite_table.clone(),
                };
                let plane = Arc::new(
                    HiqliteControlPlane::connect(config)
                        .await
                        .context("connect hiqlite backend")?,
                );
                (plane.clone(), plane)
            }
        };

    let state = AppState {
        node_id: cli.id,
        cluster: cli.cluster_namespace.clone(),
        storage: storage.clone(),
        raft: raft_actor,
        controller,
        kv,
    };

    let router_state = state.clone();
    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/init", post(init_cluster))
        .route("/add-learner", post(add_learner))
        .route("/change-membership", post(change_membership))
        .route("/write", post(write_handler))
        .route("/read", post(read_handler))
        .with_state(router_state);

    info!("aspen-node {} listening on {}", cli.id, cli.http_addr);

    let listener = tokio::net::TcpListener::bind(cli.http_addr)
        .await
        .context("bind HTTP listener")?;
    let server = axum::serve(listener, app);

    tokio::select! {
        res = server => {
            if let Err(err) = res {
                error!(error = %err, "http server failed");
            }
        },
        _ = signal::ctrl_c() => {
            info!("received shutdown signal");
        }
    }

    info!("shutting down raft actor and NodeServer");
    state.raft.stop(None);
    if let Some(transport) = iroh_transport {
        transport
            .shutdown()
            .await
            .context("shutdown iroh transport")?;
    }
    node_server
        .shutdown()
        .await
        .context("shutdown NodeServer")?;
    Ok(())
}

#[tracing::instrument(skip(state))]
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    ok_response(json!(HealthResponse {
        status: "ok",
        node_id: state.node_id,
    }))
}

#[tracing::instrument(skip(state))]
async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_state = match state.controller.current_state().await {
        Ok(state) => state,
        Err(err) => return control_plane_error(err),
    };
    let log_next_index = state
        .storage
        .log()
        .next_index()
        .unwrap_or(1)
        .saturating_sub(1);
    let last_applied = state.storage.state_machine().last_applied().unwrap_or(0);
    let body = MetricsResponse {
        node_id: state.node_id,
        cluster: state.cluster.clone(),
        log_next_index,
        last_applied,
        cluster_state,
    };
    Json(body).into_response()
}

#[tracing::instrument(skip(state, body))]
async fn init_cluster(
    State(state): State<AppState>,
    Json(body): Json<InitRequest>,
) -> impl IntoResponse {
    match state.controller.init(body).await {
        Ok(cluster) => ok_response(json!({ "cluster": cluster })),
        Err(err) => control_plane_error(err),
    }
}

#[tracing::instrument(skip(state, body))]
async fn add_learner(
    State(state): State<AppState>,
    Json(body): Json<AddLearnerRequest>,
) -> impl IntoResponse {
    match state.controller.add_learner(body).await {
        Ok(cluster) => ok_response(json!({ "cluster": cluster })),
        Err(err) => control_plane_error(err),
    }
}

#[tracing::instrument(skip(state, body))]
async fn change_membership(
    State(state): State<AppState>,
    Json(body): Json<ChangeMembershipRequest>,
) -> impl IntoResponse {
    match state.controller.change_membership(body).await {
        Ok(cluster) => ok_response(json!({ "cluster": cluster })),
        Err(err) => control_plane_error(err),
    }
}

#[tracing::instrument(skip(state, body))]
async fn write_handler(
    State(state): State<AppState>,
    Json(body): Json<WriteRequest>,
) -> impl IntoResponse {
    match state.kv.write(body).await {
        Ok(result) => ok_response(json!({ "write": result })),
        Err(err) => kv_error(err),
    }
}

#[tracing::instrument(skip(state, body))]
async fn read_handler(
    State(state): State<AppState>,
    Json(body): Json<ReadRequest>,
) -> impl IntoResponse {
    match state.kv.read(body).await {
        Ok(result) => ok_response(json!({ "read": result })),
        Err(err) => kv_error(err),
    }
}

fn ok_response(payload: serde_json::Value) -> axum::response::Response {
    Json(payload).into_response()
}

fn error_response(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(json!({ "message": msg }))).into_response()
}

fn control_plane_error(err: ControlPlaneError) -> axum::response::Response {
    match err {
        ControlPlaneError::InvalidRequest { reason } => {
            warn!(reason = %reason, "control-plane rejected request");
            error_response(StatusCode::BAD_REQUEST, &reason)
        }
        ControlPlaneError::NotFound { reason } => {
            warn!(reason = %reason, "control-plane reported missing resource");
            error_response(StatusCode::NOT_FOUND, &reason)
        }
        ControlPlaneError::Backend { reason } => {
            error!(reason = %reason, "control-plane backend failure");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &reason)
        }
    }
}

fn kv_error(err: KeyValueStoreError) -> axum::response::Response {
    match err {
        KeyValueStoreError::NotFound { key } => {
            warn!(%key, "key not found");
            error_response(StatusCode::NOT_FOUND, "key not found")
        }
        KeyValueStoreError::Backend { reason } => {
            error!(reason = %reason, "key-value backend failure");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &reason)
        }
    }
}
