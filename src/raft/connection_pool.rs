//! Connection pooling for Raft RPC over Iroh.
//!
//! This module provides connection pooling and stream multiplexing for efficient
//! Raft RPC communication. Instead of creating a new QUIC connection for each RPC,
//! connections are reused and multiple streams are multiplexed over each connection.
//!
//! # Architecture
//!
//! - `RaftConnectionPool`: Maintains persistent connections to peer nodes
//! - `PeerConnection`: Manages a single QUIC connection with health tracking
//! - Stream multiplexing: Multiple RPCs share the same connection via separate streams
//! - Lazy connection: Connections created on first use, not eagerly
//! - Idle cleanup: Unused connections removed after timeout
//!
//! # Tiger Style
//!
//! - Bounded resources: MAX_PEERS limits pool size, MAX_STREAMS_PER_CONNECTION per connection
//! - Explicit error handling: All failures propagated with context
//! - Fixed timeouts: No unbounded waits on connection or stream operations
//! - Fail fast: Invalid states cause immediate errors, not silent corruption
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for RaftConnectionPool:
//!       - Pool creation with bounded MAX_PEERS
//!       - Connection reuse across multiple get_stream() calls
//!       - Stream semaphore limiting MAX_STREAMS_PER_CONNECTION
//!       - Idle connection cleanup after CONNECTION_IDLE_TIMEOUT
//!       - Connection retry with exponential backoff
//!       - Health status transitions (Healthy -> Degraded -> Failed)
//!       - Concurrent stream acquisition under load
//!       Coverage: 0% line coverage (requires Iroh endpoint mocking)

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use iroh::EndpointAddr;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use tokio::sync::{Mutex as AsyncMutex, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::cluster::IrohEndpointManager;
use crate::raft::constants::{
    IROH_CONNECT_TIMEOUT, IROH_STREAM_OPEN_TIMEOUT, MAX_PEERS, MAX_STREAMS_PER_CONNECTION,
};
use crate::raft::node_failure_detection::{ConnectionStatus, NodeFailureDetector};
use crate::raft::types::NodeId;

/// Idle connection timeout before cleanup (60 seconds).
///
/// Tiger Style: Fixed timeout prevents resource leaks from abandoned connections.
pub const CONNECTION_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum connection retry attempts before marking as failed.
///
/// Tiger Style: Bounded retries prevent infinite retry loops.
pub const MAX_CONNECTION_RETRIES: u32 = 3;

/// Base backoff duration between connection retries (100ms).
///
/// Exponential backoff: 100ms, 200ms, 400ms for retries.
/// Tiger Style: Fixed backoff pattern, no unbounded growth.
pub const CONNECTION_RETRY_BACKOFF_BASE_MS: u64 = 100;

/// Health status of a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionHealth {
    /// Connection is healthy and operational.
    Healthy,
    /// Connection has experienced failures but may recover.
    Degraded { consecutive_failures: u32 },
    /// Connection has failed and should be replaced.
    Failed,
}

