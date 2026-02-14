//! Key generation functions for distributed queue operations.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_state_spec.rs` for proofs.

/// Queue key prefix constant.
pub const QUEUE_PREFIX: &str = "__queue:";

/// Generate the queue metadata key.
///
/// # Example
///
/// ```ignore
/// assert_eq!(queue_metadata_key("orders"), "__queue:orders");
/// ```
#[inline]
pub fn queue_metadata_key(name: &str) -> String {
    format!("{}{}", QUEUE_PREFIX, name)
}

/// Generate the key for a queue item.
///
/// Items are stored with zero-padded IDs for lexicographic ordering.
///
/// # Example
///
/// ```ignore
/// assert_eq!(item_key("orders", 42), "__queue:orders:items:00000000000000000042");
/// ```
#[inline]
pub fn item_key(name: &str, item_id: u64) -> String {
    format!("{}{}:items:{:020}", QUEUE_PREFIX, name, item_id)
}

/// Generate the prefix for scanning all items in a queue.
///
/// # Example
///
/// ```ignore
/// assert_eq!(items_prefix("orders"), "__queue:orders:items:");
/// ```
#[inline]
pub fn items_prefix(name: &str) -> String {
    format!("{}{}:items:", QUEUE_PREFIX, name)
}

/// Generate the key for a pending item.
///
/// # Example
///
/// ```ignore
/// assert_eq!(pending_key("orders", 42), "__queue:orders:pending:00000000000000000042");
/// ```
#[inline]
pub fn pending_key(name: &str, item_id: u64) -> String {
    format!("{}{}:pending:{:020}", QUEUE_PREFIX, name, item_id)
}

/// Generate the prefix for scanning all pending items in a queue.
///
/// # Example
///
/// ```ignore
/// assert_eq!(pending_prefix("orders"), "__queue:orders:pending:");
/// ```
#[inline]
pub fn pending_prefix(name: &str) -> String {
    format!("{}{}:pending:", QUEUE_PREFIX, name)
}

/// Generate the key for a dead letter queue item.
///
/// # Example
///
/// ```ignore
/// assert_eq!(dlq_key("orders", 42), "__queue:orders:dlq:00000000000000000042");
/// ```
#[inline]
pub fn dlq_key(name: &str, item_id: u64) -> String {
    format!("{}{}:dlq:{:020}", QUEUE_PREFIX, name, item_id)
}

/// Generate the prefix for scanning all DLQ items in a queue.
///
/// # Example
///
/// ```ignore
/// assert_eq!(dlq_prefix("orders"), "__queue:orders:dlq:");
/// ```
#[inline]
pub fn dlq_prefix(name: &str) -> String {
    format!("{}{}:dlq:", QUEUE_PREFIX, name)
}

/// Generate the key for a deduplication entry.
///
/// # Example
///
/// ```ignore
/// assert_eq!(dedup_key("orders", "order-123"), "__queue:orders:dedup:order-123");
/// ```
#[inline]
pub fn dedup_key(name: &str, dedup_id: &str) -> String {
    format!("{}{}:dedup:{}", QUEUE_PREFIX, name, dedup_id)
}

/// Generate the prefix for scanning all dedup entries in a queue.
///
/// # Example
///
/// ```ignore
/// assert_eq!(dedup_prefix("orders"), "__queue:orders:dedup:");
/// ```
#[inline]
pub fn dedup_prefix(name: &str) -> String {
    format!("{}{}:dedup:", QUEUE_PREFIX, name)
}

/// Generate the sequence key for item ID generation.
///
/// # Example
///
/// ```ignore
/// assert_eq!(sequence_key("orders"), "__queue:orders:seq");
/// ```
#[inline]
pub fn sequence_key(name: &str) -> String {
    format!("{}{}:seq", QUEUE_PREFIX, name)
}
