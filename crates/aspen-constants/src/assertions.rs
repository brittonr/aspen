//! Compile-time constant assertions for Tiger Style compliance.
//!
//! These assertions catch configuration errors at compile time rather than runtime.
//! Each assertion verifies a relationship between constants that must hold for
//! correct system operation.
//!
//! # Organization
//!
//! Assertions are grouped by category:
//! - **Network timeouts**: Ordering invariants for timeout progression
//! - **Size limits**: Relationships between key, value, and batch sizes
//! - **Queue bounds**: Consistency of queue-related constants
//! - **Coordination primitives**: RWLock, Semaphore, Lock sanity checks
//! - **Service registry**: Discovery and instance limits
//! - **CI/VM resources**: Resource allocation bounds
//! - **Memory bounds**: Memory usage constraints
//! - **Protocol limits**: Wire format compatibility

use super::api::*;
use super::ci::*;
use super::coordination::*;
use super::network::*;
use super::raft::*;

// ============================================================================
// Network Timeout Ordering
// ============================================================================
// Timeouts must be ordered from fastest to slowest to reflect the expected
// sequence of operations: connect -> open stream -> read response.

const _: () = assert!(IROH_CONNECT_TIMEOUT_SECS < IROH_READ_TIMEOUT_SECS);
const _: () = assert!(IROH_STREAM_OPEN_TIMEOUT_SECS < IROH_READ_TIMEOUT_SECS);
const _: () = assert!(IROH_CONNECT_TIMEOUT_SECS > 0);
const _: () = assert!(IROH_STREAM_OPEN_TIMEOUT_SECS > 0);
const _: () = assert!(IROH_READ_TIMEOUT_SECS > 0);

// ReadIndex should complete within read timeout
const _: () = assert!(READ_INDEX_TIMEOUT_SECS <= IROH_READ_TIMEOUT_SECS * 2);

// Membership operations are longer due to consensus rounds
const _: () = assert!(MEMBERSHIP_OPERATION_TIMEOUT_SECS > READ_INDEX_TIMEOUT_SECS);

// Gossip timeouts
const _: () = assert!(GOSSIP_SUBSCRIBE_TIMEOUT_SECS > 0);
const _: () = assert!(GOSSIP_SUBSCRIBE_TIMEOUT_SECS <= IROH_READ_TIMEOUT_SECS * 2);

// Gossip interval ordering
const _: () = assert!(GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS < GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS);
const _: () = assert!(GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS > 0);

// ============================================================================
// Key-Value Size Limits
// ============================================================================
// Keys must be smaller than values; batch operations must not exceed memory limits.

// Keys must fit within reasonable bounds
const _: () = assert!(MAX_KEY_SIZE > 0);
const _: () = assert!(MAX_KEY_SIZE < MAX_VALUE_SIZE);
const _: () = assert!(MAX_KEY_SIZE <= 1024 * 1024); // max 1MB key (sanity check)

// Values must be bounded
const _: () = assert!(MAX_VALUE_SIZE > 0);
const _: () = assert!(MAX_VALUE_SIZE <= 100 * 1024 * 1024); // max 100MB value (sanity check)

// SetMulti operations should not exceed practical memory limits
// 100 keys * 1MB values = 100MB max per SetMulti (acceptable)
const _: () = assert!(MAX_SETMULTI_KEYS > 0);
const _: () = assert!(MAX_SETMULTI_KEYS <= 1000); // sanity check

// Scan results must be bounded
const _: () = assert!(DEFAULT_SCAN_LIMIT > 0);
const _: () = assert!(DEFAULT_SCAN_LIMIT <= MAX_SCAN_RESULTS);
const _: () = assert!(MAX_SCAN_RESULTS > 0);
const _: () = assert!(MAX_SCAN_RESULTS <= 100_000); // sanity check

// ============================================================================
// SQL Constants (when sql feature is enabled)
// ============================================================================
#[cfg(feature = "sql")]
const _: () = {
    assert!(MAX_SQL_QUERY_SIZE > 0);
    assert!(MAX_SQL_PARAMS > 0);
    assert!(DEFAULT_SQL_RESULT_ROWS <= MAX_SQL_RESULT_ROWS);
    assert!(DEFAULT_SQL_TIMEOUT_MS <= MAX_SQL_TIMEOUT_MS);
    assert!(DEFAULT_SQL_TIMEOUT_MS > 0);
    assert!(MAX_SQL_TIMEOUT_MS > 0);
};

