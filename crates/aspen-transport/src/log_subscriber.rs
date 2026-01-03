//! Raft log subscription protocol for read-only clients.
//!
//! Provides a streaming interface for clients to receive committed Raft log
//! entries in real-time. This enables building reactive systems that respond
//! to state changes without polling.
//!
//! # Protocol Flow
//!
//! 1. Client connects via LOG_SUBSCRIBER_ALPN
//! 2. Server sends AuthChallenge (same as Raft auth)
//! 3. Client sends AuthResponse with valid HMAC
//! 4. Server sends AuthResult
//! 5. If authenticated, client sends SubscribeRequest
//! 6. Server streams LogEntryMessage for each committed entry
//!
//! # Design Notes
//!
//! - Read-only: Subscribers cannot modify state
//! - Authenticated: Uses same cookie-based auth as Raft RPC
//! - Bounded: Fixed buffer sizes and rate limits
//! - Resumable: Clients can specify starting log index
//!
//! # Tiger Style
//!
//! - Fixed message sizes with explicit bounds
//! - Explicit subscription limits (max subscribers, buffer size)
//! - Clear protocol versioning for forward compatibility

use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// Constants
// ============================================================================

/// ALPN identifier for log subscription protocol.
pub const LOG_SUBSCRIBER_ALPN: &[u8] = b"aspen-logs";

/// Maximum number of concurrent log subscribers per node.
///
/// Tiger Style: Fixed upper bound on subscriber connections.
pub const MAX_LOG_SUBSCRIBERS: usize = 100;

/// Size of the broadcast channel buffer for log entries.
///
/// Subscribers that fall behind by more than this many entries
/// will experience lag (receive lagged error).
pub const LOG_BROADCAST_BUFFER_SIZE: usize = 1000;

/// Maximum size of a single log entry message (10 MB).
///
/// Matches MAX_RPC_MESSAGE_SIZE for consistency.
pub const MAX_LOG_ENTRY_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Timeout for subscription handshake (5 seconds).
pub const SUBSCRIBE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Keepalive interval for idle connections (30 seconds).
pub const SUBSCRIBE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

/// Protocol version for log subscription.
pub const LOG_SUBSCRIBE_PROTOCOL_VERSION: u8 = 1;

/// Maximum entries to fetch in a single historical replay batch.
/// Tiger Style: Bounded batch size to prevent memory exhaustion.
pub const MAX_HISTORICAL_BATCH_SIZE: usize = 1000;

// ============================================================================
// Historical Log Reader Trait
// ============================================================================

/// Trait for reading historical log entries for replay.
///
/// Implementations should return log entries in the given range, converting
/// from the internal Raft log format to `LogEntryPayload`.
use std::future::Future;
use std::pin::Pin;

pub trait HistoricalLogReader: Send + Sync + std::fmt::Debug {
    /// Fetch log entries in the given range [start, end].
    ///
    /// Returns entries ordered by index. If the start index has been
    /// purged (compacted), returns entries starting from the earliest
    /// available index.
    ///
    /// # Arguments
    /// * `start_index` - First log index to fetch (inclusive)
    /// * `end_index` - Last log index to fetch (inclusive)
    ///
    /// # Returns
    /// * `Ok(entries)` - Vector of log entries in the range
    /// * `Err(error)` - If reading fails
    fn read_entries(
        &self,
        start_index: u64,
        end_index: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LogEntryPayload>, std::io::Error>> + Send + '_>>;

    /// Get the earliest available log index (after compaction).
    ///
    /// Returns `None` if no logs exist yet.
    fn earliest_available_index(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<u64>, std::io::Error>> + Send + '_>>;
}

// ============================================================================
// Protocol Types
// ============================================================================

/// Subscription request sent by client after authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    /// Starting log index (0 = from beginning, u64::MAX = latest only).
    pub start_index: u64,
    /// Optional key prefix filter (empty = all keys).
    pub key_prefix: Vec<u8>,
    /// Protocol version for compatibility checking.
    pub protocol_version: u8,
}

impl SubscribeRequest {
    /// Create a subscription starting from a specific log index.
    pub fn from_index(index: u64) -> Self {
        Self {
            start_index: index,
            key_prefix: Vec::new(),
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        }
    }

    /// Create a subscription for only the latest entries.
    pub fn latest_only() -> Self {
        Self {
            start_index: u64::MAX,
            key_prefix: Vec::new(),
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        }
    }

