//! nix-daemon protocol support for aspen-snix-bridge.
//!
//! Exposes a Unix domain socket speaking the nix-daemon worker protocol,
//! backed by the same `BlobService`/`DirectoryService`/`PathInfoService`
//! instances as the gRPC bridge. Any standard `nix` CLI can connect:
//!
//! ```bash
//! nix path-info --store unix:///tmp/aspen-nix-daemon.sock /nix/store/...
//! nix copy --to unix:///tmp/aspen-nix-daemon.sock /nix/store/...
//! ```
//!
//! The daemon handles these operations (via [`nix_daemon::SnixDaemon`]):
//! - `QueryPathInfo` — look up store paths in `PathInfoService`
//! - `IsValidPath` — check existence in `PathInfoService`
//! - `AddToStoreNar` — ingest NARs via `ingest_nar_and_hash`
//! - `QueryValidPaths` — batch validation

use std::path::Path;
use std::sync::Arc;

use nix_compat::nix_daemon::handler::NixDaemon;
use nix_daemon::SnixDaemon;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::pathinfoservice::PathInfoService;
use tokio::net::UnixListener;
use tracing::error;
use tracing::info;

/// Start the nix-daemon protocol listener on the given Unix socket path.
///
/// Accepts connections in a loop, spawning a task per client. Each client
/// goes through the nix-daemon handshake and then processes operations
/// until the connection closes.
///
/// The daemon shares the same service instances as the gRPC bridge, so
/// paths added via one protocol are visible via the other immediately.
pub async fn serve_daemon(
    socket_path: &Path,
    blob_service: Arc<dyn BlobService>,
    directory_service: Arc<dyn DirectoryService>,
    path_info_service: Arc<dyn PathInfoService>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Remove stale socket
    if socket_path.exists() {
        tokio::fs::remove_file(socket_path).await?;
    }
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(socket_path)?;
    info!(socket = %socket_path.display(), "nix-daemon listener started");

    let io = Arc::new(SnixDaemon::new(blob_service, directory_service, path_info_service));

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let io = Arc::clone(&io);
                        tokio::spawn(async move {
                            match NixDaemon::initialize(io, stream).await {
                                Ok(mut daemon) => {
                                    if let Err(e) = daemon.handle_client().await {
                                        // UnexpectedEof is normal — client closed the connection
                                        if e.kind() != std::io::ErrorKind::UnexpectedEof {
                                            error!(error = %e, "nix-daemon client error");
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "nix-daemon handshake failed");
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "nix-daemon accept failed");
                    }
                }
            }
            _ = shutdown.changed() => {
                info!("nix-daemon listener shutting down");
                break;
            }
        }
    }

    // Clean up socket
    let _ = tokio::fs::remove_file(socket_path).await;
    Ok(())
}