// ============================================================================
// VM Execution Constants
// ============================================================================

// VM timeouts must be ordered
const _: () = assert!(VM_STARTUP_TIMEOUT_MS < DEFAULT_VM_TIMEOUT_MS);
const _: () = assert!(DEFAULT_VM_TIMEOUT_MS <= MAX_VM_TIMEOUT_MS);
const _: () = assert!(VM_STARTUP_TIMEOUT_MS > 0);

// VM resources must be bounded
const _: () = assert!(MAX_CONCURRENT_VMS > 0);
const _: () = assert!(MAX_CONCURRENT_VMS <= 1000); // sanity check

// Binary size must be bounded
const _: () = assert!(MAX_BINARY_SIZE > 0);
const _: () = assert!(MAX_BINARY_SIZE <= 1024 * 1024 * 1024); // max 1GB (sanity check)

// Build time must be reasonable
const _: () = assert!(MAX_BUILD_TIME_MS > 0);
const _: () = assert!(MAX_BUILD_TIME_MS <= 60 * 60 * 1000); // max 1 hour (sanity check)

// ============================================================================
// Queue Bounds Consistency
// ============================================================================

// Queue item size must be bounded
const _: () = assert!(MAX_QUEUE_ITEM_SIZE > 0);
const _: () = assert!(MAX_QUEUE_ITEM_SIZE <= MAX_VALUE_SIZE);

// Batch sizes must be reasonable
const _: () = assert!(MAX_QUEUE_BATCH_SIZE > 0);
const _: () = assert!(MAX_QUEUE_BATCH_SIZE <= MAX_SCAN_RESULTS);
const _: () = assert!(MAX_QUEUE_CLEANUP_BATCH > 0);

// Queue timeout ordering
const _: () = assert!(DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS > 0);
const _: () = assert!(DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS <= MAX_QUEUE_VISIBILITY_TIMEOUT_MS);
const _: () = assert!(MAX_QUEUE_VISIBILITY_TIMEOUT_MS > 0);

// Queue polling ordering
const _: () = assert!(DEFAULT_QUEUE_POLL_INTERVAL_MS > 0);
const _: () = assert!(DEFAULT_QUEUE_POLL_INTERVAL_MS <= MAX_QUEUE_POLL_INTERVAL_MS);
const _: () = assert!(MAX_QUEUE_POLL_INTERVAL_MS > 0);

// Queue TTL must be reasonable
const _: () = assert!(DEFAULT_QUEUE_DEDUP_TTL_MS > 0);
const _: () = assert!(MAX_QUEUE_ITEM_TTL_MS > DEFAULT_QUEUE_DEDUP_TTL_MS);
const _: () = assert!(MAX_QUEUE_ITEM_TTL_MS > MAX_QUEUE_VISIBILITY_TIMEOUT_MS);

// ============================================================================
// RWLock/Semaphore/Lock Sanity
// ============================================================================

// RWLock must allow at least one reader and one writer
const _: () = assert!(MAX_RWLOCK_READERS > 0);
const _: () = assert!(MAX_RWLOCK_PENDING_WRITERS > 0);

// Semaphore must allow at least one holder
const _: () = assert!(MAX_SEMAPHORE_HOLDERS > 0);

// Lock timeout ordering
const _: () = assert!(DEFAULT_LOCK_WAIT_TIMEOUT_MS > 0);
const _: () = assert!(DEFAULT_LOCK_WAIT_TIMEOUT_MS <= MAX_LOCK_WAIT_TIMEOUT_MS);
const _: () = assert!(MAX_LOCK_WAIT_TIMEOUT_MS > 0);

// Lock TTL ordering
const _: () = assert!(DEFAULT_LOCK_TTL_MS > 0);
const _: () = assert!(DEFAULT_LOCK_TTL_MS <= MAX_LOCK_TTL_MS);
const _: () = assert!(MAX_LOCK_TTL_MS > 0);

