//! Centralized constants for Raft network and RPC operations.
//!
//! This module contains constants used by the Raft RPC layer that are
//! independent of the main Aspen crate. For application-level constants
//! (key sizes, batch limits, etc.), see `aspen-core::constants`.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.

use std::time::Duration;

// ============================================================================
// Network Constants
// ============================================================================

/// Maximum size for RPC messages (10 MB).
///
/// Tiger Style: Fixed limit to prevent unbounded memory use during RPC serialization
/// and deserialization. Applied to both request and response payloads.
///
/// Used in:
/// - `network.rs`: Message reading with `read_to_end(MAX_RPC_MESSAGE_SIZE)`
/// - `server.rs`: RPC message deserialization from streams
pub const MAX_RPC_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// Timeout for Iroh connection establishment (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs on unreachable peers.
/// Applied when initiating peer connections.
///
/// Used in:
/// - `network.rs`: `endpoint.connect()` with timeout wrapper
pub const IROH_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for bidirectional stream open (2 seconds).
///
/// Tiger Style: Bounded wait for stream establishment after connection succeeds.
/// Prevents indefinite blocking during stream initialization.
///
/// Used in:
/// - `network.rs`: `connection.open_bi()` with timeout wrapper
pub const IROH_STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(2);

/// Timeout for RPC response read (10 seconds).
///
/// Accounts for slow snapshot transfers and disk I/O from the peer.
/// Tiger Style: Prevents indefinite blocking on slow or stalled peers.
/// Much higher than connect/stream timeouts due to variable snapshot sizes.
///
/// Used in:
/// - `network.rs`: `recv_stream.read_to_end()` with timeout wrapper
pub const IROH_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum snapshot size (100 MB).
///
/// Tiger Style: Fixed limit prevents unbounded memory allocation from malicious
/// or corrupt snapshots. Prevents DoS attacks via large snapshot payloads.
///
/// Used in:
/// - `network.rs`: Chunked snapshot reading with size validation
pub const MAX_SNAPSHOT_SIZE: u64 = 100 * 1024 * 1024;

// ============================================================================
// Peer and Connection Constants
// ============================================================================

/// Maximum number of peers in the peer map (1000).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from peer map growth.
/// Applied in the network factory to prevent Sybil attacks.
///
/// Used in:
/// - `network.rs`: Peer map bounds
pub const MAX_PEERS: u32 = 1000;

/// Maximum number of concurrent connections (500).
///
/// Tiger Style: Fixed limit prevents connection exhaustion attacks.
/// Applied in the RPC server to limit total concurrent connections.
///
/// Used in:
/// - `server.rs`: Connection acceptance limits
pub const MAX_CONCURRENT_CONNECTIONS: u32 = 500;

/// Maximum number of concurrent streams per connection (100).
///
/// Tiger Style: Fixed limit prevents DoS attacks via unbounded stream creation.
/// Applied in the RPC server to limit streams from any single peer.
///
/// Used in:
/// - `server.rs`: Stream acceptance limits
pub const MAX_STREAMS_PER_CONNECTION: u32 = 100;

// ============================================================================
// Timeout and Hanging Prevention Constants
// ============================================================================

/// Timeout for ReadIndex linearizability check (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs when leader is unavailable.
/// Applied to all ReadIndex `await_ready()` calls to ensure bounded wait times.
///
/// Used in:
/// - `node.rs`: KeyValueStore::read(), KeyValueStore::scan(), SqlQueryExecutor::execute_sql()
pub const READ_INDEX_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for cluster membership operations (30 seconds).
///
/// Tiger Style: Explicit timeout prevents hangs during partition events.
/// Applied to init(), add_learner(), and change_membership() operations.
/// Membership operations may require multiple round trips and quorum confirmation.
///
/// Used in:
/// - `node.rs`: ClusterController::init(), add_learner(), change_membership()
pub const MEMBERSHIP_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for snapshot installation per segment (5000 milliseconds).
///
/// Tiger Style: Explicit timeout prevents hangs during large snapshot transfers.
/// Default OpenRaft value of 200ms is too short for production snapshots.
/// 5 seconds allows for 100MB snapshots at ~20MB/s transfer rate.
///
/// Used in:
/// - `bootstrap.rs`: RaftConfig::install_snapshot_timeout
pub const SNAPSHOT_INSTALL_TIMEOUT_MS: u64 = 5000;

