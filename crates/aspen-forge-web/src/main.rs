//! Forge web frontend binary.
//!
//! Serves the Aspen Forge web UI over HTTP/3 via iroh-h3.
//! No axum — raw h3 request handling.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use tracing::info;

/// Aspen Forge web frontend.
#[derive(Parser)]
#[command(name = "aspen-forge-web")]
#[command(about = "Web frontend for Aspen Forge over HTTP/3 via iroh")]
struct Cli {
    /// Cluster ticket for connecting to the Aspen cluster.
    #[arg(long)]
    ticket: String,

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
    info!("connected");

    let state = Arc::new(aspen_forge_web::state::AppState::new(client));

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .bind()
        .await
        .context("failed to bind iroh endpoint")?;

    let addr = endpoint.addr();
    info!(
        endpoint_id = %addr.id.fmt_short(),
        "forge web serving HTTP/3 over iroh QUIC"
    );

    let handler = aspen_forge_web::server::ForgeH3Handler::new(state);
    let _router = iroh::protocol::Router::builder(endpoint)
        .accept(b"aspen/forge-web/1", handler)
        .spawn();

    // Wait for shutdown
    tokio::signal::ctrl_c().await.ok();
    info!("shutting down");
    Ok(())
}
