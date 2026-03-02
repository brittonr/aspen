//! Minimal VirtioFS test server backed by in-memory AspenFs.
//!
//! Pre-seeds a KV store with test content, then runs a vhost-user-fs daemon.
//! Used by NixOS VM integration tests to verify the full data path:
//!   AspenFs KV → VirtioFS → cloud-hypervisor → guest mount → nginx
//!
//! Usage:
//!   aspen-virtiofs-test-server --socket /tmp/virtiofs.sock

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use aspen_fuse::AspenFs;
use aspen_fuse::spawn_virtiofs_daemon;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let socket_path = match args.iter().position(|a| a == "--socket") {
        Some(i) if i + 1 < args.len() => PathBuf::from(&args[i + 1]),
        _ => {
            eprintln!("Usage: aspen-virtiofs-test-server --socket <path>");
            std::process::exit(1);
        }
    };

    // Pre-seed the in-memory KV store with content for nginx to serve
    let store: Arc<RwLock<BTreeMap<String, Vec<u8>>>> = Arc::new(RwLock::new(BTreeMap::new()));

    {
        let mut s = store.write().expect("lock poisoned");
        s.insert("index.html".to_string(), b"hello from aspen kv\n".to_vec());
        s.insert("status.json".to_string(), br#"{"source":"aspen-kv","ok":true}"#.to_vec());
    }

    tracing::info!(
        keys = store.read().unwrap().len(),
        socket = %socket_path.display(),
        "starting AspenFs VirtioFS daemon with pre-seeded KV store"
    );

    // Build AspenFs with the pre-seeded store
    let fs = AspenFs::from_shared_store(0, 0, Arc::clone(&store));

    // Spawn the VirtioFS daemon — it blocks in accept() until CH connects,
    // then serves requests until CH disconnects or we shut down.
    let handle = spawn_virtiofs_daemon(&socket_path, fs).expect("failed to spawn VirtioFS daemon");

    tracing::info!("VirtioFS daemon running, waiting for shutdown signal");

    // Block until SIGTERM (systemd stop) or SIGINT (Ctrl-C)
    // On signal, shut down the daemon cleanly.
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .expect("failed to set signal handler");
    let _ = rx.recv(); // blocks forever until signal

    tracing::info!("signal received, shutting down");
    handle.shutdown().expect("daemon shutdown failed");
}