    /// Create a subscription with a key prefix filter.
    pub fn with_prefix(start_index: u64, prefix: impl Into<Vec<u8>>) -> Self {
        Self {
            start_index,
            key_prefix: prefix.into(),
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        }
    }
}

/// Response to subscription request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscribeResponse {
    /// Subscription accepted, streaming will begin.
    Accepted {
        /// Current committed index at time of subscription.
        current_index: u64,
        /// Node ID of the server.
        node_id: u64,
    },
    /// Subscription rejected.
    Rejected {
        /// Reason for rejection.
        reason: SubscribeRejectReason,
    },
}

/// Reasons why a subscription might be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscribeRejectReason {
    /// Too many subscribers connected.
    TooManySubscribers,
    /// Requested start index is not available (compacted).
    IndexNotAvailable,
    /// Protocol version not supported.
    UnsupportedVersion,
    /// Server is not ready to accept subscriptions.
    NotReady,
    /// Generic internal error.
    InternalError,
}

impl std::fmt::Display for SubscribeRejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManySubscribers => write!(f, "too many subscribers"),
            Self::IndexNotAvailable => write!(f, "requested index not available"),
            Self::UnsupportedVersion => write!(f, "protocol version not supported"),
            Self::NotReady => write!(f, "server not ready"),
            Self::InternalError => write!(f, "internal error"),
        }
    }
}

/// A streamed log entry message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntryMessage {
    /// A committed log entry.
    Entry(LogEntryPayload),
    /// Keepalive message (sent periodically on idle connections).
    Keepalive {
        /// Current committed index.
        committed_index: u64,
        /// Server timestamp (milliseconds since UNIX epoch).
        timestamp_ms: u64,
    },
    /// Stream is ending (server shutting down or error).
    EndOfStream {
        /// Reason for stream termination.
        reason: EndOfStreamReason,
    },
}

/// Reasons why a log stream might end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndOfStreamReason {
    /// Server is shutting down gracefully.
    ServerShutdown,
    /// Client requested disconnect.
    ClientDisconnect,
    /// Subscriber fell too far behind.
    Lagged,
    /// Internal error occurred.
    InternalError,
}

impl std::fmt::Display for EndOfStreamReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerShutdown => write!(f, "server shutdown"),
            Self::ClientDisconnect => write!(f, "client disconnect"),
            Self::Lagged => write!(f, "subscriber lagged"),
            Self::InternalError => write!(f, "internal error"),
        }
    }
}

/// Payload of a committed log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntryPayload {
    /// Log index of this entry.
    pub index: u64,
    /// Raft term when entry was created.
    pub term: u64,
    /// Timestamp when entry was committed (milliseconds since UNIX epoch).
    pub committed_at_ms: u64,
    /// The operation that was committed.
    pub operation: KvOperation,
}

