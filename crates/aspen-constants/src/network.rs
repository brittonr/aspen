//! Network constants for Aspen distributed system.
//!
//! This module contains constants for network timeouts, message sizes,
//! gossip protocol parameters, and connection limits.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//!
//! Note: Duration constants are expressed as primitive integer types (seconds, milliseconds)
//! to avoid dependencies. Use `Duration::from_secs()` or `Duration::from_millis()` when needed.

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

/// Timeout for Iroh connection establishment in seconds (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs on unreachable peers.
/// Applied when initiating peer connections.
///
/// Used in:
/// - `network.rs`: `endpoint.connect()` with timeout wrapper
pub const IROH_CONNECT_TIMEOUT_SECS: u64 = 5;

/// Timeout for bidirectional stream open in seconds (2 seconds).
///
/// Tiger Style: Bounded wait for stream establishment after connection succeeds.
/// Prevents indefinite blocking during stream initialization.
///
/// Used in:
/// - `network.rs`: `connection.open_bi()` with timeout wrapper
pub const IROH_STREAM_OPEN_TIMEOUT_SECS: u64 = 2;

/// Timeout for RPC response read in seconds (10 seconds).
///
/// Accounts for slow snapshot transfers and disk I/O from the peer.
/// Tiger Style: Prevents indefinite blocking on slow or stalled peers.
/// Much higher than connect/stream timeouts due to variable snapshot sizes.
///
/// Used in:
/// - `network.rs`: `recv_stream.read_to_end()` with timeout wrapper
pub const IROH_READ_TIMEOUT_SECS: u64 = 10;

/// Maximum snapshot size (100 MB).
///
/// Tiger Style: Fixed limit prevents unbounded memory allocation from malicious
/// or corrupt snapshots. Prevents DoS attacks via large snapshot payloads.
///
/// Used in:
/// - `network.rs`: Chunked snapshot reading with size validation
pub const MAX_SNAPSHOT_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum number of concurrent streams per connection (100).
///
/// Tiger Style: Fixed limit prevents DoS attacks via unbounded stream creation.
/// Applied in the RPC server to limit streams from any single peer.
///
/// Used in:
/// - `server.rs`: Stream acceptance limits
pub const MAX_STREAMS_PER_CONNECTION: u32 = 100;

/// Maximum number of concurrent connections (500).
///
/// Tiger Style: Fixed limit prevents connection exhaustion attacks.
/// Applied in the RPC server to limit total concurrent connections.
///
/// Used in:
/// - `server.rs`: Connection acceptance limits
pub const MAX_CONCURRENT_CONNECTIONS: u32 = 500;

/// Maximum number of peers in the peer map (1000).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from peer map growth.
/// Applied in the network factory to prevent Sybil attacks.
///
/// Used in:
/// - `network.rs`: Peer map bounds
pub const MAX_PEERS: u32 = 1000;

// ============================================================================
// Failure Detection Constants
// ============================================================================

/// Maximum number of tracked unreachable nodes (1000).
///
/// Tiger Style: Bounded storage for node failure tracking.
/// Prevents unbounded growth of failure detector state even if many nodes crash.
///
/// Used in:
/// - `node_failure_detection.rs`: NodeFailureDetector state management
pub const MAX_UNREACHABLE_NODES: u32 = 1000;

/// Maximum number of peers to track in gossip discovery (1000).
///
/// Tiger Style: Bounded storage for peer discovery state.
/// Prevents unbounded growth of peer list even in large clusters.
///
/// Used in:
/// - `gossip_actor.rs`: GossipActor peer management
pub const MAX_PEER_COUNT: u32 = 1000;

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
// Gossip Rate Limiting Constants (HIGH-6 Security Enhancement)
// ============================================================================

/// Maximum number of peers to track for rate limiting (256).
///
/// Tiger Style: Bounded LRU-style cache prevents unbounded memory growth.
/// Sufficient for tracking active gossipers while limiting memory to ~8KB.
/// Oldest entries are evicted when limit is reached.
///
/// Used in:
/// - `gossip_discovery.rs`: Per-peer rate limiter cache size
pub const GOSSIP_MAX_TRACKED_PEERS: usize = 256;

/// Per-peer gossip announcement rate limit (messages per minute).
///
/// Tiger Style: Fixed limit prevents individual peer abuse.
/// Allows normal operation (6/minute at 10s interval) with margin for retries.
/// Value of 12 allows 1 message every 5 seconds average.
///
/// Used in:
/// - `gossip_discovery.rs`: Per-peer rate limiter threshold
pub const GOSSIP_PER_PEER_RATE_PER_MINUTE: u32 = 12;

