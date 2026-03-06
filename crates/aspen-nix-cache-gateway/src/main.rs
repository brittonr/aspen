//! Nix binary cache gateway for Aspen.
//!
//! A lightweight HTTP/1.1 server that translates Nix binary cache protocol
//! requests into Aspen cluster RPC calls. This is a protocol bridge, like
//! `git-remote-aspen` bridges Git protocol to Aspen.
//!
//! # Usage
//!
//! ```bash
//! aspen-nix-cache-gateway --ticket <cluster-ticket> [--port 8380] [--cache-name aspen-cache]
//! ```
//!
//! Then configure Nix:
//! ```bash
//! nix build --substituters http://127.0.0.1:8380 --trusted-public-keys "aspen-cache:<key>"
//! ```

mod client_kv;
mod server;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use aspen_cache::CacheSigningKey;
use aspen_cache::DEFAULT_CACHE_NAME;
use aspen_cache::signing::ensure_signing_key;
use aspen_client::AspenClient;
use clap::Parser;
use client_kv::ClientKvAdapter;
use tracing::info;

/// Nix binary cache gateway — translates HTTP cache requests to Aspen RPC.
#[derive(Parser)]
#[command(name = "aspen-nix-cache-gateway")]
#[command(about = "HTTP gateway for Aspen's distributed Nix binary cache")]
struct Cli {
    /// Cluster ticket for connecting to the Aspen cluster.
    #[arg(long)]
    ticket: String,

    /// HTTP port to listen on.
    #[arg(long, default_value = "8380")]
    port: u16,

    /// Bind address (default: localhost only).
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Cache name for signing (appears in public key and signatures).
    #[arg(long, default_value = DEFAULT_CACHE_NAME)]
    cache_name: String,

    /// RPC timeout in seconds.
    #[arg(long, default_value_t = 30)]
    timeout_secs: u64,
}

/// Shared state for the HTTP server.
pub struct GatewayState {
    /// Aspen client for cluster communication.
    pub client: AspenClient,
    /// Cache signing key.
    pub signing_key: CacheSigningKey,
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

    info!(
        port = cli.port,
        bind = %cli.bind,
        cache_name = %cli.cache_name,
        "starting nix cache gateway"
    );

    // Connect to cluster (for HTTP request handling)
    let timeout = Duration::from_secs(cli.timeout_secs);
    let client = AspenClient::connect(&cli.ticket, timeout, None).await.context("failed to connect to cluster")?;

    info!("connected to cluster");

    // Create separate KV client for signing key management
    let kv_client = AspenClient::connect(&cli.ticket, timeout, None).await.context("failed to connect KV client")?;
    let kv_store: Arc<dyn aspen_traits::KeyValueStore> = Arc::new(ClientKvAdapter::new(Arc::new(kv_client)));

    // Ensure signing key exists (generates if first time)
    let (signing_key, public_key) =
        ensure_signing_key(&kv_store, &cli.cache_name).await.context("failed to ensure signing key")?;

    info!(
        public_key = %public_key,
        "cache signing key ready — add to trusted-public-keys in nix.conf"
    );

    let state = Arc::new(GatewayState { client, signing_key });

    let addr: SocketAddr = format!("{}:{}", cli.bind, cli.port).parse().context("invalid bind address")?;

    server::run(state, addr).await
}
