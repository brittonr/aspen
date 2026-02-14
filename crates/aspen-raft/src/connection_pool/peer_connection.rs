//! Peer connection management with health tracking and stream multiplexing.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use iroh::endpoint::Connection;
use iroh::endpoint::RecvStream;
use iroh::endpoint::SendStream;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::warn;

use super::ConnectionHealth;
use super::MAX_CONNECTION_RETRIES;
use crate::constants::IROH_STREAM_OPEN_TIMEOUT;
use crate::constants::MAX_STREAMS_PER_CONNECTION;
use crate::types::NodeId;

/// A persistent connection to a peer node.
///
/// Manages a single QUIC connection with health tracking and stream multiplexing.
/// No per-stream authentication is needed - NodeId is verified at connection time
/// by Iroh's QUIC TLS layer.
///
/// Tiger Style: Bounded stream count, explicit health states.
pub struct PeerConnection {
    /// The persistent QUIC connection.
    connection: Connection,
    /// Node ID of the peer.
    node_id: NodeId,
    /// Last successful RPC timestamp (for idle timeout).
    last_used: AsyncMutex<Instant>,
    /// Connection health status.
    health: AsyncMutex<ConnectionHealth>,
    /// Semaphore for limiting concurrent streams.
    stream_semaphore: Arc<Semaphore>,
    /// Active stream count (for metrics).
    active_streams: Arc<AtomicU32>,
}

impl PeerConnection {
    /// Create a new peer connection.
    ///
    /// # Arguments
    ///
    /// * `connection` - The QUIC connection to the peer
    /// * `node_id` - The peer's Raft node ID
    ///
    /// # Security Note
    /// Authentication is handled by Iroh's QUIC TLS layer. The remote NodeId
    /// is cryptographically verified during connection establishment.
    pub fn new(connection: Connection, node_id: NodeId) -> Self {
        Self {
            connection,
            node_id,
            last_used: AsyncMutex::new(Instant::now()),
            health: AsyncMutex::new(ConnectionHealth::Healthy),
            stream_semaphore: Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize)),
            active_streams: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Acquire a bidirectional stream from this connection.
    ///
    /// Returns a `StreamHandle` that automatically decrements the active stream count
    /// and releases the semaphore permit when dropped. This ensures proper cleanup
    /// even if the caller forgets to close the streams or panics.
    ///
    /// No authentication handshake is performed - NodeId was verified at connection time.
    ///
    /// Tiger Style: Enforces stream limit per connection, fails fast on unhealthy connections.
    pub async fn acquire_stream(&self) -> Result<StreamHandle> {
        // Check connection health before attempting stream
        let health = *self.health.lock().await;
        if health == ConnectionHealth::Failed {
            return Err(anyhow::anyhow!("connection marked as failed"));
        }

        // Acquire stream permit (bounded concurrency)
        let permit = self
            .stream_semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| anyhow::anyhow!("stream limit reached ({} streams in use)", MAX_STREAMS_PER_CONNECTION))?;

        // Track active streams
        let active_count = self.active_streams.fetch_add(1, Ordering::Relaxed) + 1;
        debug!(
            node_id = %self.node_id,
            active_streams = active_count,
            "acquiring stream from connection"
        );

        // Open bidirectional stream with timeout
        let stream_result = tokio::time::timeout(IROH_STREAM_OPEN_TIMEOUT, self.connection.open_bi())
            .await
            .context("timeout opening stream")?
            .context("failed to open stream");

        // Handle stream open result
        match stream_result {
            Ok(stream) => {
                use crate::verified::transition_connection_health;

                debug!(node_id = %self.node_id, "stream opened successfully");
                // Update last used timestamp on success
                *self.last_used.lock().await = Instant::now();

                // Transition health state using pure function
                let mut health = self.health.lock().await;
                let new_health = transition_connection_health(*health, true, MAX_CONNECTION_RETRIES);
                if *health != new_health {
                    debug!(node_id = %self.node_id, "connection health recovered");
                    *health = new_health;
                }
                drop(health);

                let active_streams = Arc::clone(&self.active_streams);
                let (send, recv) = stream;

                // Return stream handle with guard that cleans up on drop
                // NO AUTH HANDSHAKE - NodeId verified at connection time
                let guard = StreamGuard {
                    _permit: permit,
                    active_streams,
                };

                Ok(StreamHandle {
                    send,
                    recv,
                    _guard: guard,
                })
            }
            Err(err) => {
                use crate::verified::transition_connection_health;

                // Decrement active streams on failure
                self.active_streams.fetch_sub(1, Ordering::Relaxed);

                // Update health status using pure state machine
                let mut health = self.health.lock().await;
                let old_health = *health;
                let new_health = transition_connection_health(*health, false, MAX_CONNECTION_RETRIES);
                *health = new_health;

                // Log state transitions
                match (old_health, new_health) {
                    (ConnectionHealth::Healthy, ConnectionHealth::Degraded { .. }) => {
                        warn!(
                            node_id = %self.node_id,
                            error = %err,
                            "connection degraded after stream failure"
                        );
                    }
                    (ConnectionHealth::Degraded { .. }, ConnectionHealth::Failed) => {
                        error!(
                            node_id = %self.node_id,
                            "connection marked as failed after repeated stream failures"
                        );
                    }
                    _ => {}
                }

                Err(err)
            }
        }
    }

    /// Check if connection is idle and should be cleaned up.
    pub async fn is_idle(&self, timeout: Duration) -> bool {
        let last_used = *self.last_used.lock().await;
        last_used.elapsed() > timeout && self.active_streams.load(Ordering::Relaxed) == 0
    }

    /// Get current health status.
    pub async fn health(&self) -> ConnectionHealth {
        *self.health.lock().await
    }

    /// Get active stream count.
    pub fn active_stream_count(&self) -> u32 {
        self.active_streams.load(Ordering::Relaxed)
    }
}

/// Handle to an acquired stream pair with automatic cleanup.
///
/// When this handle is dropped, the active stream count is decremented
/// and the semaphore permit is released. This ensures proper resource
/// tracking even if the caller doesn't explicitly close the streams.
///
/// Tiger Style: RAII pattern ensures cleanup happens on all code paths.
pub struct StreamHandle {
    /// The send stream for writing data.
    pub send: SendStream,
    /// The receive stream for reading data.
    pub recv: RecvStream,
    /// Guard that decrements counters on drop (held for lifetime of streams).
    _guard: StreamGuard,
}

/// Guard to automatically decrement stream count when dropped.
pub(crate) struct StreamGuard {
    pub(crate) _permit: tokio::sync::OwnedSemaphorePermit,
    pub(crate) active_streams: Arc<AtomicU32>,
}

impl Drop for StreamGuard {
    fn drop(&mut self) {
        self.active_streams.fetch_sub(1, Ordering::Relaxed);
    }
}