/// Per-peer gossip announcement burst capacity (messages).
///
/// Tiger Style: Allows brief bursts (e.g., reconnection) without blocking.
/// A peer can send up to this many messages before rate limiting kicks in.
///
/// Used in:
/// - `gossip_discovery.rs`: Per-peer burst allowance
pub const GOSSIP_PER_PEER_BURST: u32 = 3;

/// Global gossip announcement rate limit (messages per minute).
///
/// Tiger Style: Cluster-wide bandwidth protection for 1000+ peer scenarios.
/// With 1000 peers each sending 6/minute, worst case is 6000/minute.
/// Setting to 10,000 provides margin for bursts and network recovery.
///
/// Used in:
/// - `gossip_discovery.rs`: Global rate limiter threshold
pub const GOSSIP_GLOBAL_RATE_PER_MINUTE: u32 = 10_000;

/// Global gossip announcement burst capacity (messages).
///
/// Tiger Style: Handles cluster-wide startup bursts gracefully.
/// Allows brief spikes without immediate rejection.
///
/// Used in:
/// - `gossip_discovery.rs`: Global burst allowance
pub const GOSSIP_GLOBAL_BURST: u32 = 100;

// ============================================================================
// Gossip Error Recovery Constants
// ============================================================================

/// Maximum gossip stream error retries before giving up (5 retries).
///
/// Tiger Style: Bounded retry count prevents infinite retry loops.
/// After this many consecutive errors, the receiver task exits.
///
/// Used in:
/// - `gossip_discovery.rs`: Receiver task error handling
pub const GOSSIP_MAX_STREAM_RETRIES: u32 = 5;

/// Gossip stream error backoff durations in seconds.
///
/// Tiger Style: Fixed backoff progression: 1s, 2s, 4s, 8s, 16s (capped).
/// Exponential backoff prevents overwhelming a recovering network.
///
/// Used in:
/// - `gossip_discovery.rs`: Receiver task backoff calculation
pub const GOSSIP_STREAM_BACKOFF_SECS: [u64; 5] = [1, 2, 4, 8, 16];

// ============================================================================
// Gossip Announcer Rate Limiting Constants
// ============================================================================

/// Minimum interval between peer announcements in seconds (10 seconds).
///
/// Tiger Style: Fixed floor prevents announcement flooding.
/// This is the normal announcement interval when network is healthy.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task interval
pub const GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS: u64 = 10;

/// Maximum interval between peer announcements in seconds (60 seconds).
///
/// Tiger Style: Upper bound on backoff prevents stale discovery.
/// Used when announcements are failing to avoid network flooding.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task adaptive interval
pub const GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS: u64 = 60;

/// Consecutive announcement failures before increasing interval (3).
///
/// Tiger Style: Bounded failures before adaptive backoff.
/// Prevents flooding the network when broadcast consistently fails.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task adaptive logic
pub const GOSSIP_ANNOUNCE_FAILURE_THRESHOLD: u32 = 3;

// ============================================================================
// Hanging Prevention Timeout Constants
// ============================================================================

/// Timeout for ReadIndex linearizability check in seconds (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs when leader is unavailable.
/// Applied to all ReadIndex `await_ready()` calls to ensure bounded wait times.
///
/// Used in:
/// - `node.rs`: KeyValueStore::read(), KeyValueStore::scan(), SqlQueryExecutor::execute_sql()
pub const READ_INDEX_TIMEOUT_SECS: u64 = 5;

/// Timeout for cluster membership operations in seconds (30 seconds).
///
/// Tiger Style: Explicit timeout prevents hangs during partition events.
/// Applied to init(), add_learner(), and change_membership() operations.
/// Membership operations may require multiple round trips and quorum confirmation.
///
/// Used in:
/// - `node.rs`: ClusterController::init(), add_learner(), change_membership()
pub const MEMBERSHIP_OPERATION_TIMEOUT_SECS: u64 = 30;

/// Timeout for gossip subscription in seconds (10 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite blocking during subscription.
/// If gossip is unavailable, the node should continue without it (non-fatal).
///
/// Used in:
/// - `gossip_discovery.rs`: GossipPeerDiscovery::spawn()
pub const GOSSIP_SUBSCRIBE_TIMEOUT_SECS: u64 = 10;

