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

    /// Start an embedded TCP proxy on this port for browser access.
    /// Bridges HTTP/1.1 on localhost to the h3 endpoint.
    #[arg(long)]
    tcp_port: Option<u16>,

    /// TCP proxy bind address (only used with --tcp-port).
    #[arg(long, default_value = "127.0.0.1")]
    tcp_bind: String,
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

    // Build full EndpointAddr with direct socket addresses so the
    // embedded TCP proxy can connect locally without DNS discovery.
    let mut target_addr = iroh::EndpointAddr::new(endpoint.id());
    for sock in endpoint.bound_sockets() {
        let fixed = if sock.ip().is_unspecified() {
            std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), sock.port())
        } else {
            sock
        };
        target_addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
    }

    info!(
        endpoint_id = %target_addr.id,
        endpoint_id_short = %target_addr.id.fmt_short(),
        "forge web serving HTTP/3 over iroh QUIC"
    );

    let handler = aspen_forge_web::server::ForgeH3Handler::new(state);
    let _router = iroh::protocol::Router::builder(endpoint).accept(b"aspen/forge-web/1", handler).spawn();

    // Optionally start embedded TCP proxy for browser access
    if let Some(tcp_port) = cli.tcp_port {
        let proxy_config = aspen_forge_web::ProxyConfig {
            bind_addr: cli.tcp_bind.clone(),
            port: tcp_port,
            target_addr,
            alpn: b"aspen/forge-web/1".to_vec(),
            request_timeout: std::time::Duration::from_secs(cli.timeout_secs),
        };
        let proxy = aspen_forge_web::H3Proxy::new(proxy_config);
        info!(port = tcp_port, "starting embedded TCP proxy for browser access");
        tokio::spawn(async move {
            if let Err(e) = proxy.run().await {
                tracing::error!(error = %e, "TCP proxy failed");
            }
        });
    }

    // Wait for shutdown
    tokio::signal::ctrl_c().await.ok();
    info!("shutting down");
    Ok(())
}
