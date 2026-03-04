//! gRPC key service bridge for Go SOPS compatibility.
//!
//! Implements the SOPS `KeyService` gRPC protocol over a Unix socket.
//! The Go `sops` binary connects via `--keyservice unix:///path/to/socket`.
//!
//! Requires the `keyservice` feature flag.

use std::path::PathBuf;

use crate::error::Result;
use crate::error::SopsError;

/// Configuration for the gRPC key service.
#[derive(Debug, Clone)]
pub struct KeyserviceConfig {
    /// Aspen cluster ticket.
    pub cluster_ticket: String,
    /// Transit key name.
    pub transit_key: String,
    /// Transit mount point.
    pub transit_mount: String,
    /// Unix socket path.
    pub socket_path: PathBuf,
}

/// Start the gRPC key service.
///
/// Binds a Unix socket and serves the SOPS `KeyService` protocol,
/// translating Encrypt/Decrypt calls to Aspen Transit RPCs.
pub async fn start_keyservice(_config: &KeyserviceConfig) -> Result<()> {
    // TODO: Implement gRPC key service bridge
    // This requires:
    // 1. SOPS KeyService protobuf definition
    // 2. tonic gRPC server on Unix socket
    // 3. AspenKeyServiceBridge translating encrypt/decrypt to Transit RPCs
    Err(SopsError::KeyServiceBind {
        path: _config.socket_path.display().to_string(),
        reason: "gRPC key service not yet implemented — use native mode instead".into(),
    })
}
