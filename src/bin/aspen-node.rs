use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ControlPlaneError,
    DeterministicClusterController, DeterministicKeyValueStore, InitRequest, KeyValueStore,
    KeyValueStoreError, ReadRequest, WriteRequest,
};
use aspen::cluster::{IrohEndpointConfig, IrohEndpointManager, NodeServerConfig};
use aspen::raft::network::IrpcRaftNetworkFactory;
use aspen::raft::server::RaftRpcServer;
use aspen::raft::storage::{InMemoryLogStore, StateMachineStore};
use aspen::raft::types::{AppTypeConfig, NodeId};
use aspen::raft::{RaftActor, RaftActorConfig, RaftActorMessage, RaftControlClient};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::{Parser, ValueEnum};
use iroh::{EndpointAddr, SecretKey};
use openraft::Config as RaftConfig;
use ractor::{Actor, ActorRef, call_t};
use serde::Serialize;
use serde_json::json;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "aspen-node")]
struct Args {
    /// Logical Raft node identifier.
    #[arg(long)]
    node_id: u64,
    /// Hostname recorded in the NodeServer's identity (informational).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    /// Port for the Ractor node listener. Use 0 to request an OS-assigned port.
    #[arg(long, default_value_t = 26000)]
    port: u16,
    /// Shared cookie for authenticating Ractor nodes.
    #[arg(long, default_value = "aspen-cookie")]
    cookie: String,
    /// Address for the HTTP control API.
    #[arg(long, default_value = "127.0.0.1:8080")]
    http_addr: SocketAddr,
    /// Control-plane implementation to use for this node.
    #[arg(long, value_enum, default_value_t = ControlBackend::Deterministic)]
    control_backend: ControlBackend,
    /// Optional Iroh secret key (hex-encoded). If not provided, a new key is generated.
    #[arg(long)]
    iroh_secret_key: Option<String>,
    /// Optional Iroh relay server URL.
    #[arg(long)]
    iroh_relay_url: Option<String>,
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
    raft_port: u16,
}

#[derive(Clone)]
struct AppState {
    node_id: u64,
    raft_actor: ActorRef<RaftActorMessage>,
    raft_core: openraft::Raft<AppTypeConfig>,
    listen_port: u16,
    controller: ClusterControllerHandle,
    kv: KeyValueStoreHandle,
    iroh_manager: Option<Arc<IrohEndpointManager>>,
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

    // Create Iroh endpoint manager
    let iroh_manager = {
        let mut iroh_config = IrohEndpointConfig::new();

        // Parse secret key if provided
        if let Some(key_hex) = args.iroh_secret_key {
            let key_bytes = hex::decode(key_hex)?;
            if key_bytes.len() != 32 {
                anyhow::bail!("secret key must be 32 bytes (64 hex characters)");
            }
            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            let key = SecretKey::from_bytes(&key_array);
            iroh_config = iroh_config.with_secret_key(key);
        }

        // Parse relay URL if provided
        if let Some(relay_url_str) = args.iroh_relay_url {
            let relay_url = relay_url_str.parse()?;
            iroh_config = iroh_config.with_relay_url(relay_url)?;
        }

        let manager = IrohEndpointManager::new(iroh_config).await?;
        let endpoint_id = manager.endpoint().id();
        info!(
            endpoint_id = %endpoint_id,
            node_addr = ?manager.node_addr(),
            "Iroh endpoint created"
        );
        Arc::new(manager)
    };

    // Parse peer addresses
    // Note: EndpointAddr doesn't implement FromStr, so we'll need to construct it manually
    // For now, we'll skip peer parsing and document this for future implementation
    let peer_addrs: HashMap<NodeId, EndpointAddr> = HashMap::new();

    if !args.peers.is_empty() {
        warn!(
            peer_count = args.peers.len(),
            "peer address parsing not yet implemented - EndpointAddr requires manual construction"
        );
    }

    info!(peer_count = peer_addrs.len(), "parsed peer addresses");

    let node_server = NodeServerConfig::new(
        format!("node-{}", args.node_id),
        args.host.clone(),
        args.port,
        args.cookie.clone(),
    )
    .launch()
    .await?;
    info!(label = %node_server.label(), addr = %node_server.addr(), "node server online");

