//! TCP-to-iroh-h3 reverse proxy binary.
//!
//! Bridges HTTP/1.1 clients to iroh endpoints serving HTTP/3 over QUIC.
//!
//! ```text
//! aspen-h3-proxy --endpoint-id <ID> --alpn "aspen/forge-web/1" --port 8080
//! ```

use std::time::Duration;

use anyhow::Context;
use aspen_h3_proxy::H3Proxy;
use aspen_h3_proxy::ProxyConfig;
use clap::Parser;
use iroh::EndpointAddr;
use iroh::PublicKey;

/// TCP-to-iroh-h3 reverse proxy.
///
/// Listens on a local TCP port and forwards HTTP/1.1 requests
/// to an iroh endpoint over HTTP/3 QUIC.
#[derive(Parser)]
#[command(name = "aspen-h3-proxy")]
#[command(about = "Bridge HTTP/1.1 clients to iroh h3 endpoints")]
struct Cli {
    /// Iroh endpoint ID (public key) of the target server.
    #[arg(long)]
    endpoint_id: String,

    /// ALPN protocol identifier (e.g., "aspen/forge-web/1" or "iroh+h3").
    #[arg(long)]
    alpn: String,

    /// TCP port to listen on.
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// TCP bind address.
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Per-request timeout in seconds.
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

    let endpoint_id: PublicKey = cli.endpoint_id.parse().context("invalid endpoint ID (expected base32 public key)")?;

    let config = ProxyConfig {
        bind_addr: cli.bind,
        port: cli.port,
        target_addr: EndpointAddr::from(endpoint_id),
        alpn: cli.alpn.as_bytes().to_vec(),
        request_timeout: Duration::from_secs(cli.timeout_secs),
    };

    let proxy = H3Proxy::new(config);
    proxy.run().await
}
