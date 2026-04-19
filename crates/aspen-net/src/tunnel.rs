//! TCP tunnel through iroh QUIC.
//!
//! Provides two sides of the tunnel:
//!
//! - **`TunnelAcceptor`**: iroh `ProtocolHandler` that accepts incoming tunnel connections on
//!   `NET_TUNNEL_ALPN`. Reads a target port from the QUIC stream, connects to `127.0.0.1:{port}`,
//!   and copies bytes bidirectionally.
//!
//! - **`open_tunnel`**: Client-side function called by the SOCKS5 proxy to open a tunnel to a
//!   remote endpoint. Connects via iroh, sends the target port, and returns the QUIC streams for
//!   bidirectional copy.
//!
//! ## Wire format
//!
//! The tunnel QUIC stream carries a 2-byte big-endian port number as its first
//! message, followed by raw TCP payload bytes in both directions.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use iroh::EndpointAddr;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use snafu::Snafu;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::constants::MAX_SOCKS5_CONNECTIONS;

/// Errors from tunnel operations.
#[derive(Debug, Snafu)]
pub enum TunnelError {
    /// Failed to read port from QUIC stream.
    #[snafu(display("failed to read tunnel port: {reason}"))]
    ReadPort { reason: String },

    /// Failed to connect to local service.
    #[snafu(display("failed to connect to 127.0.0.1:{port}: {reason}"))]
    LocalConnect { port: u16, reason: String },

    /// I/O error during tunnel copy.
    #[snafu(display("tunnel copy error: {reason}"))]
    Copy { reason: String },

    /// Connection limit reached.
    #[snafu(display("tunnel connection limit reached ({max})"))]
    ConnectionLimit { max: u32 },

    /// Failed to connect to remote endpoint.
    #[snafu(display("failed to connect to remote endpoint: {reason}"))]
    RemoteConnect { reason: String },
}

/// Iroh protocol handler that accepts tunnel connections.
///
/// When a remote peer connects on `NET_TUNNEL_ALPN`, the acceptor:
/// 1. Reads a `u16` port (big-endian) from the QUIC stream
/// 2. Connects to `127.0.0.1:{port}` via TCP
/// 3. Copies bytes bidirectionally until either side closes
pub struct TunnelAcceptor {
    cancel: CancellationToken,
    active: Arc<AtomicU32>,
    max_connections: u32,
}

impl TunnelAcceptor {
    /// Create a new tunnel acceptor.
    pub fn new(cancel: CancellationToken) -> Self {
        Self {
            cancel,
            active: Arc::new(AtomicU32::new(0)),
            max_connections: MAX_SOCKS5_CONNECTIONS,
        }
    }

    /// Get the number of active tunnel connections.
    pub fn active_count(&self) -> u32 {
        self.active.load(Ordering::Relaxed)
    }

    /// Handle a single tunnel stream: read port, connect local, copy bytes.
    async fn handle_stream(
        recv: &mut iroh::endpoint::RecvStream,
        send: &mut iroh::endpoint::SendStream,
        cancel: CancellationToken,
    ) -> Result<(), TunnelError> {
        // Read the 2-byte target port (big-endian)
        let mut target_port_bytes = [0u8; 2];
        recv.read_exact(&mut target_port_bytes)
            .await
            .map_err(|e| TunnelError::ReadPort { reason: e.to_string() })?;
        let local_service_endpoint = u16::from_be_bytes(target_port_bytes);

        debug!(local_service_endpoint, "tunnel: connecting to local service");

        // Connect to the local service
        let mut tcp =
            TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], local_service_endpoint))).await.map_err(|e| {
                TunnelError::LocalConnect {
                    port: local_service_endpoint,
                    reason: e.to_string(),
                }
            })?;

        debug!(local_service_endpoint, "tunnel: connected, starting bidirectional copy");

        // Split TCP stream
        let (mut tcp_read, mut tcp_write) = tcp.split();

        // Copy bidirectionally until either side closes or cancel fires
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!(local_service_endpoint, "tunnel: cancelled");
            }
            result = async {
                let client_to_server = tokio::io::copy(recv, &mut tcp_write);
                let server_to_client = tokio::io::copy(&mut tcp_read, send);
                tokio::try_join!(client_to_server, server_to_client)
            } => {
                match result {
                    Ok((c2s, s2c)) => {
                        debug!(local_service_endpoint, c2s, s2c, "tunnel: copy complete");
                    }
                    Err(e) => {
                        debug!(local_service_endpoint, error = %e, "tunnel: copy ended with error");
                    }
                }
            }
        }

        // Best-effort shutdown
        if let Err(error) = send.finish() {
            debug!(local_service_endpoint, error = ?error, "tunnel: finish failed");
        }

        Ok(())
    }
}

