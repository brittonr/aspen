//! Queue type definitions.
//!
//! This module contains all the struct and enum definitions used by the queue system.

use serde::Deserialize;
use serde::Serialize;

use crate::types::now_unix_ms;
use crate::verified;

/// Queue metadata state stored at `__queue:{name}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueState {
    /// Queue name.
    pub name: String,
    /// Maximum delivery attempts before moving to DLQ (0 = no limit).
    pub max_delivery_attempts: u32,
    /// Default visibility timeout in milliseconds.
    pub default_visibility_timeout_ms: u64,
    /// Default item TTL in milliseconds (0 = no expiration).
    pub default_ttl_ms: u64,
    /// Creation timestamp (Unix ms).
    pub created_at_ms: u64,
    /// Queue statistics.
    pub stats: QueueStats,
}

impl Default for QueueState {
    fn default() -> Self {
        use aspen_constants::coordination::DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS;
        Self {
            name: String::new(),
            max_delivery_attempts: 3,
            default_visibility_timeout_ms: DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS,
            default_ttl_ms: 0,
            created_at_ms: now_unix_ms(),
            stats: QueueStats::default(),
        }
    }
}

/// Queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueStats {
    /// Total items enqueued (monotonic counter).
    pub total_enqueued: u64,
    /// Total items successfully acked.
    pub total_acked: u64,
    /// Total items moved to DLQ.
    pub total_dlq: u64,
}

/// A single item in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    /// Unique item ID (monotonically increasing).
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Time enqueued (Unix ms).
    pub enqueued_at_ms: u64,
    /// Optional expiration deadline (Unix ms). 0 = no expiration.
    pub expires_at_ms: u64,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
    /// Optional message group ID for FIFO ordering within groups.
    pub message_group_id: Option<String>,
    /// Optional deduplication ID.
    pub deduplication_id: Option<String>,
}

impl QueueItem {
    /// Check if item has expired.
    pub fn is_expired(&self) -> bool {
        verified::is_queue_item_expired(self.expires_at_ms, now_unix_ms())
    }
}

/// A pending item being processed by a consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingItem {
    /// Original item ID.
    pub item_id: u64,
    /// Original item payload.
    pub payload: Vec<u8>,
    /// Consumer ID processing this item.
    pub consumer_id: String,
    /// Receipt handle for acknowledgment.
    pub receipt_handle: String,
    /// When this item was dequeued (Unix ms).
    pub dequeued_at_ms: u64,
    /// Visibility timeout deadline (Unix ms).
    pub visibility_deadline_ms: u64,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time.
    pub enqueued_at_ms: u64,
    /// Message group ID if any.
    pub message_group_id: Option<String>,
}

impl PendingItem {
    /// Check if visibility timeout has expired.
    pub fn is_visibility_expired(&self) -> bool {
        verified::is_visibility_expired(self.visibility_deadline_ms, now_unix_ms())
    }
}

/// An item in the dead letter queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQItem {
    /// Original item ID.
    pub item_id: u64,
    /// Original payload.
    pub payload: Vec<u8>,
    /// Original enqueue time.
    pub enqueued_at_ms: u64,
    /// Delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason for moving to DLQ.
    pub reason: DLQReason,
    /// Time moved to DLQ (Unix ms).
    pub moved_at_ms: u64,
    /// Last error message (if any).
    pub last_error: Option<String>,
}

/// Reason for moving an item to the dead letter queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DLQReason {
    /// Exceeded max delivery attempts.
    MaxDeliveryAttemptsExceeded,
    /// Explicitly rejected by consumer.
    ExplicitlyRejected,
    /// Item expired while pending.
    ExpiredWhilePending,
}

/// Deduplication entry for exactly-once delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationEntry {
    /// Deduplication ID.
    pub dedup_id: String,
    /// Item ID that was created.
    pub item_id: u64,
    /// Expiration time (Unix ms).
    pub expires_at_ms: u64,
}

impl DeduplicationEntry {
    /// Check if entry has expired.
    pub fn is_expired(&self) -> bool {
        verified::is_dedup_entry_expired(self.expires_at_ms, now_unix_ms())
    }
}

/// Configuration for creating a queue.
#[derive(Debug, Clone, Default)]
pub struct QueueConfig {
    /// Default visibility timeout in milliseconds.
    pub default_visibility_timeout_ms: Option<u64>,
    /// Default item TTL in milliseconds (0 = no expiration).
    pub default_ttl_ms: Option<u64>,
    /// Maximum delivery attempts before DLQ (0 = no limit).
    pub max_delivery_attempts: Option<u32>,
}

/// Options for enqueuing an item.
#[derive(Debug, Clone, Default)]
pub struct EnqueueOptions {
    /// Optional TTL in milliseconds (overrides queue default).
    pub ttl_ms: Option<u64>,
    /// Optional message group ID for FIFO ordering.
    pub message_group_id: Option<String>,
    /// Optional deduplication ID.
    pub deduplication_id: Option<String>,
}

/// Result of a dequeue operation.
#[derive(Debug, Clone)]
pub struct DequeuedItem {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Receipt handle for ack/nack.
    pub receipt_handle: String,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Visibility deadline (Unix ms).
    pub visibility_deadline_ms: u64,
}

/// Queue status information.
#[derive(Debug, Clone, Default)]
pub struct QueueStatus {
    /// Whether the queue exists.
    pub exists: bool,
    /// Approximate number of visible items.
    pub visible_count: u64,
    /// Approximate number of pending items.
    pub pending_count: u64,
    /// Approximate number of DLQ items.
    pub dlq_count: u64,
    /// Total items enqueued.
    pub total_enqueued: u64,
    /// Total items acked.
    pub total_acked: u64,
    /// Total items moved to DLQ.
    pub total_dlq: u64,
}
