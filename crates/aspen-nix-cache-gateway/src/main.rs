//! Nix binary cache gateway for Aspen.
//!
//! A lightweight HTTP server that translates Nix binary cache protocol
//! requests into Aspen cluster operations. Two backends:
//!
//! - **nar-bridge** (feature `snix-http`): Uses nar-bridge's axum router with Aspen's
//!   `BlobService`/`DirectoryService`/`PathInfoService` trait impls. Supports GET, HEAD, PUT, range
//!   requests, and compression.
//!
//! - **legacy** (feature `legacy-http`): Hand-rolled hyper server using RPC calls. Read-only (GET
//!   only), no range requests.
//!
//! # Usage
//!
//! ```bash
//! aspen-nix-cache-gateway --ticket <cluster-ticket> [--port 8380]
//! ```
//!
//! Then configure Nix:
//! ```bash
//! nix build --substituters http://127.0.0.1:8380 --trusted-public-keys "aspen-cache:<key>"
//! ```

mod client_kv;

#[cfg(feature = "snix-http")]
mod server_nar_bridge;

#[cfg(feature = "legacy-http")]
mod server;

use aspen_cache::DEFAULT_CACHE_NAME;
use clap::Parser;
use tracing::info;

/// Nix binary cache gateway — translates HTTP cache requests to Aspen storage.
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

/// Configuration extracted from CLI args, shared between backends.
pub struct GatewayConfig {
    pub ticket: String,
    pub port: u16,
    pub bind: String,
    pub cache_name: String,
    pub timeout_secs: u64,
}

/// Shared state for the legacy HTTP server.
#[cfg(feature = "legacy-http")]
pub struct GatewayState {
    /// Aspen client for cluster communication.
    pub client: aspen_client::AspenClient,
    /// Cache signing key.
    pub signing_key: aspen_cache::CacheSigningKey,
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

    let config = GatewayConfig {
        ticket: cli.ticket,
        port: cli.port,
        bind: cli.bind,
        cache_name: cli.cache_name,
        timeout_secs: cli.timeout_secs,
    };

    #[cfg(feature = "snix-http")]
    {
        info!("using nar-bridge backend");
        server_nar_bridge::run(&config).await
    }

    #[cfg(all(feature = "legacy-http", not(feature = "snix-http")))]
    {
        info!("using legacy HTTP backend");
        run_legacy(&config).await
    }

    #[cfg(not(any(feature = "snix-http", feature = "legacy-http")))]
    {
        let _ = config;
        anyhow::bail!("no HTTP backend enabled — build with 'snix-http' or 'legacy-http' feature")
    }
}

/// Run the legacy hyper-based HTTP server.
#[cfg(feature = "legacy-http")]
async fn run_legacy(config: &GatewayConfig) -> anyhow::Result<()> {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::Context;
    use aspen_cache::signing::ensure_signing_key;
    use aspen_client::AspenClient;

    let timeout = Duration::from_secs(config.timeout_secs);
    let client = AspenClient::connect(&config.ticket, timeout, None).await.context("failed to connect to cluster")?;

    info!("connected to cluster");

    let kv_store: Arc<dyn aspen_traits::KeyValueStore> =
        Arc::new(client_kv::ClientKvAdapter::new(Arc::new(client.clone())));

    let (signing_key, public_key) =
        ensure_signing_key(&kv_store, &config.cache_name).await.context("failed to ensure signing key")?;

    info!(
        public_key = %public_key,
        "cache signing key ready — add to trusted-public-keys in nix.conf"
    );

    let state = Arc::new(GatewayState { client, signing_key });

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port).parse().context("invalid bind address")?;

    server::run(state, addr).await
}
