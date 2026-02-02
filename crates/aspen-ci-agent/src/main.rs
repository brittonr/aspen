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

/// Load nix database dump from the workspace if present.
///
/// The host generates a database dump after prefetching the build closure.
/// This dump contains metadata for store paths shared via virtiofs - the
/// paths exist in /nix/store but the VM's nix-daemon doesn't know about them.
/// Loading this dump makes nix recognize these paths as valid substitutes.
async fn load_nix_db_dump() {
    use std::path::Path;
    use std::process::Stdio;
    use std::time::Instant;

    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    use tokio::process::Command;

    let dump_path = Path::new("/workspace/.nix-db-dump");

    if !dump_path.exists() {
        info!("no nix database dump found at {:?} - skipping", dump_path);
        return;
    }

    let start = Instant::now();

    // Read the dump file size for logging
    let dump_size = match tokio::fs::metadata(dump_path).await {
        Ok(meta) => meta.len(),
        Err(e) => {
            error!("failed to stat nix database dump: {}", e);
            return;
        }
    };

    info!(
        dump_path = %dump_path.display(),
        dump_size_bytes = dump_size,
        "loading nix database dump"
    );

    // Open the dump file for reading
    let dump_file = match File::open(dump_path).await {
        Ok(f) => f,
        Err(e) => {
            error!("failed to open nix database dump: {}", e);
            return;
        }
    };

    // Read the entire dump into memory (should be reasonable size)
    let mut dump_contents = Vec::new();
    let mut dump_reader = tokio::io::BufReader::new(dump_file);
    if let Err(e) = dump_reader.read_to_end(&mut dump_contents).await {
        error!("failed to read nix database dump: {}", e);
        return;
    }

    // Load the database using nix-store --load-db
    // This requires piping the dump to stdin
    let mut child = match Command::new("nix-store")
        .arg("--load-db")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            error!("failed to spawn nix-store --load-db: {}", e);
            return;
        }
    };

    // Write dump to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        if let Err(e) = stdin.write_all(&dump_contents).await {
            error!("failed to write to nix-store stdin: {}", e);
            return;
        }
        // Drop stdin to close it and signal EOF
    }

    // Wait for completion
    match child.wait().await {
        Ok(status) => {
            let elapsed = start.elapsed();
            if status.success() {
                info!(
                    dump_size_bytes = dump_size,
                    elapsed_ms = elapsed.as_millis(),
                    "nix database dump loaded successfully"
                );
            } else {
                error!(exit_code = status.code(), elapsed_ms = elapsed.as_millis(), "nix-store --load-db failed");
            }
        }
        Err(e) => {
            error!("failed to wait for nix-store --load-db: {}", e);
        }
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

    // Load nix database dump if present.
    // The host generates this file with `nix-store --dump-db` after prefetching
    // the build closure. Loading it makes the VM's nix-daemon aware of store paths
    // that are shared via virtiofs (the paths exist but lack database entries).
    load_nix_db_dump().await;

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
