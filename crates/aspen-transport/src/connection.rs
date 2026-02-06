//! Generic connection and stream management utilities.
//!
//! Provides reusable components for handling Iroh QUIC connections with
//! resource limits and graceful error handling.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use iroh::endpoint::Connection;
use iroh::endpoint::RecvStream;
use iroh::endpoint::SendStream;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::warn;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_creation() {
        let manager = ConnectionManager::new(10, 5);
        assert_eq!(manager.max_connections(), 10);
        assert_eq!(manager.max_streams_per_connection(), 5);
    }

    #[test]
    fn test_connection_permit_acquisition() {
        let manager = ConnectionManager::new(2, 5);

        // Should be able to acquire up to max_connections permits
        let permit1 = manager.try_acquire_connection();
        assert!(permit1.is_some(), "first permit should be granted");

        let permit2 = manager.try_acquire_connection();
        assert!(permit2.is_some(), "second permit should be granted");

        // Third should fail - at limit
        let permit3 = manager.try_acquire_connection();
        assert!(permit3.is_none(), "third permit should be denied");
    }

    #[test]
    fn test_connection_permit_release() {
        let manager = ConnectionManager::new(1, 5);

        // Acquire the only permit
        let permit1 = manager.try_acquire_connection();
        assert!(permit1.is_some(), "permit should be granted");

        // Cannot acquire another while held
        let permit2 = manager.try_acquire_connection();
        assert!(permit2.is_none(), "second permit should be denied while first is held");

        // Drop the first permit
        drop(permit1);

        // Should be able to acquire again
        let permit3 = manager.try_acquire_connection();
        assert!(permit3.is_some(), "permit should be available after release");
    }

    #[test]
    fn test_connection_manager_shutdown() {
        let manager = ConnectionManager::new(10, 5);

        // Shutdown should close the semaphore
        manager.shutdown();

        // Cannot acquire permits after shutdown
        let permit = manager.try_acquire_connection();
        assert!(permit.is_none(), "permits should be denied after shutdown");
    }

    #[test]
    fn test_stream_manager_creation() {
        let stream_manager = StreamManager::new(10);
        assert_eq!(stream_manager.max_streams(), 10);
        assert_eq!(stream_manager.active_streams(), 0);
    }

    #[test]
    fn test_stream_permit_acquisition() {
        let stream_manager = StreamManager::new(2);
        assert_eq!(stream_manager.active_streams(), 0);

        // Acquire first stream permit
        let permit1 = stream_manager.try_acquire_stream();
        assert!(permit1.is_some(), "first stream permit should be granted");
        assert_eq!(stream_manager.active_streams(), 1);

        // Acquire second stream permit
        let permit2 = stream_manager.try_acquire_stream();
        assert!(permit2.is_some(), "second stream permit should be granted");
        assert_eq!(stream_manager.active_streams(), 2);

        // Third should fail
        let permit3 = stream_manager.try_acquire_stream();
        assert!(permit3.is_none(), "third stream permit should be denied");
        assert_eq!(stream_manager.active_streams(), 2);
    }

    #[test]
    fn test_stream_permit_release_decrements_counter() {
        let stream_manager = StreamManager::new(5);

        let permit1 = stream_manager.try_acquire_stream();
        assert!(permit1.is_some());
        assert_eq!(stream_manager.active_streams(), 1);

        let permit2 = stream_manager.try_acquire_stream();
        assert!(permit2.is_some());
        assert_eq!(stream_manager.active_streams(), 2);

        // Drop first permit
        drop(permit1);
        assert_eq!(stream_manager.active_streams(), 1);

        // Drop second permit
        drop(permit2);
        assert_eq!(stream_manager.active_streams(), 0);
    }

    #[test]
    fn test_stream_manager_permits_reusable() {
        let stream_manager = StreamManager::new(1);

        // Acquire and release multiple times
        for i in 0..5 {
            let permit = stream_manager.try_acquire_stream();
            assert!(permit.is_some(), "permit {} should be granted", i);
            assert_eq!(stream_manager.active_streams(), 1);
            drop(permit);
            assert_eq!(stream_manager.active_streams(), 0);
        }
    }

    #[test]
    fn test_connection_manager_zero_capacity() {
        // Edge case: zero max connections
        let manager = ConnectionManager::new(0, 5);
        let permit = manager.try_acquire_connection();
        assert!(permit.is_none(), "no permits should be available with zero capacity");
    }

    #[test]
    fn test_stream_manager_zero_capacity() {
        // Edge case: zero max streams
        let stream_manager = StreamManager::new(0);
        let permit = stream_manager.try_acquire_stream();
        assert!(permit.is_none(), "no permits should be available with zero capacity");
    }

    #[test]
    fn test_connection_manager_high_capacity() {
        // Test with larger capacity to verify no overflow issues
        let manager = ConnectionManager::new(1000, 100);
        assert_eq!(manager.max_connections(), 1000);

        // Acquire a few permits
        let permits: Vec<_> = (0..10).filter_map(|_| manager.try_acquire_connection()).collect();
        assert_eq!(permits.len(), 10);
    }
}