    let (raft_core, state_machine_store) = {
        let mut cfg = RaftConfig::default();
        cfg.heartbeat_interval = 500;
        cfg.election_timeout_min = 1_500;
        cfg.election_timeout_max = 3_000;
        let config = Arc::new(cfg.validate().expect("raft config"));

        let log_store = InMemoryLogStore::default();
        let state_machine_store = StateMachineStore::new();
        let network = IrpcRaftNetworkFactory::new(Arc::clone(&iroh_manager), peer_addrs);

        let raft = openraft::Raft::new(
            args.node_id,
            config,
            network,
            log_store,
            state_machine_store.clone(),
        )
        .await
        .expect("raft init");

        (raft, state_machine_store)
    };
    let raft_config = RaftActorConfig {
        node_id: args.node_id,
        raft: raft_core.clone(),
        state_machine: state_machine_store.clone(),
    };
    let (raft_ref, raft_task) = Actor::spawn(
        Some(format!("raft-{}", args.node_id)),
        RaftActor,
        raft_config,
    )
    .await?;

    let (controller, kv_store) = args.control_backend.build(raft_ref.clone());

    // Spawn IRPC server for Raft RPC
    let rpc_server = RaftRpcServer::spawn(Arc::clone(&iroh_manager), raft_core.clone());
    info!("IRPC server spawned for Raft RPC");

    let app_state = AppState {
        node_id: args.node_id,
        raft_actor: raft_ref.clone(),
        raft_core: raft_core.clone(),
        listen_port: args.port,
        controller,
        kv: kv_store,
        iroh_manager: Some(Arc::clone(&iroh_manager)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/iroh-metrics", get(iroh_metrics))
        .route("/init", post(init_cluster))
        .route("/add-learner", post(add_learner))
        .route("/change-membership", post(change_membership))
        .route("/write", post(write_value))
        .route("/read", post(read_value))
        // HTTP Raft RPC routes removed - now using IRPC over Iroh
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(args.http_addr).await?;
    info!(addr = %args.http_addr, "http server listening");

    // TODO: expose this router via iroh-h3 once the QUIC transport is wired.
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

    // begin shutdown sequence
    info!("shutting down IRPC server");
    rpc_server.shutdown().await?;

    info!("shutting down Iroh endpoint");
    iroh_manager.shutdown().await?;

    info!("shutting down node server");
    node_server.shutdown().await?;

    info!("shutting down Raft actor");
    raft_ref.stop(Some("http-shutdown".into()));
    raft_task.await?;

    Ok(())
}

async fn health(State(ctx): State<AppState>) -> impl IntoResponse {
    match call_t!(ctx.raft_actor, RaftActorMessage::GetNodeId, 25) {
        Ok(raft_node_id) => {
            let payload = HealthResponse {
                node_id: ctx.node_id,
                raft_node_id,
                raft_port: ctx.listen_port,
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
    let body = format!(
        "# TYPE aspen_node_info gauge\naspen_node_info{{node_id=\"{}\"}} 1\n",
        ctx.node_id
    );
    (StatusCode::OK, body)
}

async fn iroh_metrics(State(ctx): State<AppState>) -> impl IntoResponse {
    if let Some(manager) = &ctx.iroh_manager {
        let endpoint_id = manager.endpoint().id();
        let body = format!(
            "# TYPE iroh_node_info gauge\niroh_node_info{{endpoint_id=\"{}\"}} 1\n",
            endpoint_id
        );
        (StatusCode::OK, body).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
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

// HTTP Raft RPC handlers removed - now using IRPC over Iroh
// (raft_vote, raft_append, raft_snapshot)

#[derive(Debug)]
enum ApiError {
    Control(ControlPlaneError),
    KeyValue(KeyValueStoreError),
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
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ControlBackend {
    Deterministic,
    RaftActor,
}

impl ControlBackend {
    fn build(
        self,
        raft: ActorRef<RaftActorMessage>,
    ) -> (ClusterControllerHandle, KeyValueStoreHandle) {
        match self {
            ControlBackend::Deterministic => (
                DeterministicClusterController::new(),
                DeterministicKeyValueStore::new(),
            ),
            ControlBackend::RaftActor => {
                let client = Arc::new(RaftControlClient::new(raft));
                (client.clone(), client)
            }
        }
    }
}