// ============================================================================
// CAS Retry Constants
// ============================================================================

// CAS retries must be bounded
const _: () = assert!(MAX_CAS_RETRIES > 0);
const _: () = assert!(MAX_CAS_RETRIES <= 1000); // sanity check

// CAS backoff ordering
const _: () = assert!(CAS_RETRY_INITIAL_BACKOFF_MS > 0);
const _: () = assert!(CAS_RETRY_INITIAL_BACKOFF_MS <= CAS_RETRY_MAX_BACKOFF_MS);
const _: () = assert!(CAS_RETRY_MAX_BACKOFF_MS > 0);

// ============================================================================
// Service Registry Bounds
// ============================================================================

// Service names and IDs must be bounded
const _: () = assert!(MAX_SERVICE_NAME_SIZE > 0);
const _: () = assert!(MAX_INSTANCE_ID_SIZE > 0);
const _: () = assert!(MAX_SERVICE_ADDRESS_SIZE > 0);

// Tags must be bounded
const _: () = assert!(MAX_SERVICE_TAGS > 0);
const _: () = assert!(MAX_SERVICE_TAG_SIZE > 0);
const _: () = assert!(MAX_SERVICE_CUSTOM_METADATA > 0);

// Discovery results must fit within instances
const _: () = assert!(SERVICE_CLEANUP_BATCH > 0);
const _: () = assert!(MAX_SERVICE_DISCOVERY_RESULTS > 0);
const _: () = assert!(MAX_SERVICE_DISCOVERY_RESULTS <= MAX_SERVICE_INSTANCES);
const _: () = assert!(MAX_SERVICE_INSTANCES > 0);

// Service TTL ordering
const _: () = assert!(DEFAULT_SERVICE_TTL_MS > 0);
const _: () = assert!(DEFAULT_SERVICE_TTL_MS <= MAX_SERVICE_TTL_MS);
const _: () = assert!(MAX_SERVICE_TTL_MS > 0);

// ============================================================================
// Capability Token Constants
// ============================================================================

// Token must allow at least one capability
const _: () = assert!(MAX_CAPABILITIES_PER_TOKEN > 0);
const _: () = assert!(MAX_DELEGATION_DEPTH > 0);
const _: () = assert!(MAX_TOKEN_SIZE > 0);
const _: () = assert!(MAX_REVOCATION_LIST_SIZE > 0);
const _: () = assert!(TOKEN_CLOCK_SKEW_SECS > 0);

// ============================================================================
// Network Connection and Rate Limits
// ============================================================================

// Connection limits must be positive
const _: () = assert!(MAX_CONCURRENT_CONNECTIONS > 0);
const _: () = assert!(MAX_STREAMS_PER_CONNECTION > 0);
const _: () = assert!(MAX_PEERS > 0);
const _: () = assert!(MAX_UNREACHABLE_NODES > 0);
const _: () = assert!(MAX_PEER_COUNT > 0);

// RPC message size must be bounded but reasonable
const _: () = assert!(MAX_RPC_MESSAGE_SIZE > 0);
const _: () = assert!(MAX_RPC_MESSAGE_SIZE >= MAX_VALUE_SIZE);
const _: () = assert!(MAX_RPC_MESSAGE_SIZE <= 1024 * 1024 * 1024); // max 1GB (sanity check)

// Snapshot size must be larger than RPC message size
const _: () = assert!(MAX_SNAPSHOT_SIZE > 0);
const _: () = assert!(MAX_SNAPSHOT_SIZE >= MAX_RPC_MESSAGE_SIZE as u64);

// Gossip rate limits must be positive
const _: () = assert!(GOSSIP_MAX_TRACKED_PEERS > 0);
const _: () = assert!(GOSSIP_PER_PEER_RATE_PER_MINUTE > 0);
const _: () = assert!(GOSSIP_PER_PEER_BURST > 0);
const _: () = assert!(GOSSIP_GLOBAL_RATE_PER_MINUTE > 0);
const _: () = assert!(GOSSIP_GLOBAL_BURST > 0);
const _: () = assert!(GOSSIP_MAX_STREAM_RETRIES > 0);

