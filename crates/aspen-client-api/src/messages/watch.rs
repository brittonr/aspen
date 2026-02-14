//! Watch operation response types.
//!
//! Response types for real-time key change notification operations.

use serde::{Deserialize, Serialize};

/// Watch creation result response.
///
/// Returns watch ID on success for use in cancel/status operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCreateResultResponse {
    /// Whether watch creation succeeded.
    pub success: bool,
    /// Unique watch ID for this subscription.
    pub watch_id: Option<u64>,
    /// Current committed log index at watch creation time.
    /// Useful for understanding the starting point.
    pub current_index: Option<u64>,
    /// Error message if watch creation failed.
    pub error: Option<String>,
}

/// Watch cancellation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchCancelResultResponse {
    /// Whether cancellation succeeded.
    pub success: bool,
    /// Watch ID that was cancelled.
    pub watch_id: u64,
    /// Error message if cancellation failed (e.g., watch not found).
    pub error: Option<String>,
}

/// Watch status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchStatusResultResponse {
    /// Whether status query succeeded.
    pub success: bool,
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
    pub include_prev_value: bool,
}

/// Streaming watch event response.
///
/// Delivered asynchronously to clients with active watches.
/// Similar to etcd's WatchResponse.
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
    /// Previous value (if include_prev_value was set).
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
