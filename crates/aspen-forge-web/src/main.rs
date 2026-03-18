//! Forge web frontend binary.
//!
//! Serves the Aspen Forge web UI over HTTP/3 via iroh-h3-axum.
//! Optionally also serves over TCP for browser compatibility.

use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use tracing::info;

/// Aspen Forge web frontend.
#[derive(Parser)]
#[command(name = "aspen-forge-web")]
#[command(about = "Web frontend for Aspen Forge, served over HTTP/3 via iroh")]
struct Cli {
    /// Cluster ticket for connecting to the Aspen cluster.
    #[arg(long)]
    ticket: String,

    /// Optional TCP port for HTTP/1.1 fallback (for browsers).
    #[arg(long)]
    tcp_port: Option<u16>,

    /// RPC timeout in seconds.
    #[arg(long, default_value_t = 30)]
    timeout_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let timeout = Duration::from_secs(cli.timeout_secs);

    info!("connecting to Aspen cluster...");
    let client = aspen_client::AspenClient::connect(&cli.ticket, timeout, None)
        .await
        .context("failed to connect to cluster")?;
    info!("connected to cluster");

    let state = aspen_forge_web::state::AppState::new(client);
    let router = aspen_forge_web::routes::router(state);

    // Start iroh-h3 serving
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .bind()
        .await
        .context("failed to bind iroh endpoint")?;

    let addr = endpoint.addr();
    info!(
        endpoint_id = %addr.id.fmt_short(),
        "forge web serving over HTTP/3 via iroh QUIC"
    );

    let h3_handler = iroh_h3_axum::IrohAxum::new(router.clone());
    let _iroh_router = iroh::protocol::Router::builder(endpoint)
        .accept(b"aspen/forge-web/1", h3_handler)
        .spawn();

    // Optionally start TCP fallback for browsers
    if let Some(port) = cli.tcp_port {
        let addr = format!("0.0.0.0:{port}");
        info!(%addr, "forge web also serving over HTTP/1.1 (TCP fallback)");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .context("failed to bind TCP listener")?;
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("TCP server error")?;
    } else {
        // Just wait for shutdown
        shutdown_signal().await;
    }

    info!("shutting down");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
}
