//! Generic connection and stream management utilities.
//!
//! Provides reusable components for handling Iroh QUIC connections with
//! resource limits and graceful error handling.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use iroh::endpoint::{Connection, RecvStream, SendStream};
use tokio::sync::Semaphore;
use tracing::{debug, warn};

/// Connection manager with bounded connection and stream limits.
///
/// Provides Tiger Style resource management:
/// - Fixed maximum concurrent connections
/// - Fixed maximum streams per connection
/// - Graceful degradation when limits are reached
pub struct ConnectionManager {
    connection_semaphore: Arc<Semaphore>,
    max_connections: u32,
    max_streams_per_connection: u32,
}

impl ConnectionManager {
    /// Create a new connection manager.
    pub fn new(max_connections: u32, max_streams_per_connection: u32) -> Self {
        Self {
            connection_semaphore: Arc::new(Semaphore::new(max_connections as usize)),
            max_connections,
            max_streams_per_connection,
        }
    }

    /// Try to acquire a connection permit.
    ///
    /// Returns `None` if the connection limit has been reached.
    pub fn try_acquire_connection(&self) -> Option<ConnectionPermit> {
        match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => Some(ConnectionPermit { _permit: permit }),
            Err(_) => None,
        }
    }

    /// Get maximum connections allowed.
    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    /// Get maximum streams per connection.
    pub fn max_streams_per_connection(&self) -> u32 {
        self.max_streams_per_connection
    }

    /// Shutdown the connection manager.
    ///
    /// This closes the semaphore and prevents new connections.
    pub fn shutdown(&self) {
        self.connection_semaphore.close();
    }
}

/// A permit representing a reserved connection slot.
///
/// When dropped, the permit is automatically returned to the semaphore.
pub struct ConnectionPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

/// Stream manager for handling multiple bidirectional streams on a connection.
///
/// Provides per-connection stream limiting with graceful degradation.
pub struct StreamManager {
    stream_semaphore: Arc<Semaphore>,
    active_streams: Arc<AtomicU32>,
    max_streams: u32,
}

impl StreamManager {
    /// Create a new stream manager for a connection.
    pub fn new(max_streams_per_connection: u32) -> Self {
        Self {
            stream_semaphore: Arc::new(Semaphore::new(max_streams_per_connection as usize)),
            active_streams: Arc::new(AtomicU32::new(0)),
            max_streams: max_streams_per_connection,
        }
    }

    /// Try to acquire a stream permit.
    ///
    /// Returns `None` if the stream limit for this connection has been reached.
    pub fn try_acquire_stream(&self) -> Option<StreamPermit> {
        match self.stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                let active_count = self.active_streams.fetch_add(1, Ordering::Relaxed);
                debug!(active_streams = active_count + 1, max_streams = self.max_streams, "acquired stream permit");
                Some(StreamPermit {
                    _permit: permit,
                    active_streams: self.active_streams.clone(),
                })
            }
            Err(_) => None,
        }
    }

    /// Get current number of active streams.
    pub fn active_streams(&self) -> u32 {
        self.active_streams.load(Ordering::Relaxed)
    }

    /// Get maximum streams allowed for this connection.
    pub fn max_streams(&self) -> u32 {
        self.max_streams
    }
}

/// A permit representing a reserved stream slot.
///
/// When dropped, decrements the active stream count and returns the permit.
pub struct StreamPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    active_streams: Arc<AtomicU32>,
}

impl Drop for StreamPermit {
    fn drop(&mut self) {
        let active_count = self.active_streams.fetch_sub(1, Ordering::Relaxed);
        debug!(active_streams = active_count - 1, "released stream permit");
    }
}

/// Accept and handle bidirectional streams on a connection with resource limits.
///
/// This function accepts streams from the connection and spawns tasks to handle them,
/// with proper resource management using the provided stream manager.
///
/// # Arguments
/// * `connection` - The Iroh QUIC connection
/// * `stream_manager` - Manager for stream resource limits
/// * `handler` - Async function to handle each stream
///
/// # Returns
/// Returns when the connection closes or an error occurs.
pub async fn handle_connection_streams<F, Fut>(
    connection: Connection,
    stream_manager: StreamManager,
    handler: F,
) -> anyhow::Result<()>
where
    F: Fn(RecvStream, SendStream) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let remote_node_id = connection.remote_id();
    let handler = Arc::new(handler);

    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                debug!(remote_node = %remote_node_id, error = %err, "connection closed");
                break;
            }
        };

        let permit = match stream_manager.try_acquire_stream() {
            Some(permit) => permit,
            None => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = stream_manager.max_streams(),
                    "stream limit reached, dropping stream"
                );
                continue;
            }
        };

        let (send, recv) = stream;
        let handler_clone = handler.clone();
        tokio::spawn(async move {
            let _permit = permit; // Keep permit alive for the duration
            if let Err(err) = handler_clone(recv, send).await {
                warn!(error = %err, "failed to handle stream");
            }
        });
    }

    Ok(())
}