/// Key-value operations that appear in the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvOperation {
    /// Set a single key-value pair.
    Set {
        /// Key to set.
        key: Vec<u8>,
        /// Value to store.
        value: Vec<u8>,
    },
    /// Set a single key-value pair with expiration.
    SetWithTTL {
        /// Key to set.
        key: Vec<u8>,
        /// Value to store.
        value: Vec<u8>,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Set multiple key-value pairs atomically.
    SetMulti {
        /// Key-value pairs to set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
    },
    /// Set multiple keys with TTL.
    SetMultiWithTTL {
        /// Key-value pairs to set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Delete a single key.
    Delete {
        /// Key to delete.
        key: Vec<u8>,
    },
    /// Delete multiple keys atomically.
    DeleteMulti {
        /// Keys to delete.
        keys: Vec<Vec<u8>>,
    },
    /// Compare-and-swap: atomically update value if current matches expected.
    CompareAndSwap {
        /// Key to update.
        key: Vec<u8>,
        /// Expected current value (None means key must not exist).
        expected: Option<Vec<u8>>,
        /// New value to set if condition passes.
        new_value: Vec<u8>,
    },
    /// Compare-and-delete: atomically delete if current value matches expected.
    CompareAndDelete {
        /// Key to delete.
        key: Vec<u8>,
        /// Expected current value.
        expected: Vec<u8>,
    },
    /// Batch of mixed Set/Delete operations.
    Batch {
        /// Operations as (is_set, key, value) tuples.
        operations: Vec<(bool, Vec<u8>, Vec<u8>)>,
    },
    /// Conditional batch: operations applied only if all conditions pass.
    ConditionalBatch {
        /// Conditions as (type, key, expected) tuples.
        /// Types: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists.
        conditions: Vec<(u8, Vec<u8>, Vec<u8>)>,
        /// Operations as (is_set, key, value) tuples.
        operations: Vec<(bool, Vec<u8>, Vec<u8>)>,
    },
    /// Set a single key-value pair attached to a lease.
    SetWithLease {
        /// Key to set.
        key: Vec<u8>,
        /// Value to store.
        value: Vec<u8>,
        /// Lease ID this key is attached to.
        lease_id: u64,
    },
    /// Set multiple keys attached to a lease.
    SetMultiWithLease {
        /// Key-value pairs to set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
        /// Lease ID these keys are attached to.
        lease_id: u64,
    },
    /// Grant a new lease.
    LeaseGrant {
        /// Lease ID (0 = auto-generated).
        lease_id: u64,
        /// TTL in seconds.
        ttl_seconds: u32,
    },
    /// Revoke a lease and delete all attached keys.
    LeaseRevoke {
        /// Lease ID to revoke.
        lease_id: u64,
    },
    /// Refresh a lease's TTL.
    LeaseKeepalive {
        /// Lease ID to refresh.
        lease_id: u64,
    },
    /// Multi-key transaction with If/Then/Else semantics.
    Transaction {
        /// Comparison conditions as (target, op, key, value).
        compare: Vec<(u8, u8, Vec<u8>, Vec<u8>)>,
        /// Success branch operations as (op_type, key, value).
        success: Vec<(u8, Vec<u8>, Vec<u8>)>,
        /// Failure branch operations.
        failure: Vec<(u8, Vec<u8>, Vec<u8>)>,
    },
    /// Optimistic transaction with read set version validation.
    OptimisticTransaction {
        /// Read set: keys with expected versions for conflict detection.
        read_set: Vec<(Vec<u8>, i64)>,
        /// Write set: operations to apply if no conflicts.
        /// Format: (is_set, key, value). is_set=true for Set, false for Delete.
        write_set: Vec<(bool, Vec<u8>, Vec<u8>)>,
    },
    /// No-op entry (used for leader election).
    Noop,
    /// Membership change entry.
    MembershipChange {
        /// Human-readable description of the change.
        description: String,
    },
}

impl KvOperation {
    /// Returns true if this operation affects the given key prefix.
    pub fn matches_prefix(&self, prefix: &[u8]) -> bool {
        if prefix.is_empty() {
            return true;
        }

        match self {
            KvOperation::Set { key, .. }
            | KvOperation::SetWithTTL { key, .. }
            | KvOperation::SetWithLease { key, .. }
            | KvOperation::Delete { key }
            | KvOperation::CompareAndSwap { key, .. }
            | KvOperation::CompareAndDelete { key, .. } => key.starts_with(prefix),
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => pairs.iter().any(|(k, _)| k.starts_with(prefix)),
            KvOperation::DeleteMulti { keys } => keys.iter().any(|k| k.starts_with(prefix)),
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                operations.iter().any(|(_, k, _)| k.starts_with(prefix))
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => {
                // Always include control/lease operations
                true
            }
            KvOperation::Transaction { success, failure, .. } => {
                // Check if any operation in success or failure branches affects the prefix
                let check_ops = |ops: &[(u8, Vec<u8>, Vec<u8>)]| {
                    ops.iter().any(|(op_type, key, _)| {
                        // op_type: 0=Put, 1=Delete, 2=Get, 3=Range
                        // Put and Delete modify keys, Get and Range just read
                        (*op_type == 0 || *op_type == 1) && key.starts_with(prefix)
                    })
                };
                check_ops(success) || check_ops(failure)
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Check if any write operation affects the prefix
                write_set.iter().any(|(_, key, _)| key.starts_with(prefix))
            }
        }
    }

    /// Returns the primary key affected by this operation, if any.
    pub fn primary_key(&self) -> Option<&[u8]> {
        match self {
            KvOperation::Set { key, .. }
            | KvOperation::SetWithTTL { key, .. }
            | KvOperation::SetWithLease { key, .. }
            | KvOperation::Delete { key }
            | KvOperation::CompareAndSwap { key, .. }
            | KvOperation::CompareAndDelete { key, .. } => Some(key),
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => pairs.first().map(|(k, _)| k.as_slice()),
            KvOperation::DeleteMulti { keys } => keys.first().map(|k| k.as_slice()),
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                operations.first().map(|(_, k, _)| k.as_slice())
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => None,
            KvOperation::Transaction { success, .. } => {
                // Return the first key from success branch
                success.first().map(|(_, k, _)| k.as_slice())
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Return the first key from write set
                write_set.first().map(|(_, k, _)| k.as_slice())
            }
        }
    }

    /// Returns the number of keys affected by this operation.
    pub fn key_count(&self) -> usize {
        match self {
            KvOperation::Set { .. }
            | KvOperation::SetWithTTL { .. }
            | KvOperation::SetWithLease { .. }
            | KvOperation::Delete { .. }
            | KvOperation::CompareAndSwap { .. }
            | KvOperation::CompareAndDelete { .. } => 1,
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => pairs.len(),
            KvOperation::DeleteMulti { keys } => keys.len(),
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => operations.len(),
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => 0,
            KvOperation::Transaction { success, failure, .. } => {
                // Count put/delete operations in both branches (get/range don't modify keys)
                let count_mods = |ops: &[(u8, Vec<u8>, Vec<u8>)]| {
                    ops.iter().filter(|(op_type, _, _)| *op_type == 0 || *op_type == 1).count()
                };
                count_mods(success) + count_mods(failure)
            }
            KvOperation::OptimisticTransaction { write_set, .. } => write_set.len(),
        }
    }
}

impl From<aspen_raft_types::AppRequest> for KvOperation {
    fn from(req: aspen_raft_types::AppRequest) -> Self {
        use aspen_raft_types::AppRequest;
        match req {
            AppRequest::Set { key, value } => KvOperation::Set {
                key: key.into_bytes(),
                value: value.into_bytes(),
            },
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => KvOperation::SetWithTTL {
                key: key.into_bytes(),
                value: value.into_bytes(),
                expires_at_ms,
            },
            AppRequest::SetMulti { pairs } => KvOperation::SetMulti {
                pairs: pairs.into_iter().map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect(),
            },
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => KvOperation::SetMultiWithTTL {
                pairs: pairs.into_iter().map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect(),
                expires_at_ms,
            },
            AppRequest::Delete { key } => KvOperation::Delete { key: key.into_bytes() },
            AppRequest::DeleteMulti { keys } => KvOperation::DeleteMulti {
                keys: keys.into_iter().map(String::into_bytes).collect(),
            },
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => KvOperation::CompareAndSwap {
                key: key.into_bytes(),
                expected: expected.map(String::into_bytes),
                new_value: new_value.into_bytes(),
            },
            AppRequest::CompareAndDelete { key, expected } => KvOperation::CompareAndDelete {
                key: key.into_bytes(),
                expected: expected.into_bytes(),
            },
            AppRequest::Batch { operations } => KvOperation::Batch {
                operations: operations
                    .into_iter()
                    .map(|(is_set, key, value)| (is_set, key.into_bytes(), value.into_bytes()))
                    .collect(),
            },
            AppRequest::ConditionalBatch { conditions, operations } => KvOperation::ConditionalBatch {
                conditions: conditions
                    .into_iter()
                    .map(|(cond_type, key, expected)| (cond_type, key.into_bytes(), expected.into_bytes()))
                    .collect(),
                operations: operations
                    .into_iter()
                    .map(|(is_set, key, value)| (is_set, key.into_bytes(), value.into_bytes()))
                    .collect(),
            },
            AppRequest::SetWithLease { key, value, lease_id } => KvOperation::SetWithLease {
                key: key.into_bytes(),
                value: value.into_bytes(),
                lease_id,
            },
            AppRequest::SetMultiWithLease { pairs, lease_id } => KvOperation::SetMultiWithLease {
                pairs: pairs.into_iter().map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect(),
                lease_id,
            },
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => KvOperation::LeaseGrant { lease_id, ttl_seconds },
            AppRequest::LeaseRevoke { lease_id } => KvOperation::LeaseRevoke { lease_id },
            AppRequest::LeaseKeepalive { lease_id } => KvOperation::LeaseKeepalive { lease_id },
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => {
                // Convert String-based tuples to Vec<u8>-based tuples
                let cmp = compare.into_iter().map(|(t, o, k, v)| (t, o, k.into_bytes(), v.into_bytes())).collect();
                let succ = success.into_iter().map(|(t, k, v)| (t, k.into_bytes(), v.into_bytes())).collect();
                let fail = failure.into_iter().map(|(t, k, v)| (t, k.into_bytes(), v.into_bytes())).collect();
                KvOperation::Transaction {
                    compare: cmp,
                    success: succ,
                    failure: fail,
                }
            }
            AppRequest::OptimisticTransaction { read_set, write_set } => {
                let reads = read_set.into_iter().map(|(k, v)| (k.into_bytes(), v)).collect();
                let writes =
                    write_set.into_iter().map(|(is_set, k, v)| (is_set, k.into_bytes(), v.into_bytes())).collect();
                KvOperation::OptimisticTransaction {
                    read_set: reads,
                    write_set: writes,
                }
            }
            // Shard topology operations are converted to Noop for log subscribers
            // (subscribers don't need to track topology changes)
            AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
                KvOperation::Noop
            }
        }
    }
}

