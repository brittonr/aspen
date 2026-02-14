//! Connection pool metrics for monitoring.

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
