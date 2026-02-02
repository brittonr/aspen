//! Aspen CI Agent - Guest agent for Cloud Hypervisor CI VMs.
//!
//! This binary runs inside a microVM and receives execution requests
//! from the host via vsock. It executes commands in an isolated environment
//! and streams logs back to the host.
//!
//! ## Usage
//!
//! ```text
//! aspen-ci-agent [OPTIONS]
//!
//! Options:
//!     --vsock-port <PORT>  Vsock port to listen on [default: 5000]
//!     --log-level <LEVEL>  Log level (trace, debug, info, warn, error) [default: info]
//! ```
//!
//! ## Protocol
//!
//! Communication uses length-prefixed JSON frames over vsock:
//! - Host sends `HostMessage` (Execute, Cancel, Ping, Shutdown)
//! - Agent responds with `AgentMessage` (Log, Pong, Ready, Error)
//!
//! ## Workspace
//!
//! Commands are executed in a directory under `/workspace`, which is
//! a virtiofs mount point shared from the host. This allows the host
//! to prepare source code and collect artifacts.

// Allow unused code - this binary shares modules with the library
#![allow(dead_code)]

mod error;
mod executor;
mod protocol;
mod vsock_server;

use tracing::error;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::protocol::vsock::DEFAULT_PORT;
use crate::vsock_server::VsockServer;

/// Command line arguments.
struct Args {
    /// Vsock port to listen on.
    vsock_port: u32,
    /// Log level.
    log_level: String,
}

impl Args {
    fn parse() -> Self {
        let mut args = Args {
            vsock_port: DEFAULT_PORT,
            log_level: "info".to_string(),
        };

        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--vsock-port" => {
                    if let Some(val) = iter.next() {
                        args.vsock_port = val.parse().unwrap_or(DEFAULT_PORT);
                    }
                }
                "--log-level" => {
                    if let Some(val) = iter.next() {
                        args.log_level = val;
                    }
                }
                "--help" | "-h" => {
                    println!("aspen-ci-agent - Guest agent for Cloud Hypervisor CI VMs");
                    println!();
                    println!("USAGE:");
                    println!("    aspen-ci-agent [OPTIONS]");
                    println!();
                    println!("OPTIONS:");
                    println!("    --vsock-port <PORT>   Vsock port to listen on [default: 5000]");
                    println!("    --log-level <LEVEL>   Log level (trace, debug, info, warn, error) [default: info]");
                    println!("    -h, --help            Print help information");
                    std::process::exit(0);
                }
                other => {
                    eprintln!("Unknown argument: {}", other);
                    std::process::exit(1);
                }
            }
        }

        args
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logging
    let filter = EnvFilter::try_new(&args.log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), vsock_port = args.vsock_port, "starting aspen-ci-agent");

    // Create /workspace if it doesn't exist (for testing)
    if let Err(e) = std::fs::create_dir_all("/workspace") {
        error!("failed to create /workspace directory: {}", e);
        // Continue anyway - it might be read-only or already exist
    }

    // NOTE: Database dump loading now happens in executor.rs before each nix command.
    // This is because the dump file is written AFTER the VM boots and the job is assigned,
    // so it won't exist at startup time. See executor::load_nix_db_dump().

    // Run vsock server
    let server = VsockServer::new(args.vsock_port);

    if let Err(e) = server.run().await {
        error!("vsock server error: {}", e);
        std::process::exit(1);
    }
}

// Re-export protocol types for library use
pub use protocol::AgentMessage;
pub use protocol::ExecutionRequest;
pub use protocol::ExecutionResult;
pub use protocol::HostMessage;
pub use protocol::LogMessage;
pub use protocol::MAX_MESSAGE_SIZE;
