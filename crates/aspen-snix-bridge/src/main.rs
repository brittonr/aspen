//! gRPC bridge for snix-store CLI ↔ Aspen storage.
//!
//! Runs a local gRPC server on a Unix socket that translates snix's
//! BlobService/DirectoryService/PathInfoService gRPC protocol to Aspen-backed
//! implementations. This lets `snix-store virtiofs` (or `snix-store daemon`)
//! use Aspen's distributed storage:
//!
//! ```bash
//! # Start the bridge
//! aspen-snix-bridge --socket /tmp/aspen-castore.sock
//!
//! # Point snix-store at the bridge
//! BLOB_SERVICE_ADDR=grpc+unix:///tmp/aspen-castore.sock \
//! DIRECTORY_SERVICE_ADDR=grpc+unix:///tmp/aspen-castore.sock \
//! PATH_INFO_SERVICE_ADDR=grpc+unix:///tmp/aspen-castore.sock \
//!   snix-store virtiofs /tmp/snix.sock
//! ```
//!
//! Currently uses in-memory backends for standalone testing.
//! Future: connect to a live Aspen cluster via ticket.

use std::path::PathBuf;
use std::sync::Arc;

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

#[derive(Parser, Debug)]
#[command(name = "aspen-snix-bridge")]
#[command(about = "gRPC bridge: snix-store ↔ Aspen distributed storage")]
struct Args {
    /// Unix socket path for gRPC server.
    #[arg(long, default_value = "/tmp/aspen-castore.sock")]
    socket: PathBuf,
    // Future: --ticket <cluster-ticket> for connecting to a live cluster
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Remove stale socket
    if args.socket.exists() {
        tokio::fs::remove_file(&args.socket).await?;
    }
    if let Some(parent) = args.socket.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Create Aspen-backed snix services.
    // Currently uses in-memory backends for standalone testing.
    // TODO: Accept --ticket and connect to a live cluster (IrpcBlobService etc.)
    use snix_castore::blobservice::BlobService;
    use snix_castore::directoryservice::DirectoryService;
    use snix_store::pathinfoservice::PathInfoService;

    let blob_store = aspen_blob::InMemoryBlobStore::new();
    let blob_svc: Arc<dyn BlobService> = Arc::new(IrohBlobService::new(blob_store));

    let kv = Arc::new(aspen_testing::DeterministicKeyValueStore::new()) as Arc<dyn aspen_core::KeyValueStore>;
    let dir_svc: Arc<dyn DirectoryService> = Arc::new(RaftDirectoryService::from_arc(kv.clone()));
    let pathinfo_svc: Arc<dyn PathInfoService> = Arc::new(RaftPathInfoService::from_arc(kv));

    // Create NAR calculation service (renders NARs from blob+dir data)
    let nar_calc = Box::new(SimpleRenderer::new(blob_svc.clone(), dir_svc.clone()));

    // Wrap in snix's gRPC server wrappers
    let blob_grpc = GRPCBlobServiceWrapper::new(blob_svc);
    let dir_grpc = GRPCDirectoryServiceWrapper::new(dir_svc);
    let pathinfo_grpc = GRPCPathInfoServiceWrapper::new(pathinfo_svc, nar_calc);

    // Bind Unix socket
    let listener = UnixListener::bind(&args.socket)?;
    let incoming = UnixListenerStream::new(listener);

    info!(socket = %args.socket.display(), "starting snix gRPC bridge");

    let (_health_reporter, health_service) = tonic_health::server::health_reporter();

    Server::builder()
        .add_service(health_service)
        .add_service(BlobServiceServer::new(blob_grpc))
        .add_service(DirectoryServiceServer::new(dir_grpc))
        .add_service(PathInfoServiceServer::new(pathinfo_grpc))
        .serve_with_incoming_shutdown(incoming, async {
            tokio::signal::ctrl_c().await.ok();
            info!("shutting down gRPC bridge");
        })
        .await?;

    // Clean up socket
    let _ = tokio::fs::remove_file(&args.socket).await;

    Ok(())
}
