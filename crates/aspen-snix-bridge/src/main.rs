//! gRPC bridge for snix-store CLI ↔ Aspen storage.
//!
//! Runs a local gRPC server on a Unix socket that translates snix's
//! BlobService/DirectoryService/PathInfoService gRPC protocol to Aspen-backed
//! implementations.
//!
//! Two modes:
//!
//! **Standalone (in-memory)** — for local testing without a cluster:
//! ```bash
//! aspen-snix-bridge --socket /tmp/aspen-castore.sock
//! ```
//!
//! **Cluster-connected** — connects to a live Aspen cluster via ticket:
//! ```bash
//! aspen-snix-bridge --socket /tmp/aspen-castore.sock --ticket <cluster-ticket>
//! ```
//!
//! Then point snix-store at the bridge:
//! ```bash
//! BLOB_SERVICE_ADDR=grpc+unix:/tmp/aspen-castore.sock \
//! DIRECTORY_SERVICE_ADDR=grpc+unix:/tmp/aspen-castore.sock \
//! PATH_INFO_SERVICE_ADDR=grpc+unix:/tmp/aspen-castore.sock \
//!   snix-store virtiofs /tmp/snix.sock
//! ```

mod client_kv;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aspen_snix::IrohBlobService;
use aspen_snix::RaftDirectoryService;
use aspen_snix::RaftPathInfoService;
use clap::Parser;
use snix_castore::proto::GRPCBlobServiceWrapper;
use snix_castore::proto::GRPCDirectoryServiceWrapper;
use snix_castore::proto::blob_service_server::BlobServiceServer;
use snix_castore::proto::directory_service_server::DirectoryServiceServer;
use snix_store::nar::SimpleRenderer;
use snix_store::proto::GRPCPathInfoServiceWrapper;
use snix_store::proto::path_info_service_server::PathInfoServiceServer;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::info;

#[derive(Parser)]
#[command(name = "aspen-snix-bridge")]
#[command(about = "gRPC bridge: snix-store ↔ Aspen distributed storage")]
struct Args {
    /// Unix socket path for gRPC server.
    #[arg(long, default_value = "/tmp/aspen-castore.sock")]
    socket: PathBuf,

    /// Aspen cluster ticket for connecting to a live cluster.
    /// When omitted, uses in-memory backends for standalone testing.
    #[arg(long)]
    ticket: Option<String>,

    /// RPC timeout in seconds for cluster communication.
    #[arg(long, default_value_t = 10)]
    timeout_secs: u64,

    /// Unix socket path for nix-daemon protocol.
    /// Allows standard `nix` CLI to use Aspen as its store:
    ///   nix path-info --store unix:///tmp/aspen-nix-daemon.sock /nix/store/...
    #[cfg(feature = "snix-daemon")]
    #[arg(long)]
    daemon_socket: Option<PathBuf>,

    /// Enable HTTP castore browser for debugging store contents.
    #[cfg(feature = "snix-http")]
    #[arg(long)]
    browse: bool,

    /// Port for the HTTP castore browser.
    #[cfg(feature = "snix-http")]
    #[arg(long, default_value_t = 9000)]
    browse_port: u16,

    /// Tracing configuration (verbosity, OTLP export, tracers).
    #[clap(flatten)]
    tracing: snix_tracing::TracingArgs,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Use snix-tracing for unified tracing setup (supports OTLP via --tracer otlp
    // and OTEL_EXPORTER_OTLP_ENDPOINT env var)
    let _tracing_handle = snix_tracing::TracingBuilder::default().handle_tracing_args(&args.tracing).build().ok();

    // Remove stale socket
    if args.socket.exists() {
        tokio::fs::remove_file(&args.socket).await?;
    }
    if let Some(parent) = args.socket.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Build snix services — either cluster-backed or in-memory
    use snix_castore::blobservice::BlobService;
    use snix_castore::directoryservice::DirectoryService;
    use snix_store::pathinfoservice::PathInfoService;

    let (blob_svc, dir_svc, pathinfo_svc): (Arc<dyn BlobService>, Arc<dyn DirectoryService>, Arc<dyn PathInfoService>) =
        if let Some(ticket) = &args.ticket {
            build_cluster_services(ticket, args.timeout_secs).await?
        } else {
            info!("no --ticket provided, using in-memory backends");
            build_inmemory_services()
        };

    // Shutdown coordination — a single Ctrl-C shuts down both gRPC and daemon
    let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);

    // Start nix-daemon listener if configured (shares same service instances)
    #[cfg(feature = "snix-daemon")]
    let daemon_handle = if let Some(ref daemon_socket) = args.daemon_socket {
        let daemon_blob = blob_svc.clone();
        let daemon_dir = dir_svc.clone();
        let daemon_pathinfo = pathinfo_svc.clone();
        let daemon_path = daemon_socket.clone();
        let daemon_shutdown = _shutdown_rx.clone();
        Some(tokio::spawn(async move {
            if let Err(e) = aspen_snix_bridge::daemon::serve_daemon(
                &daemon_path,
                daemon_blob,
                daemon_dir,
                daemon_pathinfo,
                daemon_shutdown,
            )
            .await
            {
                tracing::error!(error = %e, "nix-daemon listener failed");
            }
        }))
    } else {
        None
    };

