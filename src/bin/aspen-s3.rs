/// Aspen S3 server binary.
///
/// Provides an S3-compatible API server backed by Aspen's distributed
/// key-value store.
use anyhow::{Context, Result};
use aspen::api::{DeterministicKeyValueStore, KeyValueStore};
use aspen::s3::AspenS3Service;
use clap::Parser;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use s3s::auth::{SecretKey, SimpleAuth};
use s3s::service::S3ServiceBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

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

    /// Base domain for virtual-host style bucket addressing.
    /// When set, buckets are addressed as `bucket.domain`.
    /// When not set, path-style addressing is used (`domain/bucket`).
    #[arg(long, env = "ASPEN_S3_DOMAIN")]
    domain: Option<String>,

    /// Log level.
    #[arg(long, default_value = "info", env = "ASPEN_LOG_LEVEL")]
    log_level: String,

    /// S3 access key for authentication.
    #[arg(long, default_value = "test", env = "ASPEN_S3_ACCESS_KEY")]
    access_key: String,

    /// S3 secret key for authentication.
    #[arg(long, default_value = "test", env = "ASPEN_S3_SECRET_KEY")]
    secret_key: String,
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
    let mut builder = S3ServiceBuilder::new(s3_service);

    // Configure authentication with static credentials
    info!("Configuring S3 authentication (access_key={})", args.access_key);
    let secret_key = SecretKey::from(args.secret_key.as_str());
    let auth = SimpleAuth::from_single(args.access_key.clone(), secret_key);
    builder.set_auth(auth);

    // Configure virtual-host style if domain is provided
    if let Some(ref domain) = args.domain {
        info!("Configuring virtual-host style with domain: {}", domain);
        let host_parser = s3s::host::MultiDomain::new([domain.as_str()])
            .context("Invalid domain configuration")?;
        builder.set_host(host_parser);
    } else {
        info!("Using path-style bucket addressing (no domain configured)");
    }

    let s3_http_service = builder.build().into_shared();

    // Parse S3 address
    let s3_addr: SocketAddr = args.s3_addr.parse().context("Invalid S3 address format")?;

    // Bind TCP listener
    let listener = TcpListener::bind(s3_addr)
        .await
        .context("Failed to bind S3 server address")?;

    info!("S3 server listening on http://{}", s3_addr);
    info!("Ready to accept connections");

    // Accept and serve connections
    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };

        let service = s3_http_service.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);

            let result = http1::Builder::new().serve_connection(io, service).await;

            if let Err(e) = result {
                // Don't log normal connection closures
                if !e.is_incomplete_message() {
                    error!("Connection error from {}: {}", peer_addr, e);
                }
            }
        });
    }
}
