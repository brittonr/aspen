//! DAG sync protocol handler for the Iroh Router.
//!
//! Implements `iroh::protocol::ProtocolHandler` to accept incoming DAG sync
//! connections routed by ALPN. Each connection opens a single bidirectional
//! stream: the receiver sends a `DagSyncRequest`, the sender streams back
//! `ResponseFrame`s.
//!
//! # Usage
//!
//! ```ignore
//! use aspen_dag::handler::DagSyncProtocolHandler;
//!
//! let handler = DagSyncProtocolHandler::new(on_request);
//! router_builder.accept(DAG_SYNC_ALPN, handler);
//! ```

use std::sync::Arc;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::constants::MAX_DAG_SYNC_REQUEST_SIZE;
use crate::protocol::DagSyncRequest;
use crate::protocol::ProtocolError;
use crate::sync::SyncStats;

/// Maximum concurrent DAG sync connections.
const MAX_DAG_SYNC_CONNECTIONS: u32 = 64;

/// Callback type for handling DAG sync requests.
///
/// The handler calls this with the parsed request, a send stream, and a recv stream.
/// The callback is responsible for executing the traversal and writing response frames.
///
/// # Arguments
///
/// * `request` - The parsed DAG sync request
/// * `send` - The QUIC send stream to write response frames to
/// * `recv` - The QUIC recv stream (already consumed for the request; passed for future use)
pub type OnRequest = Arc<dyn Fn(DagSyncRequest, iroh::endpoint::SendStream) -> OnRequestFuture + Send + Sync + 'static>;

/// Future returned by the request callback.
pub type OnRequestFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<SyncStats, ProtocolError>> + Send>>;

/// Protocol handler for DAG sync over Iroh QUIC.
///
/// Accepts incoming connections on `DAG_SYNC_ALPN`, reads the request,
/// and delegates to a user-provided callback for the response.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Bounded request size via `MAX_DAG_SYNC_REQUEST_SIZE`
pub struct DagSyncProtocolHandler {
    on_request: OnRequest,
    connection_semaphore: Arc<Semaphore>,
}

impl std::fmt::Debug for DagSyncProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DagSyncProtocolHandler")
            .field("max_connections", &MAX_DAG_SYNC_CONNECTIONS)
            .finish()
    }
}

impl DagSyncProtocolHandler {
    /// Create a new DAG sync protocol handler with the given request callback.
    pub fn new(on_request: OnRequest) -> Self {
        let max_connections = match usize::try_from(MAX_DAG_SYNC_CONNECTIONS) {
            Ok(max_connections) => max_connections,
            Err(_) => usize::MAX,
        };
        Self {
            on_request,
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    /// Create from a closure for convenience.
    pub fn from_fn<F, Fut>(f: F) -> Self
    where
        F: Fn(DagSyncRequest, iroh::endpoint::SendStream) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<SyncStats, ProtocolError>> + Send + 'static,
    {
        Self::new(Arc::new(move |req, send| Box::pin(f(req, send))))
    }
}

impl ProtocolHandler for DagSyncProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();

        // Acquire connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                warn!(
                    remote = %remote,
                    max = MAX_DAG_SYNC_CONNECTIONS,
                    "DAG sync connection limit reached, rejecting"
                );
                return Err(AcceptError::from_err(std::io::Error::other("dag sync connection limit reached")));
            }
        };

        debug!(remote = %remote, "accepted DAG sync connection");

        // Accept bidirectional stream
        let (send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(|e| AcceptError::from_err(std::io::Error::other(e.to_string())))?;

        // Read request with size bound
        let max_request_size_bytes = match usize::try_from(MAX_DAG_SYNC_REQUEST_SIZE) {
            Ok(max_request_size_bytes) => max_request_size_bytes,
            Err(_) => usize::MAX,
        };
        let request_bytes = recv
            .read_to_end(max_request_size_bytes)
            .await
            .map_err(|e| AcceptError::from_err(std::io::Error::other(e.to_string())))?;

        debug!(
            remote = %remote,
            request_size = request_bytes.len(),
            "received DAG sync request"
        );

        // Parse request
        let request = DagSyncRequest::from_bytes(&request_bytes)
            .map_err(|e| AcceptError::from_err(std::io::Error::other(format!("invalid DAG sync request: {e}"))))?;

        // Delegate to the callback
        let on_request = Arc::clone(&self.on_request);
        match (on_request)(request, send).await {
            Ok(stats) => {
                info!(
                    remote = %remote,
                    data_frames = stats.data_frames,
                    hash_only_frames = stats.hash_only_frames,
                    bytes = stats.bytes_transferred,
                    "DAG sync completed"
                );
            }
            Err(e) => {
                warn!(remote = %remote, error = %e, "DAG sync failed");
            }
        }

        // Wait for the peer to close the connection.
        // This keeps the connection alive until the receiver finishes reading
        // the response stream. Without this, returning from accept() would
        // drop the connection, causing ApplicationClosed on the receiver.
        connection.closed().await;

        drop(permit);
        Ok(())
    }

    async fn shutdown(&self) {
        info!("DAG sync protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Result of connecting for DAG sync. Holds the connection alive.
pub struct DagSyncConnection {
    /// The QUIC recv stream for reading response frames.
    pub recv: iroh::endpoint::RecvStream,
    /// The connection handle. Must be kept alive until reading is done.
    _connection: iroh::endpoint::Connection,
}

/// Connect to a remote peer and run a DAG sync as the receiver.
///
/// Opens a connection with the DAG_SYNC_ALPN, sends the request,
/// and returns a [`DagSyncConnection`] holding the recv stream.
/// The connection stays alive as long as the returned value is held.
///
/// # Arguments
///
/// * `endpoint` - The local Iroh endpoint
/// * `remote` - The remote peer's address
/// * `request` - The sync request to send
///
/// # Returns
///
/// A [`DagSyncConnection`] with the recv stream ready for `recv_sync`.
pub async fn connect_dag_sync(
    endpoint: &iroh::Endpoint,
    remote: iroh::EndpointAddr,
    request: &DagSyncRequest,
) -> Result<DagSyncConnection, ProtocolError> {
    use crate::protocol::DAG_SYNC_ALPN;

    debug_assert!(!DAG_SYNC_ALPN.is_empty(), "ALPN must not be empty");
    debug_assert!(std::mem::size_of_val(request) > 0, "request must be non-zero size");

    let connection = endpoint.connect(remote, DAG_SYNC_ALPN).await.map_err(|e| ProtocolError::Io {
        source: std::io::Error::other(e.to_string()),
    })?;

    let (mut send, recv) = connection.open_bi().await.map_err(|e| ProtocolError::Io {
        source: std::io::Error::other(e.to_string()),
    })?;

    let request_bytes = request.to_bytes()?;

    send.write_all(&request_bytes).await.map_err(|e| ProtocolError::Io {
        source: std::io::Error::other(e.to_string()),
    })?;
    send.finish().map_err(|e| ProtocolError::Io {
        source: std::io::Error::other(e.to_string()),
    })?;

    Ok(DagSyncConnection {
        recv,
        _connection: connection,
    })
}