impl ProtocolHandler for TunnelAcceptor {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();

        // Check connection limit
        let count = self.active.load(Ordering::Relaxed);
        if count >= self.max_connections {
            warn!(count, max = self.max_connections, "tunnel: connection limit reached, rejecting");
            return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
        }

        // Accept a bidirectional stream
        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(|e| AcceptError::from_err(std::io::Error::other(format!("accept_bi failed: {e}"))))?;

        self.active.fetch_add(1, Ordering::Relaxed);
        let active = Arc::clone(&self.active);
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::handle_stream(&mut recv, &mut send, cancel).await {
                debug!(remote = %remote, error = %e, "tunnel: stream error");
            }
            active.fetch_sub(1, Ordering::Relaxed);
        });

        Ok(())
    }
}

impl std::fmt::Debug for TunnelAcceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunnelAcceptor")
            .field("active", &self.active.load(Ordering::Relaxed))
            .field("max_connections", &self.max_connections)
            .finish()
    }
}

/// Open a tunnel to a remote endpoint through iroh QUIC.
///
/// Returns the QUIC send/recv streams after sending the target port.
/// The caller is responsible for bidirectional copy between the returned
/// streams and the local TCP socket.
pub async fn open_tunnel(
    endpoint: &iroh::Endpoint,
    addr: EndpointAddr,
    port: u16,
) -> Result<(iroh::endpoint::SendStream, iroh::endpoint::RecvStream), TunnelError> {
    use aspen_transport::constants::NET_TUNNEL_ALPN;

    // Connect to the remote endpoint
    let connection = endpoint
        .connect(addr, NET_TUNNEL_ALPN)
        .await
        .map_err(|e| TunnelError::RemoteConnect { reason: e.to_string() })?;

    // Open a bidirectional stream
    let (mut send, recv) = connection.open_bi().await.map_err(|e| TunnelError::RemoteConnect {
        reason: format!("open_bi: {e}"),
    })?;

    // Send the target port (big-endian u16)
    send.write_all(&port.to_be_bytes()).await.map_err(|e| TunnelError::RemoteConnect {
        reason: format!("write port: {e}"),
    })?;

    info!(port, "tunnel: opened to remote endpoint");

    Ok((send, recv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunnel_acceptor_initial_state() {
        let cancel = CancellationToken::new();
        let acceptor = TunnelAcceptor::new(cancel);
        assert_eq!(acceptor.active_count(), 0);
        assert_eq!(acceptor.max_connections, MAX_SOCKS5_CONNECTIONS);
    }

    #[test]
    fn tunnel_acceptor_debug() {
        let cancel = CancellationToken::new();
        let acceptor = TunnelAcceptor::new(cancel);
        let debug = format!("{acceptor:?}");
        assert!(debug.contains("TunnelAcceptor"));
        assert!(debug.contains("active: 0"));
    }

    #[tokio::test]
    async fn open_tunnel_port_encoding() {
        // Verify port encoding is correct big-endian
        let port: u16 = 8080;
        let bytes = port.to_be_bytes();
        assert_eq!(bytes, [0x1F, 0x90]);
        assert_eq!(u16::from_be_bytes(bytes), 8080);
    }
}
