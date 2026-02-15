//! Subscriber state tracking.

use super::kv_operation::KvOperation;

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

/// Get current time in milliseconds since UNIX epoch.
///
/// Delegates to `crate::utils::current_time_ms()` for Tiger Style compliance.
#[inline]
fn current_time_ms() -> u64 {
    aspen_core::utils::current_time_ms()
}
