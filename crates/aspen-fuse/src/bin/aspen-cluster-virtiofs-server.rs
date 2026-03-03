//! VirtioFS daemon backed by a real Aspen Raft cluster.
//!
//! Connects to an Aspen cluster via ticket, pre-seeds test content
//! into the KV store, then serves it as a VirtioFS filesystem.
//!
//! Used in integration tests to verify the full data path:
//!   Raft cluster → iroh QUIC → AspenFs → VirtioFS → Cloud Hypervisor → guest mount
//!
//! # Usage
//!
//! ```bash
//! aspen-cluster-virtiofs-server --socket /tmp/aspenfs.sock --ticket <cluster-ticket>
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
#[command(name = "aspen-cluster-virtiofs-server")]
#[command(about = "VirtioFS daemon backed by Aspen Raft cluster (test server)")]
struct Args {
    /// VHost-user socket path.
    #[arg(long)]
    socket: PathBuf,

    /// Cluster ticket for Aspen connection.
    #[arg(long)]
    ticket: String,
}

/// Pre-seed test content into the cluster's KV store.
///
/// These entries appear as files through the VirtioFS mount:
///   /index.html  → "hello from aspen raft cluster\n"
///   /status.json → {"source":"aspen-raft","nodes":3}
fn seed_test_data(client: &FuseSyncClient) -> anyhow::Result<()> {
    info!("seeding test data into Raft cluster");

    client.write_key("index.html", b"<html><body>hello from aspen raft cluster</body></html>\n")?;
    info!("wrote index.html to cluster");

    client.write_key("status.json", br#"{"source":"aspen-raft","nodes":3,"ok":true}"#)?;
    info!("wrote status.json to cluster");

    // Verify reads work
    let value = client.read_key("index.html")?;
    assert!(value.is_some(), "index.html not found after write");
    info!("verified index.html readable from cluster");

    Ok(())
}

fn main() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();

    let args = Args::parse();

    info!(ticket = %args.ticket, "connecting to Aspen cluster");

    // Connect to the Raft cluster
    let client = match FuseSyncClient::from_ticket(&args.ticket) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!(error = %e, "failed to connect to Aspen cluster");
            std::process::exit(1);
        }
    };

    info!("connected to Aspen cluster");

    // Initialize the cluster (required before KV operations work).
    // This is idempotent — safe to call on an already-initialized cluster.
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

    // Pre-seed test data
    if let Err(e) = seed_test_data(&client) {
        error!(error = %e, "failed to seed test data");
        std::process::exit(1);
    }

    info!("test data seeded into Raft cluster");

    // Get current user info
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    // Create filesystem backed by the live Raft cluster
    let fs = AspenFs::new(uid, gid, client);

    info!(
        socket = %args.socket.display(),
        "starting VirtioFS daemon (backed by Raft cluster)"
    );

    // Run VirtioFS daemon — blocks until shutdown
    if let Err(e) = aspen_fuse::run_virtiofs_daemon(&args.socket, fs) {
        error!(error = %e, "VirtioFS daemon failed");
        std::process::exit(1);
    }

    info!("VirtioFS daemon stopped");
}
