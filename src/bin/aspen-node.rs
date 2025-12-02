use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use aspen::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ControlPlaneError,
    DeterministicClusterController, DeterministicKeyValueStore, InitRequest, KeyValueStore,
    KeyValueStoreError, ReadRequest, WriteRequest,
};
use aspen::cluster::{IrohClusterConfig, IrohClusterTransport, IrohEndpoint, NodeServerConfig};
use aspen::raft::network::HttpRaftNetworkFactory;
use aspen::raft::storage::{InMemoryLogStore, StateMachineStore};
use aspen::raft::types::AppTypeConfig;
use aspen::raft::{RaftActor, RaftActorConfig, RaftActorMessage, RaftControlClient};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::{Parser, ValueEnum};
use openraft::Config as RaftConfig;
use openraft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};
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
    iroh_metrics: Option<IrohEndpoint>,
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
        let network = HttpRaftNetworkFactory::new();

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

    let iroh_transport =
        match IrohClusterTransport::spawn(&node_server, IrohClusterConfig::default()).await {
            Ok(transport) => {
                transport.wait_until_online().await;
                Some(transport)
            }
            Err(err) => {
                warn!(error = %err, "failed to launch iroh cluster transport");
                None
            }
        };

    let app_state = AppState {
        node_id: args.node_id,
        raft_actor: raft_ref.clone(),
        raft_core: raft_core.clone(),
        listen_port: args.port,
        controller,
        kv: kv_store,
        iroh_metrics: iroh_transport.as_ref().map(|t| t.endpoint()),
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
        .route("/raft/vote", post(raft_vote))
        .route("/raft/append", post(raft_append))
        .route("/raft/snapshot", post(raft_snapshot))
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
    node_server.shutdown().await?;
    if let Some(transport) = iroh_transport {
        transport.shutdown().await?;
    }
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
    if let Some(endpoint) = &ctx.iroh_metrics {
        (StatusCode::OK, endpoint.metrics_snapshot()).into_response()
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

async fn raft_vote(
    State(state): State<AppState>,
    Json(request): Json<VoteRequest<AppTypeConfig>>,
) -> impl IntoResponse {
    Json(state.raft_core.vote(request).await)
}

async fn raft_append(
    State(state): State<AppState>,
    Json(request): Json<AppendEntriesRequest<AppTypeConfig>>,
) -> impl IntoResponse {
    Json(state.raft_core.append_entries(request).await)
}

async fn raft_snapshot(
    State(state): State<AppState>,
    Json(request): Json<InstallSnapshotRequest<AppTypeConfig>>,
) -> impl IntoResponse {
    Json(state.raft_core.install_snapshot(request).await)
}

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