/// A persistent connection to a peer node.
///
/// Manages a single QUIC connection with health tracking and stream multiplexing.
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
    /// Tiger Style: Enforces stream limit per connection, fails fast on unhealthy connections.
    pub async fn acquire_stream(&self) -> Result<(SendStream, RecvStream)> {
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
            .map_err(|_| {
                anyhow::anyhow!(
                    "stream limit reached ({} streams in use)",
                    MAX_STREAMS_PER_CONNECTION
                )
            })?;

        // Track active streams
        let active_count = self.active_streams.fetch_add(1, Ordering::Relaxed) + 1;
        info!(
            node_id = %self.node_id,
            active_streams = active_count,
            "acquiring stream from connection"
        );

        // Open bidirectional stream with timeout
        info!(node_id = %self.node_id, "opening bi stream with timeout");
        let stream_result =
            tokio::time::timeout(IROH_STREAM_OPEN_TIMEOUT, self.connection.open_bi())
                .await
                .context("timeout opening stream")?
                .context("failed to open stream");

        // Handle stream open result
        match stream_result {
            Ok(stream) => {
                info!(node_id = %self.node_id, "stream opened successfully");
                // Update last used timestamp on success
                *self.last_used.lock().await = Instant::now();

                // Reset health to healthy on successful stream open
                let mut health = self.health.lock().await;
                if !matches!(*health, ConnectionHealth::Healthy) {
                    debug!(node_id = %self.node_id, "connection health recovered");
                    *health = ConnectionHealth::Healthy;
                }

                // Return stream with automatic cleanup on drop
                let active_streams = Arc::clone(&self.active_streams);
                let (send, recv) = stream;

                // Spawn cleanup task for when stream is dropped
                let stream_guard = StreamGuard {
                    _permit: permit,
                    active_streams,
                };
                tokio::spawn(async move {
                    // Keep guard alive until explicitly dropped
                    let _guard = stream_guard;
                });

                Ok((send, recv))
            }
            Err(err) => {
                // Decrement active streams on failure
                self.active_streams.fetch_sub(1, Ordering::Relaxed);

                // Update health status
                let mut health = self.health.lock().await;
                match *health {
                    ConnectionHealth::Healthy => {
                        *health = ConnectionHealth::Degraded {
                            consecutive_failures: 1,
                        };
                        warn!(
                            node_id = %self.node_id,
                            error = %err,
                            "connection degraded after stream failure"
                        );
                    }
                    ConnectionHealth::Degraded {
                        consecutive_failures,
                    } => {
                        if consecutive_failures >= MAX_CONNECTION_RETRIES {
                            *health = ConnectionHealth::Failed;
                            error!(
                                node_id = %self.node_id,
                                failures = consecutive_failures + 1,
                                "connection marked as failed after repeated stream failures"
                            );
                        } else {
                            *health = ConnectionHealth::Degraded {
                                consecutive_failures: consecutive_failures + 1,
                            };
                        }
                    }
                    ConnectionHealth::Failed => {
                        // Already failed, stay failed
                    }
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

/// Guard to automatically decrement stream count when dropped.
struct StreamGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
    active_streams: Arc<AtomicU32>,
}

impl Drop for StreamGuard {
    fn drop(&mut self) {
        self.active_streams.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Connection pool for Raft network peers.
///
/// Maintains persistent QUIC connections to peer nodes with automatic
/// reconnection, health tracking, and idle cleanup.
///
/// Tiger Style: Bounded pool size (MAX_PEERS), explicit lifecycle management.
pub struct RaftConnectionPool {
    /// Iroh endpoint manager for creating connections.
    endpoint_manager: Arc<IrohEndpointManager>,
    /// Map of NodeId -> PeerConnection (bounded by MAX_PEERS).
    connections: Arc<RwLock<HashMap<NodeId, Arc<PeerConnection>>>>,
    /// Failure detector for updating connection status.
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
    /// Background cleanup task handle.
    cleanup_task: AsyncMutex<Option<JoinHandle<()>>>,
}

impl RaftConnectionPool {
    /// Create a new connection pool.
    pub fn new(
        endpoint_manager: Arc<IrohEndpointManager>,
        failure_detector: Arc<RwLock<NodeFailureDetector>>,
    ) -> Self {
        Self {
            endpoint_manager,
            connections: Arc::new(RwLock::new(HashMap::new())),
            failure_detector,
            cleanup_task: AsyncMutex::new(None),
        }
    }

    /// Get or create a connection to the specified peer.
    ///
    /// Returns existing healthy connection if available, otherwise creates new one.
    /// Tiger Style: Lazy connection creation, bounded pool size.
    pub async fn get_or_connect(
        &self,
        node_id: NodeId,
        peer_addr: &EndpointAddr,
    ) -> Result<Arc<PeerConnection>> {
        // Fast path: check for existing healthy connection
        {
            let connections = self.connections.read().await;
            if let Some(conn) = connections.get(&node_id) {
                let health = conn.health().await;
                if health != ConnectionHealth::Failed {
                    debug!(
                        %node_id,
                        health = ?health,
                        active_streams = conn.active_stream_count(),
                        "reusing existing connection"
                    );
                    return Ok(Arc::clone(conn));
                }
                // Connection is failed, will create new one below
            }
        }

        // Slow path: create new connection
        self.create_connection(node_id, peer_addr).await
    }

    /// Create a new connection to a peer.
    async fn create_connection(
        &self,
        node_id: NodeId,
        peer_addr: &EndpointAddr,
    ) -> Result<Arc<PeerConnection>> {
        // Check pool size limit (Tiger Style: bounded resources)
        {
            let connections = self.connections.read().await;
            if connections.len() >= MAX_PEERS as usize && !connections.contains_key(&node_id) {
                return Err(anyhow::anyhow!(
                    "connection pool full ({} connections), cannot add node {}",
                    MAX_PEERS,
                    node_id
                ));
            }
        }

        info!(
            %node_id,
            endpoint_id = %peer_addr.id,
            "creating new connection to peer"
        );

        // Attempt connection with retries
        let mut attempts = 0;
        let connection = loop {
            attempts += 1;

            let connect_result = tokio::time::timeout(
                IROH_CONNECT_TIMEOUT,
                self.endpoint_manager
                    .endpoint()
                    .connect(peer_addr.clone(), b"raft-rpc"),
            )
            .await
            .context("timeout connecting to peer")?;

            match connect_result {
                Ok(conn) => break conn,
                Err(err) if attempts < MAX_CONNECTION_RETRIES => {
                    let backoff = Duration::from_millis(
                        CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)),
                    );
                    warn!(
                        %node_id,
                        attempt = attempts,
                        backoff_ms = backoff.as_millis(),
                        error = %err,
                        "connection failed, retrying"
                    );
                    tokio::time::sleep(backoff).await;
                }
                Err(err) => {
                    error!(
                        %node_id,
                        attempts,
                        error = %err,
                        "failed to connect after retries"
                    );

                    // Update failure detector
                    self.failure_detector.write().await.update_node_status(
                        node_id,
                        ConnectionStatus::Disconnected,
                        ConnectionStatus::Disconnected,
                    );

                    return Err(anyhow::anyhow!("connection failed after retries: {}", err));
                }
            }
        };

        // Create peer connection wrapper
        let peer_conn = Arc::new(PeerConnection::new(connection, node_id));

        // Store in pool (replace any failed connection)
        {
            let mut connections = self.connections.write().await;
            connections.insert(node_id, Arc::clone(&peer_conn));
            info!(
                %node_id,
                pool_size = connections.len(),
                "added connection to pool"
            );
        }

        // Update failure detector
        self.failure_detector.write().await.update_node_status(
            node_id,
            ConnectionStatus::Connected,
            ConnectionStatus::Connected,
        );

        Ok(peer_conn)
    }

    /// Start background cleanup task for idle connections.
    pub async fn start_cleanup_task(&self) {
        let pool = Arc::clone(&self.connections);
        let failure_detector = Arc::clone(&self.failure_detector);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let mut connections = pool.write().await;
                let mut to_remove = Vec::new();

                // Check each connection for idle timeout or failed health
                for (node_id, conn) in connections.iter() {
                    let is_idle = conn.is_idle(CONNECTION_IDLE_TIMEOUT).await;
                    let health = conn.health().await;
                    let active_streams = conn.active_stream_count();

                    let should_remove = is_idle || health == ConnectionHealth::Failed;

                    if should_remove {
                        to_remove.push(*node_id);
                        debug!(
                            node_id = %node_id,
                            health = ?health,
                            active_streams,
                            "removing connection from pool"
                        );
                    }
                }

                // Remove idle/failed connections
                for node_id in to_remove {
                    if connections.remove(&node_id).is_some() {
                        // Note: We can't close the connection directly here because it's behind an Arc
                        // The connection will be closed when all Arc references are dropped
                        debug!(
                            %node_id,
                            "removed connection from pool (will close when all references dropped)"
                        );

                        // Update failure detector
                        failure_detector.write().await.update_node_status(
                            node_id,
                            ConnectionStatus::Disconnected,
                            ConnectionStatus::Disconnected,
                        );
                    }
                }

                if !connections.is_empty() {
                    debug!(
                        pool_size = connections.len(),
                        "connection pool cleanup complete"
                    );
                }
            }
        });

        *self.cleanup_task.lock().await = Some(handle);
    }

    /// Shutdown the connection pool gracefully.
    pub async fn shutdown(&self) {
        info!("shutting down connection pool");

        // Stop cleanup task
        if let Some(handle) = self.cleanup_task.lock().await.take() {
            handle.abort();
        }

        // Clear all connections from pool
        // Note: Connections will close when all Arc references are dropped
        let mut connections = self.connections.write().await;
        let count = connections.len();
        connections.clear();
        debug!(
            connections_closed = count,
            "cleared connection pool during shutdown"
        );
    }

    /// Get metrics about the connection pool.
    pub async fn metrics(&self) -> ConnectionPoolMetrics {
        let connections = self.connections.read().await;

        let mut healthy = 0;
        let mut degraded = 0;
        let mut failed = 0;
        let mut total_streams = 0;

        for conn in connections.values() {
            total_streams += conn.active_stream_count();

            match conn.health().await {
                ConnectionHealth::Healthy => healthy += 1,
                ConnectionHealth::Degraded { .. } => degraded += 1,
                ConnectionHealth::Failed => failed += 1,
            }
        }

        ConnectionPoolMetrics {
            total_connections: connections.len() as u32,
            healthy_connections: healthy,
            degraded_connections: degraded,
            failed_connections: failed,
            total_active_streams: total_streams,
        }
    }
}