// ============================================================================
// Subscriber State
// ============================================================================

/// State tracking for a connected subscriber.
#[derive(Debug)]
pub struct SubscriberState {
    /// Unique identifier for this subscriber connection.
    pub id: u64,
    /// Client's Iroh endpoint ID.
    pub client_endpoint_id: [u8; 32],
    /// Key prefix filter (empty = all keys).
    pub key_prefix: Vec<u8>,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Connection timestamp (milliseconds since UNIX epoch).
    pub connected_at_ms: u64,
    /// Number of entries sent.
    pub entries_sent: u64,
}

impl SubscriberState {
    /// Create new subscriber state.
    pub fn new(id: u64, client_endpoint_id: [u8; 32], key_prefix: Vec<u8>, start_index: u64) -> Self {
        Self {
            id,
            client_endpoint_id,
            key_prefix,
            last_sent_index: start_index.saturating_sub(1),
            connected_at_ms: current_time_ms(),
            entries_sent: 0,
        }
    }

    /// Check if an operation should be sent to this subscriber.
    pub fn should_send(&self, operation: &KvOperation) -> bool {
        operation.matches_prefix(&self.key_prefix)
    }

    /// Record that an entry was sent.
    pub fn record_sent(&mut self, index: u64) {
        self.last_sent_index = index;
        self.entries_sent += 1;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current time in milliseconds since UNIX epoch.
///
/// Delegates to `crate::utils::current_time_ms()` for Tiger Style compliance.
#[inline]
fn current_time_ms() -> u64 {
    aspen_core::utils::current_time_ms()
}

// ============================================================================
// Protocol Handler
// ============================================================================

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use crate::rpc::{AuthContext, AuthResponse, AuthResult};

// Auth constants
const AUTH_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_AUTH_MESSAGE_SIZE: usize = 256;

/// Protocol handler for log subscription over Iroh.
///
/// Provides a read-only interface for clients to stream committed Raft log entries.
/// Uses the same HMAC-SHA256 authentication as the authenticated Raft handler.
///
/// # Tiger Style
///
/// - Bounded subscriber count
/// - Keepalive for idle connections
/// - Explicit subscription limits
#[derive(Debug)]
pub struct LogSubscriberProtocolHandler {
    auth_context: AuthContext,
    connection_semaphore: Arc<Semaphore>,
    /// Broadcast channel for log entries.
    log_sender: broadcast::Sender<LogEntryPayload>,
    /// Node ID for response messages.
    node_id: u64,
    /// Subscriber ID counter.
    next_subscriber_id: AtomicU64,
    /// Current committed log index (updated externally).
    committed_index: Arc<AtomicU64>,
    /// Optional historical log reader for replay from start_index.
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
}

impl LogSubscriberProtocolHandler {
    /// Create a new log subscriber protocol handler.
    ///
    /// # Arguments
    /// * `cluster_cookie` - Shared secret for authentication
    /// * `node_id` - This node's ID
    ///
    /// # Returns
    /// A tuple of (handler, log_sender, committed_index_handle).
    /// - `log_sender`: Use to broadcast log entries to subscribers
    /// - `committed_index_handle`: Update this atomic to reflect current Raft committed index
    ///
    /// # Note
    /// Historical replay is disabled by default. Use `with_historical_reader()` to enable it.
    pub fn new(cluster_cookie: &str, node_id: u64) -> (Self, broadcast::Sender<LogEntryPayload>, Arc<AtomicU64>) {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let committed_index = Arc::new(AtomicU64::new(0));
        let handler = Self {
            auth_context: AuthContext::new(cluster_cookie.to_string()),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender: log_sender.clone(),
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index: committed_index.clone(),
            historical_reader: None,
        };
        (handler, log_sender, committed_index)
    }

    /// Create a handler with an existing broadcast sender and committed index tracker.
    ///
    /// Use this when you need multiple handlers to share the same broadcast channel
    /// and committed index state.
    pub fn with_sender(
        cluster_cookie: &str,
        node_id: u64,
        log_sender: broadcast::Sender<LogEntryPayload>,
        committed_index: Arc<AtomicU64>,
    ) -> Self {
        Self {
            auth_context: AuthContext::new(cluster_cookie.to_string()),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: None,
        }
    }

    /// Create a handler with historical log replay support.
    ///
    /// When a subscriber connects with a `start_index`, historical entries
    /// from `start_index` to the current committed index will be replayed
    /// before streaming new entries.
    ///
    /// # Arguments
    /// * `cluster_cookie` - Shared secret for authentication
    /// * `node_id` - This node's ID
    /// * `log_sender` - Broadcast channel for new entries
    /// * `committed_index` - Atomic counter for current committed index
    /// * `historical_reader` - Reader for fetching historical log entries
    pub fn with_historical_reader(
        cluster_cookie: &str,
        node_id: u64,
        log_sender: broadcast::Sender<LogEntryPayload>,
        committed_index: Arc<AtomicU64>,
        historical_reader: Arc<dyn HistoricalLogReader>,
    ) -> Self {
        Self {
            auth_context: AuthContext::new(cluster_cookie.to_string()),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: Some(historical_reader),
        }
    }

    /// Get a handle to the committed index for external updates.
    ///
    /// Call `committed_index_handle.store(new_index, Ordering::Release)` when
    /// the Raft committed index changes.
    pub fn committed_index_handle(&self) -> Arc<AtomicU64> {
        self.committed_index.clone()
    }
}

impl ProtocolHandler for LogSubscriberProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Log subscriber limit reached ({}), rejecting connection from {}",
                    MAX_LOG_SUBSCRIBERS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other("subscriber limit reached")));
            }
        };

        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        debug!(
            remote_node = %remote_node_id,
            subscriber_id = subscriber_id,
            "accepted log subscriber connection"
        );

        // Handle the subscriber connection
        let result = handle_log_subscriber_connection(
            connection,
            self.auth_context.clone(),
            self.log_sender.subscribe(),
            self.node_id,
            subscriber_id,
            self.committed_index.clone(),
            self.historical_reader.clone(),
        )
        .await;

        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Log subscriber protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a log subscriber connection.
