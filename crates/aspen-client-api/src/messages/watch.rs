//! Watch operation types.
//!
//! Request/response types for real-time key change notification operations.

use serde::Deserialize;
use serde::Serialize;

/// Watch domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchRequest {
    /// Create a watch on keys matching a prefix.
    WatchCreate {
        prefix: String,
        start_index: u64,
        should_include_prev_value: bool,
    },
    /// Cancel an active watch.
    WatchCancel { watch_id: u64 },
    /// Get current watch status and statistics.
    WatchStatus { watch_id: Option<u64> },
}

#[cfg(feature = "auth")]
impl WatchRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::WatchCreate { prefix, .. } => Some(Operation::Read { key: prefix.clone() }),
            Self::WatchCancel { .. } | Self::WatchStatus { .. } => None,
        }
    }
}

/// Watch creation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCreateResultResponse {
    /// Whether watch creation succeeded.
    pub is_success: bool,
    /// Unique watch ID for this subscription.
    pub watch_id: Option<u64>,
    /// Current committed log index at watch creation time.
    pub current_index: Option<u64>,
    /// Error message if watch creation failed.
    pub error: Option<String>,
}

/// Watch cancellation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCancelResultResponse {
    /// Whether cancellation succeeded.
    pub is_success: bool,
    /// Watch ID that was cancelled.
    pub watch_id: u64,
    /// Error message if cancellation failed.
    pub error: Option<String>,
}

/// Watch status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatusResultResponse {
    /// Whether status query succeeded.
    pub is_success: bool,
    /// List of watch statuses.
    pub watches: Option<Vec<WatchInfo>>,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Information about an active watch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchInfo {
    /// Unique watch ID.
    pub watch_id: u64,
    /// Key prefix being watched.
    pub prefix: String,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Number of events sent.
    pub events_sent: u64,
    /// Watch creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Whether the watch includes previous values.
    pub should_include_prev_value: bool,
}

/// Streaming watch event response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEventResponse {
    /// Watch ID this event belongs to.
    pub watch_id: u64,
    /// Log index of this event.
    pub index: u64,
    /// Raft term when the operation was committed.
    pub term: u64,
    /// Timestamp when committed (ms since epoch).
    pub committed_at_ms: u64,
    /// The key-value events in this batch.
    pub events: Vec<WatchKeyEvent>,
}

/// A single key change event within a watch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchKeyEvent {
    /// Type of event.
    pub event_type: WatchEventType,
    /// Key that changed.
    pub key: String,
    /// New value (for Put events).
    pub value: Option<Vec<u8>>,
    /// Previous value (if should_include_prev_value was set).
    pub prev_value: Option<Vec<u8>>,
}

/// Type of watch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventType {
    /// Key was created or updated.
    Put,
    /// Key was deleted.
    Delete,
}
