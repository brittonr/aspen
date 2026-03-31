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
    /// Total Raft-priority (critical) streams opened across all connections.
    pub raft_streams_opened: u32,
    /// Total bulk-priority streams opened across all connections.
    pub bulk_streams_opened: u32,
    /// Total ReadIndex retry attempts.
    pub read_index_retry_count: u64,
    /// Total ReadIndex operations that succeeded after retrying.
    pub read_index_retry_success_count: u64,
}