    // Create NAR calculation service (renders NARs from blob+dir data)
    let nar_calc = Box::new(SimpleRenderer::new(blob_svc.clone(), dir_svc.clone()));

    // Wrap in snix's gRPC server wrappers
    let blob_grpc = GRPCBlobServiceWrapper::new(blob_svc);
    let dir_grpc = GRPCDirectoryServiceWrapper::new(dir_svc);
    let pathinfo_grpc = GRPCPathInfoServiceWrapper::new(pathinfo_svc, nar_calc);

    // Bind Unix socket
    let listener = UnixListener::bind(&args.socket)?;
    let incoming = UnixListenerStream::new(listener);

    let (_health_reporter, health_service) = tonic_health::server::health_reporter();

    Server::builder()
        .add_service(health_service)
        .add_service(BlobServiceServer::new(blob_grpc))
        .add_service(DirectoryServiceServer::new(dir_grpc))
        .add_service(PathInfoServiceServer::new(pathinfo_grpc))
        .serve_with_incoming_shutdown(incoming, async {
            tokio::signal::ctrl_c().await.ok();
            info!("shutting down gRPC bridge");
            let _ = shutdown_tx.send(true);
        })
        .await?;

    // Wait for daemon to finish if it was started
    #[cfg(feature = "snix-daemon")]
    if let Some(handle) = daemon_handle {
        let _ = handle.await;
    }

    // Clean up socket
    let _ = tokio::fs::remove_file(&args.socket).await;

    Ok(())
}

/// Build snix services backed by a live Aspen cluster via ticket.
///
/// - BlobService: `RpcBlobStore` → `IrohBlobService` (blob ops via cluster RPC)
/// - DirectoryService: `ClientKvAdapter` → `RaftDirectoryService` (KV via cluster RPC)
/// - PathInfoService: `ClientKvAdapter` → `RaftPathInfoService` (KV via cluster RPC)
async fn build_cluster_services(
    ticket: &str,
    timeout_secs: u64,
) -> Result<
    (
        Arc<dyn snix_castore::blobservice::BlobService>,
        Arc<dyn snix_castore::directoryservice::DirectoryService>,
        Arc<dyn snix_store::pathinfoservice::PathInfoService>,
    ),
    Box<dyn std::error::Error>,
> {
    let timeout = Duration::from_secs(timeout_secs);

    let client = aspen_client::AspenClient::connect(ticket, timeout, None).await?;

    info!("connected to Aspen cluster");

    // Clone shares the iroh endpoint — one UDP socket, one relay connection
    let blob_client = client.clone();
    let kv_client = client;

    // BlobService: RpcBlobStore wraps AspenClient for remote blob operations
    let rpc_blob_store = aspen_client::RpcBlobStore::new(blob_client);
    let blob_svc: Arc<dyn snix_castore::blobservice::BlobService> = Arc::new(IrohBlobService::new(rpc_blob_store));

    // DirectoryService + PathInfoService: ClientKvAdapter wraps AspenClient for KV
    let kv = Arc::new(client_kv::ClientKvAdapter::new(Arc::new(kv_client))) as Arc<dyn aspen_core::KeyValueStore>;
    let dir_svc: Arc<dyn snix_castore::directoryservice::DirectoryService> =
        Arc::new(RaftDirectoryService::from_arc(kv.clone()));
    let pathinfo_svc: Arc<dyn snix_store::pathinfoservice::PathInfoService> =
        Arc::new(RaftPathInfoService::from_arc(kv));

    Ok((blob_svc, dir_svc, pathinfo_svc))
}

/// Build snix services with in-memory backends for standalone testing.
fn build_inmemory_services() -> (
    Arc<dyn snix_castore::blobservice::BlobService>,
    Arc<dyn snix_castore::directoryservice::DirectoryService>,
    Arc<dyn snix_store::pathinfoservice::PathInfoService>,
) {
    let blob_store = aspen_blob::InMemoryBlobStore::new();
    let blob_svc: Arc<dyn snix_castore::blobservice::BlobService> = Arc::new(IrohBlobService::new(blob_store));

    let kv = Arc::new(aspen_testing::DeterministicKeyValueStore::new()) as Arc<dyn aspen_core::KeyValueStore>;
    let dir_svc: Arc<dyn snix_castore::directoryservice::DirectoryService> =
        Arc::new(RaftDirectoryService::from_arc(kv.clone()));
    let pathinfo_svc: Arc<dyn snix_store::pathinfoservice::PathInfoService> =
        Arc::new(RaftPathInfoService::from_arc(kv));

    (blob_svc, dir_svc, pathinfo_svc)
}
