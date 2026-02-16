//! Connection pool management: get/create connections, cleanup, shutdown, metrics.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::NetworkTransport;
use iroh::EndpointAddr;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::CONNECTION_IDLE_TIMEOUT;
use super::CONNECTION_RETRY_BACKOFF_BASE_MS;
use super::ConnectionHealth;
use super::ConnectionPoolMetrics;
use super::MAX_CONNECTION_RETRIES;
use super::PeerConnection;
use super::RaftConnectionPool;
use crate::constants::IROH_CONNECT_TIMEOUT;
use crate::constants::MAX_PEERS;
use crate::node_failure_detection::ConnectionStatus;
use crate::types::NodeId;

impl<T> RaftConnectionPool<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    /// Get or create a connection to the specified peer.
    ///
    /// Returns existing healthy connection if available, otherwise creates new one.
    /// Tiger Style: Lazy connection creation, bounded pool size.
    pub async fn get_or_connect(&self, node_id: NodeId, peer_addr: &EndpointAddr) -> Result<Arc<PeerConnection>> {
        // Tiger Style: node_id must be valid (non-zero for most Raft implementations)
        debug_assert!(node_id.0 > 0, "POOL: node_id must be positive, got 0");

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
    async fn create_connection(&self, node_id: NodeId, peer_addr: &EndpointAddr) -> Result<Arc<PeerConnection>> {
        self.create_connection_check_pool_capacity(node_id).await?;

        let alpn = self.create_connection_select_alpn();
        self.create_connection_log_attempt(node_id, peer_addr, alpn);

        let connection = self.create_connection_with_retries(node_id, peer_addr, alpn).await?;
        let peer_conn = Arc::new(PeerConnection::new(connection, node_id));

        self.create_connection_store_in_pool(node_id, &peer_conn).await;
        self.create_connection_update_failure_detector(node_id, ConnectionStatus::Connected).await;

        Ok(peer_conn)
    }

    /// Check pool capacity before creating a new connection.
    async fn create_connection_check_pool_capacity(&self, node_id: NodeId) -> Result<()> {
        let connections = self.connections.read().await;
        let is_at_capacity = connections.len() >= MAX_PEERS as usize;
        let is_new_key = !connections.contains_key(&node_id);
        if is_at_capacity && is_new_key {
            return Err(anyhow::anyhow!(
                "connection pool full ({} connections), cannot add node {}",
                MAX_PEERS,
                node_id
            ));
        }
        Ok(())
    }

    /// Select ALPN protocol based on authentication configuration.
    #[allow(deprecated)]
    fn create_connection_select_alpn(&self) -> &'static [u8] {
        if self.use_auth_alpn {
            aspen_transport::RAFT_AUTH_ALPN
        } else {
            // SECURITY WARNING: Using unauthenticated Raft ALPN
            // This is retained for backward compatibility during migration
            tracing::warn!(
                target: "aspen_raft::security",
                "using unauthenticated RAFT_ALPN - enable auth for production"
            );
            aspen_transport::RAFT_ALPN
        }
    }

    /// Log connection attempt details.
    fn create_connection_log_attempt(&self, node_id: NodeId, peer_addr: &EndpointAddr, alpn: &[u8]) {
        info!(
            %node_id,
            endpoint_id = %peer_addr.id,
            alpn = ?std::str::from_utf8(alpn).unwrap_or("unknown"),
            use_auth = self.use_auth_alpn,
            "creating new connection to peer"
        );
    }

    /// Attempt connection with retries and exponential backoff.
    async fn create_connection_with_retries(
        &self,
        node_id: NodeId,
        peer_addr: &EndpointAddr,
        alpn: &'static [u8],
    ) -> Result<iroh::endpoint::Connection> {
        use crate::verified::calculate_connection_retry_backoff;

        let mut attempts = 0;
        loop {
            attempts += 1;

            let connect_result =
                tokio::time::timeout(IROH_CONNECT_TIMEOUT, self.transport.endpoint().connect(peer_addr.clone(), alpn))
                    .await
                    .context("timeout connecting to peer")?;

            match connect_result {
                Ok(conn) => return Ok(conn),
                Err(err) if attempts < MAX_CONNECTION_RETRIES => {
                    let backoff = calculate_connection_retry_backoff(attempts, CONNECTION_RETRY_BACKOFF_BASE_MS);
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
                    self.create_connection_update_failure_detector(node_id, ConnectionStatus::Disconnected).await;
                    return Err(anyhow::anyhow!("connection failed after retries: {}", err));
                }
            }
        }
    }

    /// Store the new connection in the pool.
    async fn create_connection_store_in_pool(&self, node_id: NodeId, peer_conn: &Arc<PeerConnection>) {
        let mut connections = self.connections.write().await;
        connections.insert(node_id, Arc::clone(peer_conn));

        // Tiger Style: pool size must not exceed MAX_PEERS
        debug_assert!(
            connections.len() <= MAX_PEERS as usize,
            "POOL: pool size {} exceeds MAX_PEERS {}",
            connections.len(),
            MAX_PEERS
        );

        info!(
            %node_id,
            pool_size = connections.len(),
            "added connection to pool"
        );
    }

    /// Update failure detector with connection status.
    async fn create_connection_update_failure_detector(&self, node_id: NodeId, status: ConnectionStatus) {
        self.failure_detector.write().await.update_node_status(node_id, status, status);
    }

    /// Start background cleanup task for idle connections.
    ///
    /// Tiger Style: Avoids holding write lock while awaiting async checks.
    /// Instead, collects connection Arcs under read lock, processes without lock,
    /// then acquires write lock only for removal.
    pub async fn start_cleanup_task(&self) {
        let pool = Arc::clone(&self.connections);
        let failure_detector = Arc::clone(&self.failure_detector);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Phase 1: Collect connection Arcs under read lock (no awaits)
                // Tiger Style: Minimize lock hold time by avoiding awaits under lock
                let candidates: Vec<(NodeId, Arc<PeerConnection>)> = {
                    let connections = pool.read().await;
                    connections.iter().map(|(id, conn)| (*id, Arc::clone(conn))).collect()
                };

                // Phase 2: Check each connection WITHOUT holding the pool lock
                // This allows other operations to proceed while we await on each connection
                let mut to_remove = Vec::new();
                for (node_id, conn) in candidates {
                    let is_idle = conn.is_idle(CONNECTION_IDLE_TIMEOUT).await;
                    let health = conn.health().await;
                    let active_streams = conn.active_stream_count();

                    let should_remove = is_idle || health == ConnectionHealth::Failed;

                    if should_remove {
                        to_remove.push(node_id);
                        debug!(
                            node_id = %node_id,
                            health = ?health,
                            active_streams,
                            "marking connection for removal from pool"
                        );
                    }
                }

                // Phase 3: Acquire write lock only for removal (quick operation)
                if !to_remove.is_empty() {
                    let mut connections = pool.write().await;
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

                    debug!(pool_size = connections.len(), "connection pool cleanup complete");
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
        debug!(connections_closed = count, "cleared connection pool during shutdown");
    }

    /// Get metrics about the connection pool.
    pub async fn metrics(&self) -> ConnectionPoolMetrics {
        let connections = self.connections.read().await;

        let mut healthy = 0u32;
        let mut degraded = 0u32;
        let mut failed = 0u32;
        let mut total_streams = 0u32;

        for conn in connections.values() {
            total_streams = total_streams.saturating_add(conn.active_stream_count());

            match conn.health().await {
                ConnectionHealth::Healthy => healthy += 1,
                ConnectionHealth::Degraded { .. } => degraded += 1,
                ConnectionHealth::Failed => failed += 1,
            }
        }

        let total_connections = connections.len() as u32;

        // Tiger Style: metrics invariants
        debug_assert!(
            healthy + degraded + failed == total_connections,
            "POOL: health counts ({} + {} + {} = {}) must equal total ({})",
            healthy,
            degraded,
            failed,
            healthy + degraded + failed,
            total_connections
        );
        debug_assert!(
            total_connections <= MAX_PEERS,
            "POOL: total_connections {} exceeds MAX_PEERS {}",
            total_connections,
            MAX_PEERS
        );

        ConnectionPoolMetrics {
            total_connections,
            healthy_connections: healthy,
            degraded_connections: degraded,
            failed_connections: failed,
            total_active_streams: total_streams,
        }
    }
}