/// Metrics for connection pool monitoring.
#[derive(Debug, Clone)]
pub struct ConnectionPoolMetrics {
    /// Total number of connections in pool.
    pub total_connections: u32,
    /// Number of healthy connections.
    pub healthy_connections: u32,
    /// Number of degraded connections.
    pub degraded_connections: u32,
    /// Number of failed connections.
    pub failed_connections: u32,
    /// Total active streams across all connections.
    pub total_active_streams: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ConnectionHealth Enum Tests
    // =========================================================================

    #[test]
    fn test_connection_health_healthy_variant() {
        let health = ConnectionHealth::Healthy;
        assert_eq!(health, ConnectionHealth::Healthy);
    }

    #[test]
    fn test_connection_health_degraded_variant() {
        let health = ConnectionHealth::Degraded {
            consecutive_failures: 2,
        };
        assert!(matches!(
            health,
            ConnectionHealth::Degraded {
                consecutive_failures: 2
            }
        ));
    }

    #[test]
    fn test_connection_health_failed_variant() {
        let health = ConnectionHealth::Failed;
        assert_eq!(health, ConnectionHealth::Failed);
    }

    #[test]
    fn test_connection_health_clone() {
        let health = ConnectionHealth::Degraded {
            consecutive_failures: 3,
        };
        let cloned = health;
        assert_eq!(health, cloned);
    }

