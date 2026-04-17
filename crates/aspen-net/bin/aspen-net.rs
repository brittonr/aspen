//! aspen-net: Service mesh daemon.
//!
//! Provides SOCKS5 proxy, DNS resolution, and service publishing
//! for accessing named Aspen cluster services.

use std::fmt::Display;
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

        /// Capability token (base64). Omit for permissive mode.
        #[arg(long, default_value = "")]
        token: String,

        /// SOCKS5 proxy bind address (ip:port or just port).
        #[arg(long, default_value = "127.0.0.1:1080")]
        socks5_addr: String,

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

struct UpCommandArgs {
    ticket: String,
    token: String,
    socks5_addr_text: String,
    dns_port: u16,
    should_enable_dns: bool,
    publish: Vec<String>,
    tags: Vec<String>,
}

fn exit_with_error(message: impl Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn init_tracing() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
}

fn build_runtime_or_exit() -> tokio::runtime::Runtime {
    match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => exit_with_error(format!("failed to create tokio runtime: {error}")),
    }
}

fn parse_socks5_addr_or_exit(socks5_addr_text: &str) -> SocketAddr {
    if let Ok(addr) = socks5_addr_text.parse() {
        return addr;
    }
    SocketAddr::from(([127, 0, 0, 1], match socks5_addr_text.parse() {
        Ok(port) => port,
        Err(_) => exit_with_error(format!("invalid SOCKS5 address: {socks5_addr_text}")),
    }))
}

fn dns_bind_addr(dns_port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], dns_port))
}

async fn handle_up_command(args: UpCommandArgs) {
    let socks5_addr = parse_socks5_addr_or_exit(&args.socks5_addr_text);
    let dns_addr = dns_bind_addr(args.dns_port);
    debug_assert!(!args.ticket.is_empty(), "daemon start requires a cluster ticket");
    debug_assert!(dns_addr.port() == args.dns_port, "dns bind helper preserves the requested port");
    let config = DaemonConfig {
        cluster_ticket: args.ticket,
        token: args.token,
        socks5_addr,
        dns_addr,
        dns_enabled: args.should_enable_dns,
        auto_publish: args.publish,
        tags: args.tags,
    };
    let mut daemon = match NetDaemon::start(config).await {
        Ok(daemon) => daemon,
        Err(error) => exit_with_error(format!("failed to start daemon: {error}")),
    };

    let cancel = daemon.cancel_token();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\nReceived Ctrl+C, shutting down...");
        }
        _ = cancel.cancelled() => {}
    }

    daemon.shutdown().await;
}

async fn handle_forward_command(spec: String) {
    match parse_forward_spec(&spec) {
        Ok((addr, service, remote_port)) => {
            eprintln!(
                "Forward {addr} -> {service}{}",
                remote_port.map(|remote_port_u16| format!(":{remote_port_u16}")).unwrap_or_default()
            );
            exit_with_error("Port forwarding requires a running daemon connection (not yet wired)");
        }
        Err(error) => exit_with_error(format!("invalid forward spec: {error}")),
    }
}

async fn run_command(command: Commands) {
    match command {
        Commands::Up {
            ticket,
            token,
            socks5_addr,
            dns_port,
            no_dns,
            publish,
            tags,
        } => {
            handle_up_command(UpCommandArgs {
                ticket,
                token,
                socks5_addr_text: socks5_addr,
                dns_port,
                should_enable_dns: !no_dns,
                publish,
                tags,
            })
            .await;
        }
        Commands::Down => exit_with_error("aspen net down: not yet implemented (requires daemon PID tracking)"),
        Commands::Forward {
            spec,
            ticket: _,
            token: _,
        } => handle_forward_command(spec).await,
        Commands::Status => exit_with_error("aspen net status: not yet implemented"),
    }
}

fn main() {
    init_tracing();
    let cli = Cli::parse();
    let runtime = build_runtime_or_exit();
    runtime.block_on(run_command(cli.command));
}
