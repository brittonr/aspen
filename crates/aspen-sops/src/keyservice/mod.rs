//! gRPC key service bridge for Go SOPS compatibility.
//!
//! Implements the SOPS `KeyService` gRPC protocol over a Unix socket.
//! The Go `sops` binary connects via `--keyservice unix:///path/to/socket`.
//!
//! ## Usage
//!
//! ```bash
//! # Start the key service bridge
//! aspen-sops keyservice --cluster-ticket aspen1q...
//!
//! # In another terminal, use Go SOPS with the key service
//! sops --hc-vault-transit vault_address:engine_path/keys/key_name \
//!      --keyservice unix:///tmp/aspen-sops.sock \
//!      encrypt secrets.yaml
//! ```
//!
//! ## Key Type Mapping
//!
//! - `VaultKey`: Mapped to Aspen Transit. `engine_path` → mount, `key_name` → key. The
//!   `vault_address` field is ignored (uses the configured cluster ticket).
//! - All other key types (KMS, PGP, GCP KMS, Azure, Age, HCKMS) are rejected with `UNIMPLEMENTED`.
//!
//! Requires the `keyservice` feature flag.

mod service;

use std::path::PathBuf;

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::info;

use self::service::AspenKeyService;
use self::service::proto::key_service_server::KeyServiceServer;
use crate::error::Result;
use crate::error::SopsError;

/// Configuration for the gRPC key service.
#[derive(Debug, Clone)]
pub struct KeyserviceConfig {
    /// Aspen cluster ticket.
    pub cluster_ticket: String,
    /// Default Transit key name (used if VaultKey doesn't specify one).
    pub transit_key: String,
    /// Default Transit mount point (used if VaultKey doesn't specify one).
    pub transit_mount: String,
    /// Unix socket path.
    pub socket_path: PathBuf,
}

/// Start the gRPC key service.
///
/// Binds a Unix socket and serves the SOPS `KeyService` protocol,
/// translating Encrypt/Decrypt calls to Aspen Transit RPCs.
///
/// This function runs until the process receives SIGINT/SIGTERM.
pub async fn start_keyservice(config: &KeyserviceConfig) -> Result<()> {
    // Remove stale socket if it exists
    if config.socket_path.exists() {
        tokio::fs::remove_file(&config.socket_path).await.map_err(|e| SopsError::KeyServiceBind {
            path: config.socket_path.display().to_string(),
            reason: format!("failed to remove stale socket: {e}"),
        })?;
    }

    // Create parent directory if needed
    if let Some(parent) = config.socket_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| SopsError::KeyServiceBind {
            path: config.socket_path.display().to_string(),
            reason: format!("failed to create socket directory: {e}"),
        })?;
    }

    // Bind Unix socket
    let listener = UnixListener::bind(&config.socket_path).map_err(|e| SopsError::KeyServiceBind {
        path: config.socket_path.display().to_string(),
        reason: e.to_string(),
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&config.socket_path, perms).map_err(|e| SopsError::KeyServiceBind {
            path: config.socket_path.display().to_string(),
            reason: format!("failed to set socket permissions: {e}"),
        })?;
    }

    let stream = UnixListenerStream::new(listener);

    // Create the key service implementation
    let svc =
        AspenKeyService::new(config.cluster_ticket.clone(), config.transit_key.clone(), config.transit_mount.clone());

    info!(
        socket = %config.socket_path.display(),
        transit_key = config.transit_key,
        transit_mount = config.transit_mount,
        "Starting SOPS key service"
    );

    info!("Use with: sops --keyservice unix://{} <command>", config.socket_path.display());

    // Serve gRPC over the Unix socket
    Server::builder()
        .add_service(KeyServiceServer::new(svc))
        .serve_with_incoming(stream)
        .await
        .map_err(|e| SopsError::KeyServiceBind {
            path: config.socket_path.display().to_string(),
            reason: format!("gRPC server error: {e}"),
        })?;

    // Cleanup socket on shutdown
    let _ = tokio::fs::remove_file(&config.socket_path).await;
    info!("Key service stopped");

    Ok(())
}