// Global rate must be higher than per-peer rate
const _: () = assert!(GOSSIP_GLOBAL_RATE_PER_MINUTE > GOSSIP_PER_PEER_RATE_PER_MINUTE);
const _: () = assert!(GOSSIP_GLOBAL_BURST >= GOSSIP_PER_PEER_BURST);

// Failure detector channel must have capacity
const _: () = assert!(FAILURE_DETECTOR_CHANNEL_CAPACITY > 0);

// ============================================================================
// File Size Limits
// ============================================================================

// All file size limits must be positive
const _: () = assert!(MAX_CONFIG_FILE_SIZE > 0);
const _: () = assert!(MAX_JOB_SPEC_SIZE > 0);
const _: () = assert!(MAX_SOPS_FILE_SIZE > 0);
const _: () = assert!(MAX_SQL_FILE_SIZE > 0);
const _: () = assert!(MAX_KEY_FILE_SIZE > 0);
const _: () = assert!(MAX_SIMULATION_ARTIFACT_SIZE > 0);
const _: () = assert!(MAX_TUI_STATE_SIZE > 0);
const _: () = assert!(MAX_GIT_PACKED_REFS_SIZE > 0);

// Git limits must be positive
const _: () = assert!(MAX_GIT_OBJECT_TREE_DEPTH > 0);
const _: () = assert!(MAX_GIT_OBJECTS_PER_PUSH > 0);
const _: () = assert!(MAX_GIT_OBJECT_SIZE > 0);

// ============================================================================
// Raft Storage Constants
// ============================================================================

// Batch sizes must be positive and reasonable
const _: () = assert!(MAX_BATCH_SIZE > 0);
const _: () = assert!(MAX_BATCH_SIZE <= 100_000); // sanity check
const _: () = assert!(MAX_SNAPSHOT_ENTRIES > 0);

// Pool sizes must be positive
const _: () = assert!(DEFAULT_READ_POOL_SIZE > 0);
const _: () = assert!(DEFAULT_CAPACITY > 0);
const _: () = assert!(MAX_CAPACITY > 0);
const _: () = assert!(DEFAULT_CAPACITY <= MAX_CAPACITY);

// Connection and voter limits
const _: () = assert!(MAX_CONNECTIONS_PER_NODE > 0);
const _: () = assert!(MAX_VOTERS > 0);
const _: () = assert!(MAX_VOTERS <= 1000); // sanity check - large clusters get expensive

// Learner and membership
const _: () = assert!(LEARNER_LAG_THRESHOLD > 0);
const _: () = assert!(MEMBERSHIP_COOLDOWN_SECS > 0);

// Supervision constants
const _: () = assert!(MAX_RESTART_HISTORY_SIZE > 0);
const _: () = assert!(MAX_BACKOFF_SECONDS > 0);

// Clock drift detection
const _: () = assert!(MAX_DRIFT_OBSERVATIONS > 0);
const _: () = assert!(CLOCK_DRIFT_WARNING_THRESHOLD_MS > 0);
const _: () = assert!(CLOCK_DRIFT_WARNING_THRESHOLD_MS < CLOCK_DRIFT_ALERT_THRESHOLD_MS);
const _: () = assert!(MIN_DRIFT_OBSERVATIONS > 0);
const _: () = assert!(DRIFT_EWMA_ALPHA > 0.0);
const _: () = assert!(DRIFT_EWMA_ALPHA <= 1.0);

// Chain verification
const _: () = assert!(CHAIN_VERIFY_INTERVAL_SECS > 0);
const _: () = assert!(CHAIN_VERIFY_BATCH_SIZE > 0);
const _: () = assert!(INTEGRITY_VERSION > 0);

// ============================================================================
// CI/CD Constants
// ============================================================================

// CI job memory limits must be ordered
const _: () = assert!(MAX_CI_JOB_MEMORY_HIGH_BYTES < MAX_CI_JOB_MEMORY_BYTES);
const _: () = assert!(MAX_CI_JOB_MEMORY_BYTES > 0);
const _: () = assert!(MAX_CI_JOB_MEMORY_HIGH_BYTES > 0);

