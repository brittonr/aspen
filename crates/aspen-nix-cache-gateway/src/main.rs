//! Nix binary cache gateway for Aspen.
//!
//! A lightweight HTTP server that translates Nix binary cache protocol
//! requests into Aspen cluster operations using nar-bridge's axum router
//! with Aspen's `BlobService`/`DirectoryService`/`PathInfoService` trait impls.
//! Supports GET, HEAD, PUT, range requests, and compression.
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

    #[cfg(not(feature = "snix-http"))]
    {
        let _ = config;
        anyhow::bail!("snix-http feature required — build with 'snix-http' feature")
    }
}
