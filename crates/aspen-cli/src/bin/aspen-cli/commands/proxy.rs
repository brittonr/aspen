//! HTTP proxy commands for TCP tunneling through remote Aspen nodes.
//!
//! Starts a local TCP listener that tunnels connections through a remote Aspen
//! node's upstream proxy over iroh QUIC. Useful for accessing services behind
//! a cluster member's network.
//!
//! # Examples
//!
//! ```bash
//! # Start a TCP tunnel to a specific service through a remote node
//! aspen-cli proxy start --target 127.0.0.1:8080 --local-addr 127.0.0.1:9090
//!
//! # Start an HTTP forward proxy through a remote node
//! aspen-cli proxy forward --local-addr 127.0.0.1:9090
//! ```

use anyhow::Context;
use anyhow::Result;
use aspen_cluster::ticket::parse_ticket_to_addrs;
use aspen_proxy::Authority;
use aspen_proxy::DownstreamProxy;
use aspen_proxy::EndpointAuthority;
use aspen_proxy::HttpProxyOpts;
use aspen_proxy::PoolOpts;
use aspen_proxy::ProxyMode;
use aspen_proxy::StaticForwardProxy;
use clap::Args;
use clap::Subcommand;
use iroh::Endpoint;
use iroh::discovery::static_provider::StaticProvider;
use tracing::debug;
use tracing::info;

/// HTTP proxy operations for TCP tunneling over iroh.
#[derive(Subcommand)]
pub enum ProxyCommand {
    /// Start a TCP tunnel to a fixed target through a remote node.
    ///
    /// Opens a local TCP listener and tunnels all connections through the
    /// remote Aspen node to the specified target address.
    Start(StartArgs),

    /// Start an HTTP forward proxy through a remote node.
    ///
    /// Opens a local TCP listener that acts as an HTTP forward proxy.
    /// Clients specify destinations via standard HTTP proxy mechanisms
    /// (CONNECT for tunneling, absolute-form URIs for plain HTTP).
    Forward(ForwardArgs),
}

/// Arguments for the TCP tunnel command.
#[derive(Args)]
pub struct StartArgs {
    /// Local bind address for the proxy listener.
    ///
    /// Format: "host:port" (e.g., "127.0.0.1:9090")
    /// Use port 0 for automatic port assignment.
    #[arg(long, default_value = "127.0.0.1:0")]
    pub local_addr: String,

    /// Target address to tunnel to (as seen by the remote node).
    ///
    /// Format: "host:port" (e.g., "127.0.0.1:8080", "api.internal:443")
    #[arg(long)]
    pub target: String,
}

/// Arguments for the HTTP forward proxy command.
#[derive(Args)]
pub struct ForwardArgs {
    /// Local bind address for the proxy listener.
    ///
    /// Format: "host:port" (e.g., "127.0.0.1:8080")
    /// Use port 0 for automatic port assignment.
    #[arg(long, default_value = "127.0.0.1:0")]
    pub local_addr: String,
}

/// Maximum time to wait for initial connection to remote node.
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// Idle connection timeout before cleanup.
const IDLE_TIMEOUT_SECS: u64 = 30;

impl ProxyCommand {
    /// Execute the proxy command.
    ///
    /// This does NOT use the standard AspenClient because the proxy needs
    /// direct access to the iroh Endpoint for QUIC stream tunneling
    /// (DownstreamProxy uses a different ALPN than the client RPC).
    pub async fn run(self, ticket: &str, _is_json: bool) -> Result<()> {
        // Parse the cluster ticket to get peer addresses
        let (_topic_id, _cluster_id, bootstrap_addrs) =
            parse_ticket_to_addrs(ticket).context("failed to parse cluster ticket")?;

        // Tiger Style: Fail fast if no peers are available.
        if bootstrap_addrs.is_empty() {
            anyhow::bail!("ticket contains no bootstrap peers");
        }

        let bootstrap_addr = bootstrap_addrs.into_iter().next().expect("checked non-empty");
        let remote_id = bootstrap_addr.id;

        info!(
            remote = %remote_id.fmt_short(),
            "connecting to remote node for proxy tunneling"
        );

        // Create a StaticProvider with the remote node's address so the
        // connection pool inside DownstreamProxy can resolve EndpointId → address.
        let discovery = StaticProvider::new();
        discovery.add_endpoint_info(bootstrap_addr);

        // Create a lightweight iroh endpoint for the proxy client.
        // No ALPNs needed — DownstreamProxy uses HTTP_PROXY_ALPN internally.
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery(discovery)
            .bind()
            .await
            .context("failed to create iroh endpoint")?;

        debug!(
            endpoint_id = %endpoint.id(),
            remote = %remote_id.fmt_short(),
            "iroh endpoint created for proxy client"
        );

        let pool_opts = PoolOpts {
            connect_timeout: std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS),
            idle_timeout: std::time::Duration::from_secs(IDLE_TIMEOUT_SECS),
        };
        let proxy = DownstreamProxy::new(endpoint, pool_opts);

        match self {
            ProxyCommand::Start(args) => run_tcp_tunnel(proxy, remote_id, args).await,
            ProxyCommand::Forward(args) => run_forward_proxy(proxy, remote_id, args).await,
        }
    }
}

/// Run a TCP tunnel to a fixed target through the remote node.
async fn run_tcp_tunnel(proxy: DownstreamProxy, remote_id: iroh::EndpointId, args: StartArgs) -> Result<()> {
    let target: Authority = args
        .target
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid target address (expected host:port): {e}"))?;

    let destination = EndpointAuthority::new(remote_id, target.clone());

    let listener = tokio::net::TcpListener::bind(&args.local_addr).await.context("failed to bind local listener")?;
    let local_addr = listener.local_addr()?;

    eprintln!("TCP tunnel listening on {local_addr}");
    eprintln!("  Tunneling to {target} via {}", remote_id.fmt_short());
    eprintln!("  Press Ctrl+C to stop");

    let mode = ProxyMode::Tcp(destination);
    proxy.forward_tcp_listener(listener, mode).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}

/// Run an HTTP forward proxy through the remote node.
async fn run_forward_proxy(proxy: DownstreamProxy, remote_id: iroh::EndpointId, args: ForwardArgs) -> Result<()> {
    let handler = StaticForwardProxy(remote_id);

    let listener = tokio::net::TcpListener::bind(&args.local_addr).await.context("failed to bind local listener")?;
    let local_addr = listener.local_addr()?;

    eprintln!("HTTP forward proxy listening on {local_addr}");
    eprintln!("  Proxying through {}", remote_id.fmt_short());
    eprintln!("  Configure your client: HTTP_PROXY=http://{local_addr}");
    eprintln!("  Press Ctrl+C to stop");

    let mode = ProxyMode::Http(HttpProxyOpts::new(handler));
    proxy.forward_tcp_listener(listener, mode).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
