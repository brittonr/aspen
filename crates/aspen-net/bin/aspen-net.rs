//! aspen-net: Service mesh daemon.
//!
//! Provides SOCKS5 proxy, DNS resolution, and service publishing
//! for accessing named Aspen cluster services.

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

        /// Services to auto-publish (format: name:port:proto).
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Up { .. } => {
            eprintln!("aspen net up: not yet implemented (daemon requires cluster connection)");
            std::process::exit(1);
        }
        Commands::Down => {
            eprintln!("aspen net down: not yet implemented");
            std::process::exit(1);
        }
        Commands::Forward { spec, .. } => {
            eprintln!("aspen net forward {spec}: not yet implemented");
            std::process::exit(1);
        }
        Commands::Status => {
            eprintln!("aspen net status: not yet implemented");
            std::process::exit(1);
        }
    }
}
