//! VirtioFS daemon simulating a CI workspace backed by Raft cluster.
//!
//! Replicates the exact data path from `aspen-ci-executor-vm`:
//!   1. Connects to Raft cluster via ticket
//!   2. Creates `AspenFs::with_prefix()` for namespace isolation
//!   3. Seeds workspace with cluster ticket and a test script
//!   4. Serves VirtioFS for the workspace mount
//!
//! The guest VM can then:
//!   - Read the test script from `/workspace/`
//!   - Execute it (creating build artifacts)
//!   - Write artifacts back through VirtioFS to the Raft KV store
//!
//! The host verifies artifacts by reading the KV store directly.
//!
//! This exercises the same code path as `CloudHypervisorWorker::start()`
//! in `aspen-ci-executor-vm/src/vm/lifecycle.rs`.
//!
//! # Usage
//!
//! ```bash
//! aspen-ci-workspace-server \
//!     --socket /tmp/workspace.sock \
//!     --ticket <cluster-ticket> \
//!     --prefix ci/workspaces/test-vm/
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use aspen_fuse::AspenFs;
use aspen_fuse::FuseSyncClient;
use clap::Parser;
use tracing::error;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "aspen-ci-workspace-server")]
#[command(about = "CI workspace VirtioFS daemon backed by Aspen Raft cluster")]
struct Args {
    /// VHost-user socket path.
    #[arg(long)]
    socket: PathBuf,

    /// Cluster ticket for Aspen connection.
    #[arg(long)]
    ticket: String,

    /// KV key prefix for workspace namespace isolation.
    /// Matches the per-VM prefix used by CloudHypervisorWorker.
    #[arg(long, default_value = "ci/workspaces/test-vm/")]
    prefix: String,
}

/// Seed the workspace with a test script and data files.
///
/// Writes directly to the KV store (with prefix) to simulate what
/// `ManagedCiVm::start()` does when provisioning a workspace.
fn seed_workspace(client: &FuseSyncClient, prefix: &str) -> anyhow::Result<()> {
    info!(prefix = %prefix, "seeding workspace");

    // Write a test build script that the guest will execute.
    // This mimics a CI job's build.sh.
    let build_script = b"#!/bin/sh
set -e
echo 'CI build starting...'

# Create build artifacts in the workspace
echo 'artifact-data-from-ci-build' > /workspace/build-output.txt
echo '{\"status\":\"success\",\"exit_code\":0}' > /workspace/result.json

# Create a subdirectory with nested artifacts
mkdir -p /workspace/artifacts
echo 'log line 1' > /workspace/artifacts/build.log
echo 'log line 2' >> /workspace/artifacts/build.log

echo 'CI build complete'
";
    let key = format!("{}{}", prefix, "build.sh");
    client.write_key(&key, build_script)?;
    info!("wrote build.sh to workspace");

    // Write test input data (simulates source checkout)
    let key = format!("{}{}", prefix, "input.txt");
    client.write_key(&key, b"test input for CI job")?;
    info!("wrote input.txt to workspace");

    // Verify reads work through the prefixed path
    let key = format!("{}{}", prefix, "build.sh");
    let value = client.read_key(&key)?;
    assert!(value.is_some(), "build.sh not found after write");
    info!("verified workspace files readable");

    Ok(())
}

fn main() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();

    let args = Args::parse();

    info!(ticket_len = args.ticket.len(), prefix = %args.prefix, "connecting to Aspen cluster");

    // Connect to the Raft cluster
    let client = match FuseSyncClient::from_ticket(&args.ticket) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!(error = %e, "failed to connect to Aspen cluster");
            std::process::exit(1);
        }
    };
    info!("connected to Aspen cluster");

    // Initialize the cluster (idempotent)
    info!("initializing cluster");
    match client.init_cluster() {
        Ok(true) => info!("cluster initialized successfully"),
        Ok(false) => {
            error!("cluster initialization returned false");
            std::process::exit(1);
        }
        Err(e) => {
            error!(error = %e, "failed to initialize cluster");
            std::process::exit(1);
        }
    }

    // Seed workspace with test data
    if let Err(e) = seed_workspace(&client, &args.prefix) {
        error!(error = %e, "failed to seed workspace");
        std::process::exit(1);
    }
    info!("workspace seeded");

    // Get current user info
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    // Create filesystem with per-workspace prefix (same as CI executor)
    let fs = AspenFs::with_prefix(uid, gid, client, args.prefix.clone());

    info!(
        socket = %args.socket.display(),
        prefix = %args.prefix,
        "starting VirtioFS daemon for CI workspace"
    );

    // Run VirtioFS daemon — blocks until shutdown
    if let Err(e) = aspen_fuse::run_virtiofs_daemon(&args.socket, fs) {
        error!(error = %e, "VirtioFS daemon failed");
        std::process::exit(1);
    }

    info!("VirtioFS daemon stopped");
}
