//! aspen-net: Service mesh daemon.
//!
//! Provides SOCKS5 proxy, DNS resolution, and service publishing
//! for accessing named Aspen cluster services.

use std::net::SocketAddr;

use aspen_net::daemon::DaemonConfig;
use aspen_net::daemon::NetDaemon;
use aspen_net::forward::parse_forward_spec;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(name = "aspen-net", about = "Aspen service mesh daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the service mesh daemon.
    Up {
        /// Cluster ticket for connecting.
        #[arg(long)]
        ticket: String,

        /// Capability token (base64).
        #[arg(long)]
        token: String,

        /// SOCKS5 proxy port.
        #[arg(long, default_value = "1080")]
        socks5_port: u16,

        /// DNS resolver port.
        #[arg(long, default_value = "5353")]
        dns_port: u16,

        /// Disable DNS resolver.
        #[arg(long)]
        no_dns: bool,

        /// Services to auto-publish (format: name:port[:proto]).
        #[arg(long = "publish")]
        publish: Vec<String>,

        /// Tags for auto-published services.
        #[arg(long = "tag")]
        tags: Vec<String>,
    },

    /// Stop the running daemon.
    Down,

    /// Forward a local port to a remote service.
    Forward {
        /// Forward spec: [bind_addr:]local_port:service[:remote_port]
        spec: String,

        /// Cluster ticket.
        #[arg(long)]
        ticket: String,

        /// Capability token (base64).
        #[arg(long)]
        token: String,
    },

    /// Show daemon status.
    Status,
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let cli = Cli::parse();

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap_or_else(|e| {
        eprintln!("failed to create tokio runtime: {e}");
        std::process::exit(1);
    });

    rt.block_on(async {
        match cli.command {
            Commands::Up {
                ticket,
                token,
                socks5_port,
                dns_port,
                no_dns,
                publish,
                tags,
            } => {
                let config = DaemonConfig {
                    cluster_ticket: ticket,
                    token,
                    socks5_addr: SocketAddr::from(([127, 0, 0, 1], socks5_port)),
                    dns_addr: SocketAddr::from(([127, 0, 0, 1], dns_port)),
                    dns_enabled: !no_dns,
                    auto_publish: publish,
                    tags,
                };

                // Start daemon
                let mut daemon = match NetDaemon::start(config).await {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("failed to start daemon: {e}");
                        std::process::exit(1);
                    }
                };

                // Wait for shutdown signal
                let cancel = daemon.cancel_token();
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        eprintln!("\nReceived Ctrl+C, shutting down...");
                    }
                    _ = cancel.cancelled() => {}
                }

                daemon.shutdown().await;
            }

            Commands::Down => {
                eprintln!("aspen net down: not yet implemented (requires daemon PID tracking)");
                std::process::exit(1);
            }

            Commands::Forward {
                spec,
                ticket: _,
                token: _,
            } => match parse_forward_spec(&spec) {
                Ok((addr, service, remote_port)) => {
                    eprintln!(
                        "Forward {addr} -> {service}{}",
                        remote_port.map(|p| format!(":{p}")).unwrap_or_default()
                    );
                    eprintln!("Port forwarding requires a running daemon connection (not yet wired)");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("invalid forward spec: {e}");
                    std::process::exit(1);
                }
            },

            Commands::Status => {
                eprintln!("aspen net status: not yet implemented");
                std::process::exit(1);
            }
        }
    });
}