// CI job resource limits must be positive
const _: () = assert!(CI_JOB_CPU_WEIGHT > 0);
const _: () = assert!(MAX_CI_JOB_PIDS > 0);
const _: () = assert!(MAX_CI_JOB_IO_BYTES_PER_SEC > 0);
const _: () = assert!(MAX_CI_JOB_IO_OPS_PER_SEC > 0);

// CI log constants
const _: () = assert!(MAX_CI_LOG_CHUNK_SIZE > 0);
const _: () = assert!(MAX_CI_LOG_CHUNKS_PER_JOB > 0);
const _: () = assert!(CI_LOG_RETENTION_MS > 0);
const _: () = assert!(CI_LOG_FLUSH_INTERVAL_MS > 0);
const _: () = assert!(DEFAULT_CI_LOG_FETCH_CHUNKS <= MAX_CI_LOG_FETCH_CHUNKS);
const _: () = assert!(DEFAULT_CI_LOG_FETCH_CHUNKS > 0);
const _: () = assert!(MAX_CI_LOG_FETCH_CHUNKS > 0);
const _: () = assert!(MAX_TUI_LOG_LINES > 0);

// CI VM constants
const _: () = assert!(MAX_CI_VMS_PER_NODE > 0);
const _: () = assert!(CI_VM_DEFAULT_MEMORY_BYTES > 0);
const _: () = assert!(CI_VM_DEFAULT_MEMORY_BYTES <= CI_VM_MAX_MEMORY_BYTES);
const _: () = assert!(CI_VM_MAX_MEMORY_BYTES > 0);

const _: () = assert!(CI_VM_DEFAULT_VCPUS > 0);
const _: () = assert!(CI_VM_DEFAULT_VCPUS <= CI_VM_MAX_VCPUS);
const _: () = assert!(CI_VM_MAX_VCPUS > 0);

const _: () = assert!(CI_VM_BOOT_TIMEOUT_MS > 0);
const _: () = assert!(CI_VM_AGENT_TIMEOUT_MS > 0);

const _: () = assert!(CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS > 0);
const _: () = assert!(CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS <= CI_VM_MAX_EXECUTION_TIMEOUT_MS);
const _: () = assert!(CI_VM_MAX_EXECUTION_TIMEOUT_MS > 0);

const _: () = assert!(CI_VM_DEFAULT_POOL_SIZE > 0);
const _: () = assert!(CI_VM_VSOCK_PORT > 0);
const _: () = assert!(CI_VM_MAX_MESSAGE_SIZE > 0);

// ============================================================================
// Pipeline Recovery Constants
// ============================================================================

const _: () = assert!(JOB_ORPHAN_DETECTION_THRESHOLD_MS > 0);
const _: () = assert!(MAX_PIPELINE_RECOVERY_BATCH > 0);
const _: () = assert!(JOB_HEARTBEAT_INTERVAL_MS > 0);
// Orphan detection must be significantly larger than heartbeat interval
const _: () = assert!(JOB_ORPHAN_DETECTION_THRESHOLD_MS > JOB_HEARTBEAT_INTERVAL_MS * 10);

// ============================================================================
// Memory Pressure Constants
// ============================================================================

const _: () = assert!(MEMORY_PRESSURE_WARNING_PERCENT > 0);
const _: () = assert!(MEMORY_PRESSURE_WARNING_PERCENT < MEMORY_PRESSURE_CRITICAL_PERCENT);
const _: () = assert!(MEMORY_PRESSURE_CRITICAL_PERCENT <= 100);
const _: () = assert!(MEMORY_WATCHER_INTERVAL_MS > 0);
const _: () = assert!(MEMORY_RESERVED_FOR_RAFT_BYTES > 0);

// ============================================================================
// Dependency and Workflow Constants
// ============================================================================

const _: () = assert!(MAX_DEPENDENCY_QUEUE_SIZE > 0);
const _: () = assert!(MAX_WORKFLOW_HISTORY_SIZE > 0);

// ============================================================================
// Event Dispatch Constants
// ============================================================================

const _: () = assert!(MAX_TTL_EVENT_DISPATCHES > 0);
const _: () = assert!(MAX_DOCS_EVENT_DISPATCHES > 0);
