//! Prometheus adapter for Aspen nodes.
//!
//! This lightweight HTTP server connects to Aspen nodes via the Iroh Client RPC
//! protocol and exposes Prometheus-format metrics for scraping.
//!
//! # Usage
//!
//! Connect via cluster ticket:
//! ```sh
//! aspen-prometheus-adapter --target aspen<ticket-data> --port 9090
//! ```
//!
//! Connect via bare endpoint ID:
//! ```sh
//! aspen-prometheus-adapter --target <endpoint-id> --port 9090
//! ```
//!
//! The adapter exposes:
//! - `GET /metrics` - Prometheus-format metrics from the Aspen node
//! - `GET /health` - Health check endpoint for the adapter itself

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use clap::Parser;
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use aspen::client_rpc::{
    ClientRpcRequest, ClientRpcResponse, MAX_CLIENT_MESSAGE_SIZE, MetricsResponse,
};
use aspen::cluster::ticket::AspenClusterTicket;

/// Client ALPN for identifying Client RPC connections.
const CLIENT_ALPN: &[u8] = b"aspen-client";

/// Default HTTP port for the adapter.
const DEFAULT_PORT: u16 = 9090;

/// Timeout for RPC calls to the Aspen node.
const RPC_TIMEOUT: Duration = Duration::from_secs(10);

/// Command line arguments.
#[derive(Parser, Debug)]
#[command(name = "aspen-prometheus-adapter")]
#[command(about = "Prometheus adapter for Aspen nodes")]
struct Args {
    /// Target Aspen node: cluster ticket (aspen...) or bare endpoint ID.
    #[arg(short, long, env = "ASPEN_TARGET")]
    target: String,

    /// HTTP port to listen on.
    #[arg(short, long, default_value_t = DEFAULT_PORT, env = "ASPEN_ADAPTER_PORT")]
    port: u16,

    /// Bind address for the HTTP server.
    #[arg(short, long, default_value = "0.0.0.0", env = "ASPEN_ADAPTER_BIND")]
    bind: String,
}

/// Application state shared between handlers.
struct AppState {
    /// Iroh endpoint for connecting to Aspen nodes.
    endpoint: Endpoint,
    /// Target node address.
    target_addr: EndpointAddr,
    /// Last successful scrape timestamp.
    last_success: RwLock<Option<std::time::Instant>>,
}

/// Parse target string as either a cluster ticket or a bare endpoint ID.
fn parse_target(target: &str) -> Result<EndpointAddr> {
    let target = target.trim();

    // Check if it's a cluster ticket (starts with "aspen")
    if target.starts_with("aspen") {
        let ticket =
            AspenClusterTicket::deserialize(target).context("failed to parse cluster ticket")?;

        // Use the first bootstrap peer
        let endpoint_id = ticket
            .bootstrap
            .iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("cluster ticket has no bootstrap peers"))?;

        Ok(EndpointAddr::new(*endpoint_id))
    } else {
        // Parse as bare endpoint ID
        let endpoint_id: EndpointId = target.parse().context("failed to parse endpoint ID")?;

        Ok(EndpointAddr::new(endpoint_id))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("aspen_prometheus_adapter=info".parse()?)
                .add_directive("iroh=warn".parse()?),
        )
        .init();

    let args = Args::parse();

    // Parse target address
    let target_addr = parse_target(&args.target)?;

    info!(
        target_node_id = %target_addr.id,
        port = args.port,
        bind = %args.bind,
        "starting Prometheus adapter"
    );

    // Create Iroh endpoint
    use rand::RngCore;
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    let secret_key = SecretKey::from(key_bytes);

    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![CLIENT_ALPN.to_vec()])
        .bind()
        .await
        .context("failed to bind Iroh endpoint")?;

    info!(
        adapter_node_id = %endpoint.id(),
        "Iroh endpoint created"
    );

    let state = Arc::new(AppState {
        endpoint,
        target_addr,
        last_success: RwLock::new(None),
    });

    // Build HTTP router
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(state);

    // Bind and serve
    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .context("invalid bind address")?;

    info!("listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Handler for `/metrics` endpoint.
///
/// Fetches metrics from the Aspen node via Client RPC and returns them
/// in Prometheus text format.
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match fetch_metrics(&state).await {
        Ok(metrics) => {
            // Update last success time
            let mut last_success = state.last_success.write().await;
            *last_success = Some(std::time::Instant::now());

            (
                StatusCode::OK,
                [("content-type", "text/plain; charset=utf-8")],
                metrics.prometheus_text,
            )
        }
        Err(e) => {
            error!("failed to fetch metrics: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                [("content-type", "text/plain; charset=utf-8")],
                format!("# Failed to fetch metrics from Aspen node: {}\n", e),
            )
        }
    }
}

/// Handler for `/health` endpoint.
///
/// Returns health status of the adapter itself.
async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let last_success = state.last_success.read().await;

    // Consider healthy if we've never scraped or last scrape was within 5 minutes
    let healthy = match *last_success {
        Some(instant) => instant.elapsed() < Duration::from_secs(300),
        None => true, // Haven't scraped yet, so we're nominally healthy
    };

    if healthy {
        (StatusCode::OK, "OK")
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "UNHEALTHY: No recent successful scrapes",
        )
    }
}

/// Fetch metrics from the Aspen node via Client RPC.
async fn fetch_metrics(state: &AppState) -> Result<MetricsResponse> {
    debug!(
        target_node_id = %state.target_addr.id,
        "fetching metrics from Aspen node"
    );

    // Connect to the target node with timeout
    let connection = tokio::time::timeout(
        RPC_TIMEOUT,
        state
            .endpoint
            .connect(state.target_addr.clone(), CLIENT_ALPN),
    )
    .await
    .context("connection timeout")?
    .context("failed to connect to Aspen node")?;

    // Open a bidirectional stream
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open stream")?;

    // Send GetMetrics request
    let request = ClientRpcRequest::GetMetrics;
    let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

    send.write_all(&request_bytes)
        .await
        .context("failed to send request")?;
    send.finish().context("failed to finish send stream")?;

    // Read the response with timeout
    let response_bytes =
        tokio::time::timeout(RPC_TIMEOUT, recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE))
            .await
            .context("read timeout")?
            .context("failed to read response")?;

    // Deserialize the response
    let response: ClientRpcResponse =
        postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

    match response {
        ClientRpcResponse::Metrics(metrics) => Ok(metrics),
        ClientRpcResponse::Error(err) => {
            anyhow::bail!("RPC error {}: {}", err.code, err.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}