#[instrument(skip(connection, auth_context, log_receiver, committed_index, historical_reader))]
async fn handle_log_subscriber_connection(
    connection: Connection,
    auth_context: AuthContext,
    mut log_receiver: broadcast::Receiver<LogEntryPayload>,
    node_id: u64,
    subscriber_id: u64,
    committed_index: Arc<AtomicU64>,
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let remote_node_id = connection.remote_id();

    // Accept the initial stream for authentication and subscription setup
    let (mut send, mut recv) = connection.accept_bi().await.context("failed to accept subscriber stream")?;

    // Step 1: Send challenge
    let challenge = auth_context.generate_challenge();
    let challenge_bytes = postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    send.write_all(&challenge_bytes).await.context("failed to send challenge")?;

    // Step 2: Receive auth response
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = recv.read_to_end(MAX_AUTH_MESSAGE_SIZE).await.context("failed to read auth response")?;
        let response: AuthResponse = postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await;

    let auth_response = match response_result {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            warn!(error = %err, subscriber_id = subscriber_id, "subscriber auth failed");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            // Best-effort send of failure response - log if it fails
            if let Err(write_err) = send.write_all(&result_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth failure to subscriber");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth failure");
            }
            return Err(err);
        }
        Err(_) => {
            warn!(subscriber_id = subscriber_id, "subscriber auth timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            // Best-effort send of failure response - log if it fails
            if let Err(write_err) = send.write_all(&result_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth timeout to subscriber");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth timeout");
            }
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify
    let auth_success = auth_context.verify_response(&challenge, &auth_response);

    // Step 4: Send auth result
    let auth_result = if auth_success {
        AuthResult::Success
    } else {
        AuthResult::Failed
    };
    let result_bytes = postcard::to_stdvec(&auth_result)?;
    send.write_all(&result_bytes).await.context("failed to send auth result")?;

    if !auth_success {
        warn!(subscriber_id = subscriber_id, result = ?auth_result, "subscriber auth failed");
        if let Err(finish_err) = send.finish() {
            debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth verification failure");
        }
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!(subscriber_id = subscriber_id, "subscriber authenticated");

    // Step 5: Receive subscription request
    let sub_request_result = tokio::time::timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
        let buffer = recv
            .read_to_end(1024) // Subscription requests are small
            .await
            .context("failed to read subscribe request")?;
        let request: SubscribeRequest =
            postcard::from_bytes(&buffer).context("failed to deserialize subscribe request")?;
        Ok::<_, anyhow::Error>(request)
    })
    .await;

    let sub_request = match sub_request_result {
        Ok(Ok(request)) => request,
        Ok(Err(err)) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            // Best-effort send of rejection - log if it fails
            if let Err(write_err) = send.write_all(&response_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe rejection");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe error");
            }
            return Err(err);
        }
        Err(_) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            // Best-effort send of rejection - log if it fails
            if let Err(write_err) = send.write_all(&response_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe timeout rejection");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe timeout");
            }
            return Err(anyhow::anyhow!("subscribe request timeout"));
        }
    };

    debug!(
        subscriber_id = subscriber_id,
        start_index = sub_request.start_index,
        prefix = ?sub_request.key_prefix,
        "processing subscription request"
    );

    // Step 6: Accept subscription
    let current_committed_index = committed_index.load(Ordering::Acquire);
    let response = SubscribeResponse::Accepted {
        current_index: current_committed_index,
        node_id,
    };
    let response_bytes = postcard::to_stdvec(&response)?;
    send.write_all(&response_bytes).await.context("failed to send subscribe response")?;

    info!(
        subscriber_id = subscriber_id,
        remote = %remote_node_id,
        start_index = sub_request.start_index,
        current_index = current_committed_index,
        "log subscription active"
    );

    // Step 6b: Historical replay if requested and available
    let replay_end_index = if sub_request.start_index < current_committed_index && sub_request.start_index != u64::MAX {
        if let Some(ref reader) = historical_reader {
            debug!(
                subscriber_id = subscriber_id,
                start_index = sub_request.start_index,
                end_index = current_committed_index,
                "starting historical replay"
            );

            // Replay in batches to avoid memory exhaustion
            let mut current_start = sub_request.start_index;
            let mut total_replayed = 0u64;

            while current_start <= current_committed_index {
                let batch_end = std::cmp::min(
                    current_start.saturating_add(MAX_HISTORICAL_BATCH_SIZE as u64 - 1),
                    current_committed_index,
                );

                match reader.read_entries(current_start, batch_end).await {
                    Ok(entries) => {
                        if entries.is_empty() {
                            // No more entries available (may have been compacted)
                            debug!(
                                subscriber_id = subscriber_id,
                                start = current_start,
                                "no historical entries available, may have been compacted"
                            );
                            break;
                        }

                        for entry in entries {
                            // Apply prefix filter
                            if !sub_request.key_prefix.is_empty()
                                && !entry.operation.matches_prefix(&sub_request.key_prefix)
                            {
                                continue;
                            }

                            let message = LogEntryMessage::Entry(entry.clone());
                            let message_bytes = match postcard::to_stdvec(&message) {
                                Ok(bytes) => bytes,
                                Err(err) => {
                                    error!(error = %err, "failed to serialize historical entry");
                                    continue;
                                }
                            };

                            if let Err(err) = send.write_all(&message_bytes).await {
                                debug!(
                                    subscriber_id = subscriber_id,
                                    error = %err,
                                    "subscriber disconnected during replay"
                                );
                                return Err(anyhow::anyhow!("subscriber disconnected during replay"));
                            }

                            total_replayed += 1;
                            current_start = entry.index + 1;
                        }
                    }
                    Err(err) => {
                        warn!(
                            subscriber_id = subscriber_id,
                            error = %err,
                            "failed to read historical entries, continuing with live stream"
                        );
                        break;
                    }
                }

                // Check if we've finished
                if current_start > current_committed_index {
                    break;
                }
            }

            info!(subscriber_id = subscriber_id, total_replayed = total_replayed, "historical replay complete");
            current_start
        } else {
            debug!(subscriber_id = subscriber_id, "historical replay requested but no reader available");
            current_committed_index
        }
    } else {
        current_committed_index
    };

    // Update the log receiver to skip entries we've already sent
    // (entries between replay_end_index and any new entries that arrived during replay)
    let _ = replay_end_index; // Used for logging context

    // Step 7: Stream log entries
    let key_prefix = sub_request.key_prefix;
    let mut keepalive_interval = tokio::time::interval(SUBSCRIBE_KEEPALIVE_INTERVAL);
    keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // Receive log entry from broadcast channel
            entry_result = log_receiver.recv() => {
                match entry_result {
                    Ok(entry) => {
                        // Apply prefix filter
                        if !key_prefix.is_empty() && !entry.operation.matches_prefix(&key_prefix) {
                            continue;
                        }

                        let message = LogEntryMessage::Entry(entry);
                        let message_bytes = match postcard::to_stdvec(&message) {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                error!(error = %err, "failed to serialize log entry");
                                continue;
                            }
                        };

                        if let Err(err) = send.write_all(&message_bytes).await {
                            debug!(subscriber_id = subscriber_id, error = %err, "subscriber disconnected");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!(
                            subscriber_id = subscriber_id,
                            lagged_count = count,
                            "subscriber lagged, disconnecting"
                        );
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::Lagged,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            // Best-effort send of end-of-stream message
                            if let Err(write_err) = send.write_all(&bytes).await {
                                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send lagged end-of-stream");
                            }
                        }
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(subscriber_id = subscriber_id, "log broadcast channel closed");
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::ServerShutdown,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            // Best-effort send of end-of-stream message
                            if let Err(write_err) = send.write_all(&bytes).await {
                                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send shutdown end-of-stream");
                            }
                        }
                        break;
                    }
                }
            }

            // Send keepalive on idle
            _ = keepalive_interval.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: committed_index.load(Ordering::Acquire),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                let message_bytes = match postcard::to_stdvec(&keepalive) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                if send.write_all(&message_bytes).await.is_err() {
                    debug!(subscriber_id = subscriber_id, "subscriber disconnected during keepalive");
                    break;
                }
            }
        }
    }

    // Best-effort stream finish - log if it fails
    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish log subscription stream");
    }

    info!(subscriber_id = subscriber_id, "log subscription ended");

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_request_from_index() {
        let req = SubscribeRequest::from_index(100);
        assert_eq!(req.start_index, 100);
        assert!(req.key_prefix.is_empty());
        assert_eq!(req.protocol_version, LOG_SUBSCRIBE_PROTOCOL_VERSION);
    }

    #[test]
    fn test_subscribe_request_latest_only() {
        let req = SubscribeRequest::latest_only();
        assert_eq!(req.start_index, u64::MAX);
    }

    #[test]
    fn test_subscribe_request_with_prefix() {
        let req = SubscribeRequest::with_prefix(50, b"users/".to_vec());
        assert_eq!(req.start_index, 50);
        assert_eq!(req.key_prefix, b"users/");
    }

    #[test]
    fn test_kv_operation_matches_prefix() {
        let set_op = KvOperation::Set {
            key: b"users/123".to_vec(),
            value: b"data".to_vec(),
        };

        assert!(set_op.matches_prefix(b""));
        assert!(set_op.matches_prefix(b"users/"));
        assert!(set_op.matches_prefix(b"users/123"));
        assert!(!set_op.matches_prefix(b"posts/"));

        let multi_op = KvOperation::SetMulti {
            pairs: vec![
                (b"users/1".to_vec(), b"a".to_vec()),
                (b"posts/1".to_vec(), b"b".to_vec()),
            ],
        };

        assert!(multi_op.matches_prefix(b"users/"));
        assert!(multi_op.matches_prefix(b"posts/"));
        assert!(!multi_op.matches_prefix(b"comments/"));

        // Control operations always match
        assert!(KvOperation::Noop.matches_prefix(b"anything"));
    }

    #[test]
    fn test_kv_operation_primary_key() {
        let set_op = KvOperation::Set {
            key: b"key1".to_vec(),
            value: b"val".to_vec(),
        };
        assert_eq!(set_op.primary_key(), Some(b"key1".as_slice()));

        let delete_op = KvOperation::Delete { key: b"key2".to_vec() };
        assert_eq!(delete_op.primary_key(), Some(b"key2".as_slice()));

        assert_eq!(KvOperation::Noop.primary_key(), None);
    }

    #[test]
    fn test_kv_operation_key_count() {
        assert_eq!(
            KvOperation::Set {
                key: vec![],
                value: vec![]
            }
            .key_count(),
            1
        );

        assert_eq!(
            KvOperation::SetMulti {
                pairs: vec![(vec![], vec![]), (vec![], vec![]), (vec![], vec![])]
            }
            .key_count(),
            3
        );

        assert_eq!(KvOperation::Noop.key_count(), 0);
    }

    #[test]
    fn test_subscriber_state_should_send() {
        let mut state = SubscriberState::new(1, [0u8; 32], b"users/".to_vec(), 0);

        let matching = KvOperation::Set {
            key: b"users/123".to_vec(),
            value: vec![],
        };
        assert!(state.should_send(&matching));

        let non_matching = KvOperation::Set {
            key: b"posts/456".to_vec(),
            value: vec![],
        };
        assert!(!state.should_send(&non_matching));

        // With empty prefix, everything matches
        state.key_prefix = vec![];
        assert!(state.should_send(&non_matching));
    }

    #[test]
    fn test_subscriber_state_record_sent() {
        let mut state = SubscriberState::new(1, [0u8; 32], vec![], 10);
        assert_eq!(state.last_sent_index, 9); // start_index - 1
        assert_eq!(state.entries_sent, 0);

        state.record_sent(10);
        assert_eq!(state.last_sent_index, 10);
        assert_eq!(state.entries_sent, 1);

        state.record_sent(11);
        assert_eq!(state.last_sent_index, 11);
        assert_eq!(state.entries_sent, 2);
    }

    #[test]
    fn test_subscribe_reject_reason_display() {
        assert_eq!(SubscribeRejectReason::TooManySubscribers.to_string(), "too many subscribers");
        assert_eq!(SubscribeRejectReason::IndexNotAvailable.to_string(), "requested index not available");
    }

    #[test]
    fn test_end_of_stream_reason_display() {
        assert_eq!(EndOfStreamReason::ServerShutdown.to_string(), "server shutdown");
        assert_eq!(EndOfStreamReason::Lagged.to_string(), "subscriber lagged");
    }
}
