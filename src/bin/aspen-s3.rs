/// Aspen S3 server binary.
///
/// Provides an S3-compatible API server backed by Aspen's distributed
/// key-value store.
use anyhow::{Context, Result};
use aspen::api::{DeterministicKeyValueStore, KeyValueStore};
use aspen::s3::AspenS3Service;
use clap::Parser;
use s3s::service::S3ServiceBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

/// Aspen S3 server - S3-compatible API for Aspen.
#[derive(Parser, Debug)]
#[command(name = "aspen-s3")]
#[command(about = "S3-compatible API server for Aspen distributed storage")]
struct Args {
    /// Node ID (must be unique in cluster).
    #[arg(long, default_value = "1", env = "ASPEN_NODE_ID")]
    node_id: u64,

    /// HTTP address to bind for S3 API.
    #[arg(long, default_value = "127.0.0.1:9000", env = "ASPEN_S3_ADDR")]
    s3_addr: String,

    /// Enable path-style bucket addressing (default: virtual-host style).
    #[arg(long, env = "ASPEN_S3_PATH_STYLE")]
    path_style: bool,

    /// Log level.
    #[arg(long, default_value = "info", env = "ASPEN_LOG_LEVEL")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level)),
        )
        .init();

    info!(
        "Starting Aspen S3 server (node_id={}, s3_addr={})",
        args.node_id, args.s3_addr
    );

    // For now, use a simple in-memory KV store for testing
    // TODO: Replace with actual Raft-backed KV store once cluster bootstrap is working
    let kv_store: Arc<dyn KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
    let s3_service = AspenS3Service::new(kv_store, args.node_id);

    // Build S3 HTTP service using s3s
    info!("Building S3 HTTP service...");
    let _s3_http_service = S3ServiceBuilder::new(s3_service).build();

    // Parse S3 address
    let s3_addr: SocketAddr = args.s3_addr.parse().context("Invalid S3 address format")?;

    info!("S3 server would listen on http://{}", s3_addr);
    info!("Note: S3 server HTTP loop not yet implemented");
    info!("The S3 service skeleton is in place and compiles successfully");

    // TODO: Implement HTTP server loop with s3s service
    // This requires integrating with hyper/tower properly
    // For now, we have verified the S3 service skeleton compiles

    Ok(())
}