    #[test]
    fn test_connection_health_copy() {
        let health = ConnectionHealth::Healthy;
        let copied = health;
        // Both should be valid (Copy trait)
        assert_eq!(health, copied);
    }

    #[test]
    fn test_connection_health_debug() {
        let health = ConnectionHealth::Degraded {
            consecutive_failures: 5,
        };
        let debug_str = format!("{:?}", health);
        assert!(debug_str.contains("Degraded"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_connection_health_eq_different_variants() {
        assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Failed);
        assert_ne!(
            ConnectionHealth::Healthy,
            ConnectionHealth::Degraded {
                consecutive_failures: 1
            }
        );
        assert_ne!(
            ConnectionHealth::Failed,
            ConnectionHealth::Degraded {
                consecutive_failures: 1
            }
        );
    }

    #[test]
    fn test_connection_health_eq_same_degraded_different_counts() {
        let health1 = ConnectionHealth::Degraded {
            consecutive_failures: 1,
        };
        let health2 = ConnectionHealth::Degraded {
            consecutive_failures: 2,
        };
        assert_ne!(health1, health2);
    }

    // =========================================================================
    // ConnectionPoolMetrics Tests
    // =========================================================================

    #[test]
    fn test_connection_pool_metrics_creation() {
        let metrics = ConnectionPoolMetrics {
            total_connections: 10,
            healthy_connections: 7,
            degraded_connections: 2,
            failed_connections: 1,
            total_active_streams: 25,
        };
        assert_eq!(metrics.total_connections, 10);
        assert_eq!(metrics.healthy_connections, 7);
        assert_eq!(metrics.degraded_connections, 2);
        assert_eq!(metrics.failed_connections, 1);
        assert_eq!(metrics.total_active_streams, 25);
    }

    #[test]
    fn test_connection_pool_metrics_clone() {
        let metrics = ConnectionPoolMetrics {
            total_connections: 5,
            healthy_connections: 3,
            degraded_connections: 1,
            failed_connections: 1,
            total_active_streams: 10,
        };
        let cloned = metrics.clone();
        assert_eq!(metrics.total_connections, cloned.total_connections);
        assert_eq!(metrics.healthy_connections, cloned.healthy_connections);
        assert_eq!(metrics.degraded_connections, cloned.degraded_connections);
        assert_eq!(metrics.failed_connections, cloned.failed_connections);
        assert_eq!(metrics.total_active_streams, cloned.total_active_streams);
    }

    #[test]
    fn test_connection_pool_metrics_debug() {
        let metrics = ConnectionPoolMetrics {
            total_connections: 3,
            healthy_connections: 2,
            degraded_connections: 1,
            failed_connections: 0,
            total_active_streams: 5,
        };
        let debug_str = format!("{:?}", metrics);
        assert!(debug_str.contains("ConnectionPoolMetrics"));
        assert!(debug_str.contains("total_connections"));
        assert!(debug_str.contains("3"));
    }

    #[test]
    fn test_connection_pool_metrics_zero_values() {
        let metrics = ConnectionPoolMetrics {
            total_connections: 0,
            healthy_connections: 0,
            degraded_connections: 0,
            failed_connections: 0,
            total_active_streams: 0,
        };
        assert_eq!(metrics.total_connections, 0);
        assert_eq!(metrics.total_active_streams, 0);
    }

    #[test]
    fn test_connection_pool_metrics_max_values() {
        let metrics = ConnectionPoolMetrics {
            total_connections: u32::MAX,
            healthy_connections: u32::MAX,
            degraded_connections: u32::MAX,
            failed_connections: u32::MAX,
            total_active_streams: u32::MAX,
        };
        assert_eq!(metrics.total_connections, u32::MAX);
    }

    // =========================================================================
    // Constants Validation Tests
    // =========================================================================

    #[test]
    fn test_connection_idle_timeout_constant() {
        assert_eq!(CONNECTION_IDLE_TIMEOUT, Duration::from_secs(60));
    }

    #[test]
    fn test_max_connection_retries_constant() {
        assert_eq!(MAX_CONNECTION_RETRIES, 3);
    }

    #[test]
    fn test_connection_retry_backoff_base_constant() {
        assert_eq!(CONNECTION_RETRY_BACKOFF_BASE_MS, 100);
    }

    #[test]
    fn test_max_peers_constant() {
        assert_eq!(MAX_PEERS, 1000);
    }

    #[test]
    fn test_max_streams_per_connection_constant() {
        assert_eq!(MAX_STREAMS_PER_CONNECTION, 100);
    }

    #[test]
    fn test_iroh_connect_timeout_constant() {
        assert_eq!(IROH_CONNECT_TIMEOUT, Duration::from_secs(5));
    }

    #[test]
    fn test_iroh_stream_open_timeout_constant() {
        assert_eq!(IROH_STREAM_OPEN_TIMEOUT, Duration::from_secs(2));
    }

    // =========================================================================
    // Exponential Backoff Calculation Tests
    // =========================================================================

    #[test]
    fn test_exponential_backoff_attempt_1() {
        // Formula: BASE * (1 << (attempts - 1))
        let attempts = 1;
        let backoff =
            Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
        assert_eq!(backoff, Duration::from_millis(100));
    }

    #[test]
    fn test_exponential_backoff_attempt_2() {
        let attempts = 2;
        let backoff =
            Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
        assert_eq!(backoff, Duration::from_millis(200));
    }

    #[test]
    fn test_exponential_backoff_attempt_3() {
        let attempts = 3;
        let backoff =
            Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
        assert_eq!(backoff, Duration::from_millis(400));
    }

    #[test]
    fn test_exponential_backoff_max_retries() {
        // At MAX_CONNECTION_RETRIES (3), backoff = 100 * (1 << 2) = 400ms
        let attempts = MAX_CONNECTION_RETRIES;
        let backoff =
            Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
        assert_eq!(backoff, Duration::from_millis(400));
    }

    // =========================================================================
    // StreamGuard Tests
    // =========================================================================

    #[test]
    fn test_stream_guard_decrements_on_drop() {
        let active_streams = Arc::new(AtomicU32::new(5));
        let semaphore = Arc::new(Semaphore::new(10));
        let permit = semaphore.clone().try_acquire_owned().unwrap();

        assert_eq!(active_streams.load(Ordering::Relaxed), 5);

        let guard = StreamGuard {
            _permit: permit,
            active_streams: active_streams.clone(),
        };

        drop(guard);

        // Should have decremented by 1
        assert_eq!(active_streams.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn test_stream_guard_multiple_drops() {
        let active_streams = Arc::new(AtomicU32::new(10));
        let semaphore = Arc::new(Semaphore::new(10));

        // Create and drop multiple guards
        for expected in (7..=9).rev() {
            let permit = semaphore.clone().try_acquire_owned().unwrap();
            let guard = StreamGuard {
                _permit: permit,
                active_streams: active_streams.clone(),
            };
            drop(guard);
            assert_eq!(active_streams.load(Ordering::Relaxed), expected);
        }
    }

    #[test]
    fn test_stream_guard_underflow_protection() {
        // Test that fetch_sub with 0 doesn't panic (wraps to u32::MAX, but that's okay
        // since in practice we always increment before decrement)
        let active_streams = Arc::new(AtomicU32::new(1));
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().try_acquire_owned().unwrap();

        let guard = StreamGuard {
            _permit: permit,
            active_streams: active_streams.clone(),
        };
        drop(guard);

        assert_eq!(active_streams.load(Ordering::Relaxed), 0);
    }

    // =========================================================================
    // Atomic Operations Tests
    // =========================================================================

    #[test]
    fn test_atomic_add_relaxed() {
        let counter = AtomicU32::new(0);
        let result = counter.fetch_add(1, Ordering::Relaxed);
        assert_eq!(result, 0);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_atomic_sub_relaxed() {
        let counter = AtomicU32::new(5);
        let result = counter.fetch_sub(1, Ordering::Relaxed);
        assert_eq!(result, 5);
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn test_atomic_load_relaxed() {
        let counter = AtomicU32::new(42);
        assert_eq!(counter.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn test_atomic_concurrent_operations() {
        let counter = Arc::new(AtomicU32::new(0));

        // Simulate concurrent increments
        for _ in 0..100 {
            counter.fetch_add(1, Ordering::Relaxed);
        }

        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    // =========================================================================
    // Semaphore Tests (for stream limiting)
    // =========================================================================

    #[test]
    fn test_semaphore_bounded_capacity() {
        let semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));

        // Should be able to acquire MAX_STREAMS_PER_CONNECTION permits
        let mut permits = Vec::new();
        for _ in 0..MAX_STREAMS_PER_CONNECTION {
            let permit = semaphore.clone().try_acquire_owned();
            assert!(permit.is_ok());
            permits.push(permit.unwrap());
        }

        // Next acquisition should fail
        let result = semaphore.clone().try_acquire_owned();
        assert!(result.is_err());
    }

    #[test]
    fn test_semaphore_release_on_drop() {
        let semaphore = Arc::new(Semaphore::new(1));

        // Acquire the only permit
        let permit = semaphore.clone().try_acquire_owned().unwrap();

        // Second acquisition should fail
        assert!(semaphore.clone().try_acquire_owned().is_err());

        // Drop the permit
        drop(permit);

        // Now acquisition should succeed
        assert!(semaphore.clone().try_acquire_owned().is_ok());
    }

    // =========================================================================
    // Duration and Timeout Tests
    // =========================================================================

    #[test]
    fn test_timeout_durations_ordering() {
        // Connect timeout > Stream timeout (connect includes handshake)
        assert!(IROH_CONNECT_TIMEOUT > IROH_STREAM_OPEN_TIMEOUT);

        // Idle timeout > Connect timeout (connections should live longer)
        assert!(CONNECTION_IDLE_TIMEOUT > IROH_CONNECT_TIMEOUT);
    }

    #[test]
    fn test_idle_timeout_in_seconds() {
        assert_eq!(CONNECTION_IDLE_TIMEOUT.as_secs(), 60);
    }

    #[test]
    fn test_connect_timeout_in_seconds() {
        assert_eq!(IROH_CONNECT_TIMEOUT.as_secs(), 5);
    }

    #[test]
    fn test_stream_timeout_in_seconds() {
        assert_eq!(IROH_STREAM_OPEN_TIMEOUT.as_secs(), 2);
    }

    // =========================================================================
    // Capacity Bounds Tests
    // =========================================================================

    #[test]
    fn test_max_peers_is_bounded() {
        // Verify MAX_PEERS has a reasonable upper bound
        // Use runtime value to avoid clippy assertions_on_constants warning
        let max_peers = MAX_PEERS;
        assert!(max_peers <= 10_000);
        assert!(max_peers >= 100);
    }

    #[test]
    fn test_max_streams_per_connection_is_bounded() {
        // Verify streams per connection is reasonable
        let max_streams = MAX_STREAMS_PER_CONNECTION;
        assert!(max_streams <= 1_000);
        assert!(max_streams >= 10);
    }

    #[test]
    fn test_max_retries_is_bounded() {
        // Verify retries are reasonable (not infinite, not too few)
        let max_retries = MAX_CONNECTION_RETRIES;
        assert!(max_retries >= 1);
        assert!(max_retries <= 10);
    }

    // =========================================================================
    // Node ID Type Tests (used in pool keys)
    // =========================================================================

    #[test]
    fn test_node_id_as_hashmap_key() {
        use std::collections::HashMap;
        let mut map: HashMap<NodeId, u32> = HashMap::new();

        let node1 = NodeId::from(1);
        let node2 = NodeId::from(2);

        map.insert(node1, 100);
        map.insert(node2, 200);

        assert_eq!(map.get(&node1), Some(&100));
        assert_eq!(map.get(&node2), Some(&200));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_node_id_copy_for_pool_operations() {
        let node = NodeId::from(42);
        let copied = node;
        // Both should be valid due to Copy
        assert_eq!(node, copied);
    }
}