/// Minimum number of log entries between automatic snapshots.
///
/// Tiger Style: This is a safety floor. The actual configured value in
/// `openraft::SnapshotPolicy::LogsSinceLast(N)` must be >= this constant.
///
/// Regression: A threshold of 100 caused a race between the state machine's
/// eagerly-applied `last_applied` (updated during `append()`) and openraft's
/// `apply_progress` (updated during `apply()` callback). When the snapshot
/// fired at index 100 but apply_progress was at 99, openraft panicked with
/// `snapshot.submitted(100) > apply_progress.submitted(99)`, killing the
/// Raft core and making all operations return NOT_LEADER.
///
/// Used in:
/// - `aspen-cluster::bootstrap::node::storage_init` snapshot policy
/// - `aspen-cluster::bootstrap::node::sharding_init` snapshot policy
pub const MIN_SNAPSHOT_LOG_THRESHOLD: u64 = 1_000;

/// Capacity of failure detector update channel.
///
/// Tiger Style: Bounded channel prevents unbounded task spawning.
/// Used to batch failure detector updates from multiple concurrent RPC failures
/// through a single consumer task instead of spawning unbounded tasks.
///
/// Used in:
/// - `network.rs`: IrpcRaftNetworkFactory failure update channel
pub const FAILURE_DETECTOR_CHANNEL_CAPACITY: usize = 100;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Network message limits must be positive
const _: () = assert!(MAX_RPC_MESSAGE_SIZE > 0);
const _: () = assert!(MAX_SNAPSHOT_SIZE > 0);

// Snapshots can be larger than single RPC messages (chunked transfer)
const _: () = assert!(MAX_SNAPSHOT_SIZE > MAX_RPC_MESSAGE_SIZE as u64);

// Connection limits must be positive
const _: () = assert!(MAX_PEERS > 0);
const _: () = assert!(MAX_CONCURRENT_CONNECTIONS > 0);
const _: () = assert!(MAX_STREAMS_PER_CONNECTION > 0);

// Concurrent connections should not exceed total peers (sanity check)
const _: () = assert!(MAX_CONCURRENT_CONNECTIONS <= MAX_PEERS);

// Failure detector channel must have capacity
const _: () = assert!(FAILURE_DETECTOR_CHANNEL_CAPACITY > 0);

// Snapshot install timeout must be positive
const _: () = assert!(SNAPSHOT_INSTALL_TIMEOUT_MS > 0);

// Snapshot threshold must be >= safety floor
const _: () = assert!(MIN_SNAPSHOT_LOG_THRESHOLD >= 1_000);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_ordering() {
        // Connect timeout should be less than read timeout
        assert!(IROH_CONNECT_TIMEOUT < IROH_READ_TIMEOUT);

        // Stream timeout should be less than connect timeout
        assert!(IROH_STREAM_OPEN_TIMEOUT < IROH_CONNECT_TIMEOUT);

        // ReadIndex should be reasonable relative to read timeout
        assert!(READ_INDEX_TIMEOUT <= IROH_READ_TIMEOUT);

        // Membership operations take longer due to consensus rounds
        assert!(MEMBERSHIP_OPERATION_TIMEOUT > READ_INDEX_TIMEOUT);
    }

    /// Regression: LogsSinceLast(100) triggered a snapshot race that panicked
    /// the Raft core. The threshold must stay above the safety floor.
    #[test]
    fn test_snapshot_threshold_above_safety_floor() {
        assert!(
            MIN_SNAPSHOT_LOG_THRESHOLD >= 1_000,
            "snapshot threshold must be >= 1000 to avoid the append/apply race \
             (see napkin: 2026-02-26 snapshot race)"
        );
    }
}
