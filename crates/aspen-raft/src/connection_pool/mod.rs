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
//! # Iroh-Native Authentication
//!
//! Authentication is handled by Iroh at the QUIC TLS layer:
//! - NodeId is cryptographically verified during connection establishment
//! - No per-stream authentication handshake is needed
//! - Server validates NodeId against trusted peers registry at accept time
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
//! Unit tests in this module (mod tests) cover:
//! - ConnectionHealth enum variants, equality, Copy/Clone, Debug
//! - ConnectionPoolMetrics struct fields, clone, debug, edge cases
//! - Tiger Style resource bound constants validation
//! - Exponential backoff calculation pattern
//! - StreamGuard decrement-on-drop behavior
//! - Atomic operations correctness
//! - Semaphore bounded capacity and release
//! - Timeout duration ordering validation
//! - Capacity bounds verification
//!
//! Pure function tests in `src/raft/pure.rs` cover:
//! - Health state transitions (Healthy -> Degraded -> Failed)
//! - Connection retry backoff calculation (exponential)
//!
//! Additional tests in `tests/connection_pool_test.rs` cover:
//! - Integration-level ConnectionHealth and ConnectionPoolMetrics validation
//! - State machine validity verification

mod metrics;
pub(crate) mod peer_connection;
mod pool_management;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_core::NetworkTransport;
// Re-export ConnectionHealth from aspen-raft-types for backwards compatibility
pub use aspen_raft_types::network::ConnectionHealth;
pub use metrics::ConnectionPoolMetrics;
pub use peer_connection::PeerConnection;
pub use peer_connection::StreamHandle;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::node_failure_detection::NodeFailureDetector;
use crate::types::NodeId;

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

/// Connection pool for Raft network peers.
///
/// Maintains persistent QUIC connections to peer nodes with automatic
/// reconnection, health tracking, and idle cleanup.
///
/// Authentication is handled by Iroh at the QUIC TLS layer - no per-stream
/// authentication handshake is needed.
///
/// # Type Parameters
///
/// * `T` - Transport implementation that provides Iroh endpoint access. Must implement
///   `NetworkTransport` with Iroh-specific associated types.
///
/// Tiger Style: Bounded pool size (MAX_PEERS), explicit lifecycle management.
pub struct RaftConnectionPool<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>
{
    /// Transport providing endpoint access for creating connections.
    transport: Arc<T>,
    /// Map of NodeId -> PeerConnection (bounded by MAX_PEERS).
    connections: Arc<RwLock<HashMap<NodeId, Arc<PeerConnection>>>>,
    /// Failure detector for updating connection status.
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
    /// Background cleanup task handle.
    cleanup_task: AsyncMutex<Option<JoinHandle<()>>>,
    /// Whether to use RAFT_AUTH_ALPN instead of RAFT_ALPN for connections.
    ///
    /// When true, outgoing connections use `RAFT_AUTH_ALPN` (raft-auth) which
    /// requires the server to have authentication enabled. When false (default),
    /// uses legacy `RAFT_ALPN` (raft-rpc) for backward compatibility.
    use_auth_alpn: bool,
}

impl<T> RaftConnectionPool<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    /// Create a new connection pool.
    ///
    /// # Arguments
    ///
    /// * `transport` - Network transport providing endpoint access for creating connections
    /// * `failure_detector` - Failure detector for tracking node health
    /// * `use_auth_alpn` - When true, use `RAFT_AUTH_ALPN` for connections (requires auth server).
    ///   When false, use legacy `RAFT_ALPN` for backward compatibility.
    ///
    /// # Security Note
    ///
    /// When `use_auth_alpn` is true, outgoing connections use `RAFT_AUTH_ALPN` (raft-auth).
    /// The server must also be configured with `enable_raft_auth` to accept these connections.
    /// This provides authenticated connections for production deployments.
    ///
    /// When `use_auth_alpn` is false (default), connections use `RAFT_ALPN` (raft-rpc)
    /// for backward compatibility with nodes that don't have authentication enabled.
    pub fn new(transport: Arc<T>, failure_detector: Arc<RwLock<NodeFailureDetector>>, use_auth_alpn: bool) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            failure_detector,
            cleanup_task: AsyncMutex::new(None),
            use_auth_alpn,
        }
    }
}