/// Timeout for snapshot installation per segment in milliseconds (5000 milliseconds).
///
/// Tiger Style: Explicit timeout prevents hangs during large snapshot transfers.
/// Default OpenRaft value of 200ms is too short for production snapshots.
/// 5 seconds allows for 100MB snapshots at ~20MB/s transfer rate.
///
/// Used in:
/// - `bootstrap.rs`: RaftConfig::install_snapshot_timeout
pub const SNAPSHOT_INSTALL_TIMEOUT_MS: u64 = 5000;

// ============================================================================
// File Size Limit Constants
// ============================================================================
// Tiger Style: All file reads must have size limits to prevent memory exhaustion
// from maliciously large or corrupted files.

/// Maximum size for configuration files (10 MB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from oversized config files.
/// Most config files are < 100 KB; 10 MB allows for complex configurations.
///
/// Used in:
/// - `cluster/config.rs`: Cluster configuration loading
/// - `git-remote-aspen`: Git config file reading
pub const MAX_CONFIG_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum size for job specification files (1 MB).
///
/// Tiger Style: Fixed limit for job spec JSON/TOML files.
/// Job specs should be concise; 1 MB allows for complex workflows.
///
/// Used in:
/// - `aspen-jobs/replay.rs`: Job spec loading
pub const MAX_JOB_SPEC_SIZE: u64 = 1024 * 1024;

/// Maximum size for SOPS encrypted files (10 MB).
///
/// Tiger Style: Fixed limit for encrypted secret files.
/// SOPS files are typically < 100 KB; 10 MB allows for large secret bundles.
///
/// Used in:
/// - `aspen-secrets/sops/decryptor.rs`: SOPS file loading
pub const MAX_SOPS_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum size for SQL query files (1 MB).
///
/// Tiger Style: Fixed limit for SQL script files.
/// Most queries are < 10 KB; 1 MB allows for complex migrations.
///
/// Used in:
/// - `aspen-cli/commands/sql.rs`: SQL file execution
pub const MAX_SQL_FILE_SIZE: u64 = 1024 * 1024;

/// Maximum size for key files (64 KB).
///
/// Tiger Style: Fixed limit for cryptographic key files.
/// Keys are typically < 10 KB; 64 KB is generous for any key format.
///
/// Used in:
/// - `aspen-cli/commands/git.rs`: SSH key loading
/// - `aspen-cluster/lib.rs`: Key file reading
pub const MAX_KEY_FILE_SIZE: u64 = 64 * 1024;

/// Maximum size for simulation artifact files (10 MB).
///
/// Tiger Style: Fixed limit for simulation JSON artifacts.
/// Simulation traces can be large but should be bounded.
///
/// Used in:
/// - `aspen-core/simulation.rs`: Artifact loading
pub const MAX_SIMULATION_ARTIFACT_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum size for TUI state files (1 MB).
///
/// Tiger Style: Fixed limit for TUI persistent state.
/// State files should be small; 1 MB is generous.
///
/// Used in:
/// - `aspen-tui/types.rs`: State file loading
pub const MAX_TUI_STATE_SIZE: u64 = 1024 * 1024;

/// Maximum size for git packed-refs file (10 MB).
///
/// Tiger Style: Fixed limit for git packed-refs files.
/// Large repos can have many refs; 10 MB allows for ~100K refs.
///
/// Used in:
/// - `git-remote-aspen`: packed-refs parsing
pub const MAX_GIT_PACKED_REFS_SIZE: u64 = 10 * 1024 * 1024;

// ============================================================================
// Git Remote Helper Constants (Tiger Style Resource Bounds)
// ============================================================================

/// Maximum depth for recursive git object traversal (1000 levels).
///
/// Tiger Style: Fixed limit prevents stack overflow from maliciously crafted
/// git repositories with deeply nested structures (e.g., deeply nested trees
/// or long commit chains).
///
/// Used in:
/// - `git-remote-aspen/main.rs`: collect_objects() recursion
pub const MAX_GIT_OBJECT_TREE_DEPTH: u32 = 1000;

/// Maximum number of git objects in a single push operation (100,000 objects).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from large repositories.
/// A repository with 100K objects is already quite large; larger repos should
/// use incremental pushes.
///
/// Used in:
/// - `git-remote-aspen/main.rs`: collect_objects() visited set bounds
pub const MAX_GIT_OBJECTS_PER_PUSH: u32 = 100_000;

/// Maximum decompressed git object size (100 MB).
///
/// Tiger Style: Fixed limit prevents compression bomb attacks where a small
/// compressed object expands to exhaust memory during decompression.
///
/// Used in:
/// - `git-remote-aspen/main.rs`: read_loose_object() bounded decompression
pub const MAX_GIT_OBJECT_SIZE: u64 = 100 * 1024 * 1024;
