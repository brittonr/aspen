//! Enqueue operations and item construction functions.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_enqueue_spec.rs` for proofs.
//!
//! # Tiger Style
//!
//! Uses saturating arithmetic for all calculations.

use crate::queue::PendingItem;
use crate::queue::QueueItem;

/// Create a queue item from a pending item for return to the queue.
///
/// Used when:
/// - Visibility timeout expires (item redelivered)
/// - Consumer nacks without moving to DLQ
/// - Consumer releases unchanged
///
/// # Arguments
///
/// * `pending` - The pending item to convert
/// * `decrement_attempts` - If true, decrements delivery_attempts by 1 (for release_unchanged)
///
/// # Returns
///
/// A new `QueueItem` ready to be re-enqueued.
///
/// # Tiger Style
///
/// - Uses saturating_sub for decrement to prevent underflow
/// - Clears deduplication_id to prevent false duplicate detection on retry
#[inline]
pub fn create_queue_item_from_pending(pending: &PendingItem, decrement_attempts: bool) -> QueueItem {
    let delivery_attempts = if decrement_attempts {
        pending.delivery_attempts.saturating_sub(1)
    } else {
        pending.delivery_attempts
    };

    assert!(pending.item_id > 0, "QUEUE: pending item must have positive ID, got {}", pending.item_id);
    if decrement_attempts {
        assert!(
            delivery_attempts <= pending.delivery_attempts,
            "QUEUE: decremented delivery_attempts ({delivery_attempts}) must be <= original ({})",
            pending.delivery_attempts
        );
    }

    QueueItem {
        item_id: pending.item_id,
        payload: pending.payload.clone(),
        enqueued_at_ms: pending.enqueued_at_ms,
        expires_at_ms: 0, // Reset expiration on retry
        delivery_attempts,
        message_group_id: pending.message_group_id.clone(),
        deduplication_id: None, // Don't dedupe retries
    }
}

/// Generate a receipt handle from components.
///
/// Receipt handles have the format: `{item_id}:{timestamp}:{random}`
///
/// # Arguments
///
/// * `item_id` - The queue item ID
/// * `timestamp_ms` - Current timestamp in milliseconds
/// * `random_value` - A random value for uniqueness
///
/// # Example
///
/// ```ignore
/// let handle = generate_receipt_handle(123, 1000, 456);
/// assert_eq!(handle, "123:1000:456");
/// ```
#[inline]
pub fn generate_receipt_handle(item_id: u64, timestamp_ms: u64, random_value: u64) -> String {
    format!("{}:{}:{}", item_id, timestamp_ms, random_value)
}

/// Parse the item ID from a receipt handle.
///
/// Receipt handles have the format: `{item_id}:{timestamp}:{random}`
///
/// # Arguments
///
/// * `receipt_handle` - The receipt handle string
///
/// # Returns
///
/// The item ID if successfully parsed, None otherwise.
///
/// # Example
///
/// ```ignore
/// assert_eq!(parse_receipt_handle("123:456:789"), Some(123));
/// assert_eq!(parse_receipt_handle("invalid"), None);
/// assert_eq!(parse_receipt_handle(""), None);
/// ```
#[inline]
pub fn parse_receipt_handle(receipt_handle: &str) -> Option<u64> {
    let parts: Vec<&str> = receipt_handle.split(':').collect();
    if parts.is_empty() {
        return None;
    }
    parts[0].parse().ok()
}

/// Allocate the next item ID.
///
/// # Arguments
///
/// * `next_id` - Current next ID value
///
/// # Returns
///
/// Tuple of (allocated ID, new next ID). Saturates at u64::MAX.
#[inline]
pub fn allocate_next_id(next_id: u64) -> (u64, u64) {
    (next_id, next_id.saturating_add(1))
}

/// Allocate the next item ID (alternative naming).
#[inline]
pub fn allocate_item_id(current_next_id: u64) -> (u64, u64) {
    (current_next_id, current_next_id.saturating_add(1))
}

/// Check if next ID can be allocated (no overflow).
#[inline]
pub fn can_allocate_id(next_id: u64) -> bool {
    next_id < u64::MAX
}
