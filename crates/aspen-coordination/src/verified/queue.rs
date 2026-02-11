//! Pure queue computation functions.
//!
//! This module contains pure functions for distributed queue operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification
//! - Explicit error types (no panics)

use crate::queue::DLQReason;
use crate::queue::PendingItem;
use crate::queue::QueueItem;

/// Queue key prefix constant.
pub const QUEUE_PREFIX: &str = "__queue:";

// ============================================================================
// Key Generation
// ============================================================================

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

// ============================================================================
// Visibility Timeout
// ============================================================================

/// Compute the visibility deadline from current time and timeout.
///
/// # Arguments
///
/// * `current_time_ms` - Current time in Unix milliseconds
/// * `visibility_timeout_ms` - Visibility timeout in milliseconds
///
/// # Returns
///
/// Visibility deadline in Unix milliseconds.
///
/// # Tiger Style
///
/// Uses saturating_add to prevent overflow.
#[inline]
pub fn compute_visibility_deadline(current_time_ms: u64, visibility_timeout_ms: u64) -> u64 {
    current_time_ms.saturating_add(visibility_timeout_ms)
}

// ============================================================================
// Receipt Handle
// ============================================================================

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

// ============================================================================
// Message Group Filtering
// ============================================================================

/// Check if an item should be skipped due to message group ordering.
///
/// In FIFO queues with message groups, only one item per group can be
/// in-flight at a time. This function checks if the item's group is
/// currently being processed.
///
/// # Arguments
///
/// * `pending_groups` - List of message groups currently pending
/// * `message_group_id` - The item's message group (None if no group)
///
/// # Returns
///
/// `true` if the item should be skipped because its group is pending.
///
/// # Example
///
/// ```ignore
/// let pending = vec!["group-a".to_string(), "group-b".to_string()];
/// assert!(should_skip_for_message_group(&pending, &Some("group-a".to_string())));
/// assert!(!should_skip_for_message_group(&pending, &Some("group-c".to_string())));
/// assert!(!should_skip_for_message_group(&pending, &None));
/// ```
#[inline]
pub fn should_skip_for_message_group(pending_groups: &[String], message_group_id: &Option<String>) -> bool {
    match message_group_id {
        Some(group) => pending_groups.contains(group),
        None => false,
    }
}

/// Check if delivery attempts have been exceeded.
///
/// # Arguments
///
/// * `delivery_attempts` - Current number of delivery attempts
/// * `max_delivery_attempts` - Maximum allowed attempts (0 = no limit)
///
/// # Returns
///
/// `true` if max delivery attempts exceeded.
#[inline]
pub fn has_exceeded_max_delivery_attempts(delivery_attempts: u32, max_delivery_attempts: u32) -> bool {
    max_delivery_attempts > 0 && delivery_attempts >= max_delivery_attempts
}

/// Compute delivery attempts for a requeue operation.
///
/// # Arguments
///
/// * `current_attempts` - Current delivery attempts
/// * `increment` - Whether to increment attempts (false for release_unchanged)
///
/// # Returns
///
/// New delivery attempts value.
#[inline]
pub fn compute_requeue_delivery_attempts(current_attempts: u32, increment: bool) -> u32 {
    if increment {
        current_attempts.saturating_add(1)
    } else {
        current_attempts.saturating_sub(1)
    }
}

// ============================================================================
// Expiry Checks
// ============================================================================

/// Check if a queue item has expired based on its TTL.
///
/// # Arguments
///
/// * `expires_at_ms` - Expiration deadline in Unix milliseconds (0 = no expiration)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the item has expired.
///
/// # Example
///
/// ```ignore
/// // Item with no expiration
/// assert!(!is_queue_item_expired(0, 1000));
///
/// // Expired item
/// assert!(is_queue_item_expired(1000, 2000));
///
/// // Active item
/// assert!(!is_queue_item_expired(2000, 1000));
/// ```
#[inline]
pub fn is_queue_item_expired(expires_at_ms: u64, now_ms: u64) -> bool {
    expires_at_ms > 0 && now_ms > expires_at_ms
}

/// Check if a pending item's visibility timeout has expired.
///
/// When the visibility timeout expires, the item should be returned
/// to the queue for redelivery.
///
/// # Arguments
///
/// * `visibility_deadline_ms` - Visibility deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the visibility timeout has expired.
#[inline]
pub fn is_visibility_expired(visibility_deadline_ms: u64, now_ms: u64) -> bool {
    now_ms > visibility_deadline_ms
}

/// Check if a deduplication entry has expired.
///
/// # Arguments
///
/// * `expires_at_ms` - Expiration deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the dedup entry has expired.
#[inline]
pub fn is_dedup_entry_expired(expires_at_ms: u64, now_ms: u64) -> bool {
    now_ms > expires_at_ms
}

// ============================================================================
// DLQ Decision
// ============================================================================

/// Decision about whether to move an item to the dead letter queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DLQDecision {
    /// Whether the item should be moved to DLQ.
    pub should_move: bool,
    /// Reason for moving to DLQ (if should_move is true).
    pub reason: Option<DLQReason>,
}

impl DLQDecision {
    /// Create a decision to not move to DLQ.
    #[inline]
    pub fn keep() -> Self {
        Self {
            should_move: false,
            reason: None,
        }
    }

    /// Create a decision to move to DLQ with the given reason.
    #[inline]
    pub fn move_to_dlq(reason: DLQReason) -> Self {
        Self {
            should_move: true,
            reason: Some(reason),
        }
    }
}

/// Determine whether an item should be moved to the dead letter queue (extended version).
///
/// An item is moved to DLQ if:
/// - It is explicitly rejected by the consumer, OR
/// - It has exceeded the maximum delivery attempts
///
/// # Arguments
///
/// * `delivery_count` - Number of times the item has been delivered
/// * `max_delivery_attempts` - Maximum allowed attempts (0 = no limit)
/// * `explicit_reject` - Whether the consumer explicitly rejected the item
///
/// # Returns
///
/// A `DLQDecision` indicating whether to move and the reason.
///
/// # Example
///
/// ```ignore
/// // Explicit rejection
/// let decision = should_move_to_dlq_with_reason(1, 3, true);
/// assert!(decision.should_move);
/// assert_eq!(decision.reason, Some(DLQReason::ExplicitlyRejected));
///
/// // Max attempts exceeded
/// let decision = should_move_to_dlq_with_reason(3, 3, false);
/// assert!(decision.should_move);
///
/// // Still has attempts remaining
/// let decision = should_move_to_dlq_with_reason(2, 3, false);
/// assert!(!decision.should_move);
/// ```
#[inline]
pub fn should_move_to_dlq_with_reason(
    delivery_count: u32,
    max_delivery_attempts: u32,
    explicit_reject: bool,
) -> DLQDecision {
    if explicit_reject {
        return DLQDecision::move_to_dlq(DLQReason::ExplicitlyRejected);
    }

    if max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts {
        return DLQDecision::move_to_dlq(DLQReason::MaxDeliveryAttemptsExceeded);
    }

    DLQDecision::keep()
}

/// Check if item should be moved to DLQ (Verus-aligned).
///
/// # Arguments
///
/// * `delivery_count` - Current delivery count
/// * `max_delivery_attempts` - Maximum allowed attempts (0 = unlimited)
///
/// # Returns
///
/// `true` if item should be dead-lettered.
#[inline]
pub fn should_move_to_dlq(delivery_count: u32, max_delivery_attempts: u32) -> bool {
    max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts
}

// ============================================================================
// Item Construction
// ============================================================================

/// Compute the expiration time for a queue item.
///
/// # Arguments
///
/// * `ttl_ms` - Requested TTL in milliseconds (0 = use default)
/// * `default_ttl_ms` - Queue's default TTL (0 = no expiration)
/// * `max_ttl_ms` - Maximum allowed TTL
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Expiration deadline in Unix milliseconds (0 = no expiration).
///
/// # Tiger Style
///
/// - TTL is capped at max_ttl_ms
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_item_expiration(ttl_ms: u64, default_ttl_ms: u64, max_ttl_ms: u64, now_ms: u64) -> u64 {
    let effective_ttl = if ttl_ms > 0 { ttl_ms } else { default_ttl_ms };
    let capped_ttl = effective_ttl.min(max_ttl_ms);

    if capped_ttl > 0 {
        now_ms.saturating_add(capped_ttl)
    } else {
        0
    }
}

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

// ============================================================================
// Receipt Handle
// ============================================================================

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

// ============================================================================
// Batch Size Computation
// ============================================================================

/// Compute the effective batch size for dequeue operations.
///
/// Caps the requested batch size at the maximum allowed.
///
/// # Arguments
///
/// * `requested` - Number of items requested
/// * `max_batch_size` - Maximum allowed batch size
///
/// # Returns
///
/// The effective batch size to use.
#[inline]
pub fn compute_dequeue_batch_size(requested: u32, max_batch_size: u32) -> u32 {
    requested.min(max_batch_size)
}

/// Compute the effective visibility timeout.
///
/// Caps the requested timeout at the maximum allowed.
///
/// # Arguments
///
/// * `requested_ms` - Requested visibility timeout in milliseconds
/// * `max_timeout_ms` - Maximum allowed timeout
///
/// # Returns
///
/// The effective visibility timeout to use.
#[inline]
pub fn compute_effective_visibility_timeout(requested_ms: u64, max_timeout_ms: u64) -> u64 {
    requested_ms.min(max_timeout_ms)
}

// ============================================================================
// Message Group FIFO Ordering
// ============================================================================

/// Information about pending message groups with timestamps.
#[derive(Debug, Clone)]
pub struct PendingGroupInfo {
    /// Group ID.
    pub group_id: String,
    /// When this group started processing (Unix ms).
    pub started_at_ms: u64,
}

/// Check if a message group can be dequeued based on pending state.
///
/// A message group can be dequeued if:
/// - The item has no message group, OR
/// - The group is not currently pending, OR
/// - The pending group has expired (visibility timeout passed)
///
/// # Arguments
///
/// * `message_group_id` - The item's message group (None if no group)
/// * `pending_groups` - Map of group IDs to their pending start times
/// * `visibility_timeout_ms` - Visibility timeout in milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the item can be dequeued.
#[inline]
pub fn can_dequeue_from_group(
    message_group_id: Option<&str>,
    pending_groups: &[PendingGroupInfo],
    visibility_timeout_ms: u64,
    now_ms: u64,
) -> bool {
    match message_group_id {
        None => true, // No group, can always dequeue
        Some(group_id) => {
            // Check if group is pending
            match pending_groups.iter().find(|g| g.group_id == group_id) {
                None => true, // Group not pending
                Some(pending) => {
                    // Check if the pending lock has expired
                    let deadline = pending.started_at_ms.saturating_add(visibility_timeout_ms);
                    now_ms > deadline
                }
            }
        }
    }
}

// ============================================================================
// Requeue Priority
// ============================================================================

/// Priority level for requeued items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequeuePriority {
    /// Normal priority (back of queue).
    Normal,
    /// Elevated priority (front of queue).
    Elevated,
    /// High priority (immediate processing).
    High,
}

/// Compute the requeue priority for a failed item.
///
/// Items that have failed fewer times get lower priority (back of queue).
/// Items approaching max attempts get higher priority (for faster DLQ decision).
///
/// # Arguments
///
/// * `delivery_attempts` - Number of times this item has been delivered
/// * `max_delivery_attempts` - Maximum allowed attempts (0 = no limit)
///
/// # Returns
///
/// Priority level for the requeued item.
#[inline]
pub fn compute_requeue_priority(delivery_attempts: u32, max_delivery_attempts: u32) -> RequeuePriority {
    if max_delivery_attempts == 0 {
        // No limit, always normal priority
        return RequeuePriority::Normal;
    }

    let remaining = max_delivery_attempts.saturating_sub(delivery_attempts);
    let total = max_delivery_attempts;

    // Last attempt: high priority (needs fast resolution)
    if remaining <= 1 {
        return RequeuePriority::High;
    }

    // Less than half attempts remaining: elevated priority
    if remaining <= total / 2 {
        return RequeuePriority::Elevated;
    }

    RequeuePriority::Normal
}

// ============================================================================
// Dequeue Eligibility
// ============================================================================

/// Result of checking item eligibility for dequeue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DequeueEligibility {
    /// Item can be dequeued.
    Eligible,
    /// Item has expired (TTL exceeded).
    Expired,
    /// Item's message group is pending.
    GroupPending,
    /// Item has exceeded max delivery attempts.
    MaxAttemptsExceeded,
}

/// Check if an item is eligible for dequeue.
///
/// Combines multiple eligibility checks into a single function.
///
/// # Arguments
///
/// * `expires_at_ms` - Item expiration time (0 = no expiration)
/// * `delivery_attempts` - Number of delivery attempts
/// * `max_delivery_attempts` - Maximum allowed (0 = no limit)
/// * `message_group_id` - Item's message group
/// * `pending_groups` - Currently pending message groups
/// * `now_ms` - Current time
///
/// # Returns
///
/// Eligibility result indicating whether item can be dequeued.
#[inline]
pub fn check_dequeue_eligibility(
    expires_at_ms: u64,
    delivery_attempts: u32,
    max_delivery_attempts: u32,
    message_group_id: Option<&str>,
    pending_groups: &[String],
    now_ms: u64,
) -> DequeueEligibility {
    // Check expiration
    if is_queue_item_expired(expires_at_ms, now_ms) {
        return DequeueEligibility::Expired;
    }

    // Check max delivery attempts
    if has_exceeded_max_delivery_attempts(delivery_attempts, max_delivery_attempts) {
        return DequeueEligibility::MaxAttemptsExceeded;
    }

    // Check message group
    if let Some(group) = message_group_id
        && pending_groups.iter().any(|g| g == group)
    {
        return DequeueEligibility::GroupPending;
    }

    DequeueEligibility::Eligible
}

// ============================================================================
// Dequeue Validation (Verus-aligned)
// ============================================================================

/// Check if dequeue parameters are valid.
///
/// # Arguments
///
/// * `max_items` - Maximum items to dequeue
/// * `visibility_timeout_ms` - Visibility timeout
/// * `consumer_id_len` - Length of consumer ID
/// * `current_time_ms` - Current time
///
/// # Returns
///
/// `true` if parameters are valid for dequeue.
#[inline]
pub fn are_dequeue_params_valid(
    max_items: u32,
    visibility_timeout_ms: u64,
    consumer_id_len: u64,
    current_time_ms: u64,
) -> bool {
    max_items > 0
        && max_items <= 100
        && visibility_timeout_ms > 0
        && visibility_timeout_ms <= 3_600_000
        && consumer_id_len > 0
        && current_time_ms <= u64::MAX - visibility_timeout_ms
}

/// Calculate deduplication expiration time.
///
/// # Arguments
///
/// * `enqueue_time_ms` - Time of enqueue in Unix milliseconds
/// * `dedup_window_ms` - Deduplication window in milliseconds
///
/// # Returns
///
/// Expiration time for dedup entry.
#[inline]
pub fn calculate_dedup_expiration(enqueue_time_ms: u64, dedup_window_ms: u64) -> u64 {
    enqueue_time_ms.saturating_add(dedup_window_ms)
}

/// Compute deduplication entry expiration.
///
/// Uses fixed 5 minute dedup TTL (300,000 ms).
///
/// # Arguments
///
/// * `current_time_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Dedup entry expiration time.
#[inline]
pub fn compute_dedup_expiration(current_time_ms: u64) -> u64 {
    current_time_ms.saturating_add(300_000)
}

/// Check if dedup TTL can be computed without overflow.
///
/// Uses fixed 5 minute dedup window (300,000 ms).
#[inline]
pub fn can_compute_dedup_ttl(current_time_ms: u64) -> bool {
    current_time_ms <= u64::MAX - 300_000
}

// ============================================================================
// Ack/Nack/Redrive Validation (Verus-aligned)
// ============================================================================

/// Check if acknowledgment is valid.
#[inline]
pub fn is_ack_valid(is_inflight: bool, receipt_matches: bool) -> bool {
    is_inflight && receipt_matches
}

/// Check if nack is valid.
#[inline]
pub fn is_nack_valid(is_inflight: bool, receipt_matches: bool) -> bool {
    is_inflight && receipt_matches
}

/// Check if redrive is valid.
#[inline]
pub fn is_redrive_valid(is_in_dlq: bool) -> bool {
    is_in_dlq
}

/// Check if acquire (dequeue) is valid.
#[inline]
pub fn is_acquire_valid(queue_not_empty: bool, consumer_id_valid: bool) -> bool {
    queue_not_empty && consumer_id_valid
}

// ============================================================================
// Delivery Count Operations (Verus-aligned)
// ============================================================================

/// Increment delivery count for dequeue.
#[inline]
pub fn increment_delivery_count_for_dequeue(current_count: u32) -> u32 {
    current_count.saturating_add(1)
}

/// Check if delivery count can be incremented.
#[inline]
pub fn can_increment_delivery_count(current_count: u32) -> bool {
    current_count < u32::MAX
}

/// Check if a message is a duplicate based on dedup cache.
///
/// # Arguments
///
/// * `has_dedup_entry` - Whether dedup entry exists
/// * `dedup_expires_at_ms` - Expiration of existing entry
/// * `current_time_ms` - Current time
///
/// # Returns
///
/// `true` if message is a duplicate.
#[inline]
pub fn is_duplicate_message(
    has_dedup_entry: bool,
    dedup_expires_at_ms: u64,
    current_time_ms: u64,
) -> bool {
    has_dedup_entry && dedup_expires_at_ms > current_time_ms
}

// ============================================================================
// DLQ Decisions (Verus-aligned)
// ============================================================================

/// Check if item should be moved to DLQ (exec version, Verus-aligned).
#[inline]
pub fn should_move_to_dlq_exec(delivery_count: u32, max_delivery_attempts: u32) -> bool {
    max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts
}

// ============================================================================
// Visibility Timeout (Verus-aligned)
// ============================================================================

/// Check if visibility timeout has expired (exec version).
#[inline]
pub fn is_visibility_expired_exec(visibility_deadline_ms: u64, current_time_ms: u64) -> bool {
    current_time_ms > visibility_deadline_ms
}

/// Alias for is_visibility_expired for consistency.
#[inline]
pub fn is_visibility_timeout_expired(visibility_deadline_ms: u64, current_time_ms: u64) -> bool {
    current_time_ms > visibility_deadline_ms
}

/// Calculate time until visibility expires.
///
/// # Arguments
///
/// * `visibility_deadline_ms` - Visibility deadline
/// * `current_time_ms` - Current time
///
/// # Returns
///
/// Time remaining until expiration (0 if already expired).
#[inline]
pub fn time_until_visibility_expires(visibility_deadline_ms: u64, current_time_ms: u64) -> u64 {
    visibility_deadline_ms.saturating_sub(current_time_ms)
}

/// Calculate visibility deadline.
#[inline]
pub fn calculate_visibility_deadline(dequeue_time_ms: u64, visibility_timeout_ms: u64) -> u64 {
    dequeue_time_ms.saturating_add(visibility_timeout_ms)
}

/// Calculate extended visibility deadline.
#[inline]
pub fn calculate_extended_deadline(current_time_ms: u64, extension_ms: u64) -> u64 {
    current_time_ms.saturating_add(extension_ms)
}

/// Check if visibility can be extended (production version).
#[inline]
pub fn can_extend_visibility(is_inflight: bool, receipt_matches: bool, additional_timeout_ms: u64) -> bool {
    is_inflight && receipt_matches && additional_timeout_ms > 0 && additional_timeout_ms <= 3_600_000
}

/// Check if extend visibility is valid (Verus-aligned version).
///
/// # Arguments
///
/// * `current_deadline_ms` - Current visibility deadline
/// * `requested_extension_ms` - Requested extension
/// * `max_visibility_ms` - Maximum allowed visibility timeout
/// * `current_time_ms` - Current time
///
/// # Returns
///
/// `true` if extension is valid.
#[inline]
pub fn is_extend_visibility_valid(
    current_deadline_ms: u64,
    requested_extension_ms: u64,
    max_visibility_ms: u64,
    current_time_ms: u64,
) -> bool {
    requested_extension_ms <= max_visibility_ms && current_deadline_ms > current_time_ms
}

/// Compute new visibility deadline.
///
/// # Arguments
///
/// * `current_time_ms` - Current time
/// * `additional_timeout_ms` - Additional timeout to add
///
/// # Returns
///
/// New visibility deadline (saturating at u64::MAX).
#[inline]
pub fn compute_extended_deadline(current_time_ms: u64, additional_timeout_ms: u64) -> u64 {
    current_time_ms.saturating_add(additional_timeout_ms)
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

/// Check if TTL computation would overflow.
#[inline]
pub fn can_compute_ttl(current_time_ms: u64, ttl_ms: u64) -> bool {
    ttl_ms == 0 || current_time_ms <= u64::MAX - ttl_ms
}

/// Check if batch size is valid.
#[inline]
pub fn is_batch_size_valid(batch_size: u32) -> bool {
    batch_size <= 100
}

/// Check if payload size is valid.
#[inline]
pub fn is_payload_size_valid(payload_len: u64) -> bool {
    payload_len <= 1_000_000
}

/// Check if a queue item has expired.
#[inline]
pub fn is_item_expired(expires_at_ms: u64, current_time_ms: u64) -> bool {
    expires_at_ms > 0 && current_time_ms > expires_at_ms
}

/// Check if a deduplication entry has expired (Verus-aligned).
#[inline]
pub fn is_dedup_expired(dedup_expires_at_ms: u64, current_time_ms: u64) -> bool {
    current_time_ms > dedup_expires_at_ms
}

/// Check if a queue is empty.
#[inline]
pub fn is_queue_empty(pending_count: u32, inflight_count: u32) -> bool {
    pending_count == 0 && inflight_count == 0
}

/// Increment delivery count (Verus-aligned).
#[inline]
pub fn increment_delivery_count(current_count: u32) -> u32 {
    current_count.saturating_add(1)
}

/// Decrement delivery count for release unchanged.
#[inline]
pub fn decrement_delivery_count_for_release(delivery_count: u32) -> u32 {
    delivery_count.saturating_sub(1)
}

/// Determine nack action based on delivery count.
///
/// # Arguments
///
/// * `delivery_count` - Current delivery count
/// * `max_delivery_attempts` - Maximum delivery attempts
/// * `explicit_dlq` - Whether DLQ was explicitly requested
///
/// # Returns
///
/// `true` if should move to DLQ.
#[inline]
pub fn should_nack_to_dlq(delivery_count: u32, max_delivery_attempts: u32, explicit_dlq: bool) -> bool {
    explicit_dlq || (max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Key Generation Tests
    // ========================================================================

    #[test]
    fn test_queue_metadata_key() {
        assert_eq!(queue_metadata_key("orders"), "__queue:orders");
        assert_eq!(queue_metadata_key("my-queue"), "__queue:my-queue");
        assert_eq!(queue_metadata_key(""), "__queue:");
    }

    #[test]
    fn test_item_key() {
        assert_eq!(item_key("orders", 0), "__queue:orders:items:00000000000000000000");
        assert_eq!(item_key("orders", 42), "__queue:orders:items:00000000000000000042");
        assert_eq!(item_key("q", u64::MAX), "__queue:q:items:18446744073709551615");
    }

    #[test]
    fn test_items_prefix() {
        assert_eq!(items_prefix("orders"), "__queue:orders:items:");
    }

    #[test]
    fn test_pending_key() {
        assert_eq!(pending_key("orders", 42), "__queue:orders:pending:00000000000000000042");
    }

    #[test]
    fn test_pending_prefix() {
        assert_eq!(pending_prefix("orders"), "__queue:orders:pending:");
    }

    #[test]
    fn test_dlq_key() {
        assert_eq!(dlq_key("orders", 42), "__queue:orders:dlq:00000000000000000042");
    }

    #[test]
    fn test_dlq_prefix() {
        assert_eq!(dlq_prefix("orders"), "__queue:orders:dlq:");
    }

    #[test]
    fn test_dedup_key() {
        assert_eq!(dedup_key("orders", "order-123"), "__queue:orders:dedup:order-123");
    }

    #[test]
    fn test_dedup_prefix() {
        assert_eq!(dedup_prefix("orders"), "__queue:orders:dedup:");
    }

    #[test]
    fn test_sequence_key() {
        assert_eq!(sequence_key("orders"), "__queue:orders:seq");
    }

    // ========================================================================
    // Visibility Timeout Tests
    // ========================================================================

    #[test]
    fn test_compute_visibility_deadline() {
        assert_eq!(compute_visibility_deadline(1000, 5000), 6000);
        assert_eq!(compute_visibility_deadline(0, 0), 0);
    }

    #[test]
    fn test_compute_visibility_deadline_overflow() {
        assert_eq!(compute_visibility_deadline(u64::MAX, 1), u64::MAX);
    }

    // ========================================================================
    // Receipt Handle Tests
    // ========================================================================

    #[test]
    fn test_generate_receipt_handle() {
        assert_eq!(generate_receipt_handle(123, 1000, 456), "123:1000:456");
        assert_eq!(generate_receipt_handle(0, 0, 0), "0:0:0");
    }

    #[test]
    fn test_generate_and_parse_receipt_handle_roundtrip() {
        let handle = generate_receipt_handle(12345, 1000, 999);
        assert_eq!(parse_receipt_handle(&handle), Some(12345));
    }

    // ========================================================================
    // Message Group Tests
    // ========================================================================

    #[test]
    fn test_should_skip_for_message_group_pending() {
        let pending = vec!["group-a".to_string(), "group-b".to_string()];
        assert!(should_skip_for_message_group(&pending, &Some("group-a".to_string())));
        assert!(should_skip_for_message_group(&pending, &Some("group-b".to_string())));
    }

    #[test]
    fn test_should_skip_for_message_group_not_pending() {
        let pending = vec!["group-a".to_string()];
        assert!(!should_skip_for_message_group(&pending, &Some("group-c".to_string())));
    }

    #[test]
    fn test_should_skip_for_message_group_none() {
        let pending = vec!["group-a".to_string()];
        assert!(!should_skip_for_message_group(&pending, &None));
    }

    #[test]
    fn test_should_skip_for_message_group_empty_pending() {
        let pending: Vec<String> = vec![];
        assert!(!should_skip_for_message_group(&pending, &Some("group-a".to_string())));
    }

    // ========================================================================
    // Delivery Attempts Tests
    // ========================================================================

    #[test]
    fn test_has_exceeded_max_delivery_attempts() {
        assert!(has_exceeded_max_delivery_attempts(3, 3));
        assert!(has_exceeded_max_delivery_attempts(4, 3));
        assert!(!has_exceeded_max_delivery_attempts(2, 3));
    }

    #[test]
    fn test_has_exceeded_max_delivery_attempts_no_limit() {
        assert!(!has_exceeded_max_delivery_attempts(100, 0));
        assert!(!has_exceeded_max_delivery_attempts(u32::MAX, 0));
    }

    #[test]
    fn test_compute_requeue_delivery_attempts_increment() {
        assert_eq!(compute_requeue_delivery_attempts(2, true), 3);
        assert_eq!(compute_requeue_delivery_attempts(0, true), 1);
    }

    #[test]
    fn test_compute_requeue_delivery_attempts_decrement() {
        assert_eq!(compute_requeue_delivery_attempts(2, false), 1);
        assert_eq!(compute_requeue_delivery_attempts(0, false), 0); // saturates
    }

    #[test]
    fn test_compute_requeue_delivery_attempts_overflow() {
        assert_eq!(compute_requeue_delivery_attempts(u32::MAX, true), u32::MAX);
    }

    // ========================================================================
    // Expiry Tests
    // ========================================================================

    #[test]
    fn test_queue_item_expired_no_expiration() {
        // expires_at_ms = 0 means no expiration
        assert!(!is_queue_item_expired(0, 1000));
        assert!(!is_queue_item_expired(0, u64::MAX));
    }

    #[test]
    fn test_queue_item_expired_past() {
        assert!(is_queue_item_expired(1000, 2000));
    }

    #[test]
    fn test_queue_item_expired_active() {
        assert!(!is_queue_item_expired(2000, 1000));
    }

    #[test]
    fn test_queue_item_expired_at_deadline() {
        // At exactly deadline, not yet expired
        assert!(!is_queue_item_expired(1000, 1000));
    }

    #[test]
    fn test_visibility_expired() {
        assert!(is_visibility_expired(1000, 2000));
        assert!(!is_visibility_expired(2000, 1000));
        assert!(!is_visibility_expired(1000, 1000));
    }

    #[test]
    fn test_dedup_entry_expired() {
        assert!(is_dedup_entry_expired(1000, 2000));
        assert!(!is_dedup_entry_expired(2000, 1000));
    }

    // ========================================================================
    // DLQ Decision Tests
    // ========================================================================

    #[test]
    fn test_dlq_explicit_reject() {
        let decision = should_move_to_dlq_with_reason(1, 3, true);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(DLQReason::ExplicitlyRejected));
    }

    #[test]
    fn test_dlq_max_attempts_exceeded() {
        let decision = should_move_to_dlq_with_reason(3, 3, false);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(DLQReason::MaxDeliveryAttemptsExceeded));
    }

    #[test]
    fn test_dlq_attempts_remaining() {
        let decision = should_move_to_dlq_with_reason(2, 3, false);
        assert!(!decision.should_move);
        assert_eq!(decision.reason, None);
    }

    #[test]
    fn test_dlq_no_limit() {
        // max_delivery_attempts = 0 means no limit
        let decision = should_move_to_dlq_with_reason(100, 0, false);
        assert!(!decision.should_move);
    }

    #[test]
    fn test_dlq_explicit_reject_overrides_attempts() {
        // Even with attempts remaining, explicit reject moves to DLQ
        let decision = should_move_to_dlq_with_reason(1, 10, true);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(DLQReason::ExplicitlyRejected));
    }

    #[test]
    fn test_dlq_verus_aligned() {
        // Test the Verus-aligned version (returns bool, no explicit_reject param)
        assert!(should_move_to_dlq(3, 3));  // At limit
        assert!(should_move_to_dlq(4, 3));  // Over limit
        assert!(!should_move_to_dlq(2, 3)); // Under limit
        assert!(!should_move_to_dlq(100, 0)); // No limit (0 = unlimited)
    }

    // ========================================================================
    // Item Expiration Tests
    // ========================================================================

    #[test]
    fn test_item_expiration_with_ttl() {
        let expires = compute_item_expiration(5000, 30000, 60000, 1000);
        assert_eq!(expires, 6000); // 1000 + 5000
    }

    #[test]
    fn test_item_expiration_uses_default() {
        let expires = compute_item_expiration(0, 30000, 60000, 1000);
        assert_eq!(expires, 31000); // 1000 + 30000
    }

    #[test]
    fn test_item_expiration_capped_at_max() {
        let expires = compute_item_expiration(100000, 30000, 60000, 1000);
        assert_eq!(expires, 61000); // 1000 + min(100000, 60000)
    }

    #[test]
    fn test_item_expiration_no_ttl() {
        let expires = compute_item_expiration(0, 0, 60000, 1000);
        assert_eq!(expires, 0); // No expiration
    }

    #[test]
    fn test_item_expiration_overflow_safety() {
        let expires = compute_item_expiration(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
        assert_eq!(expires, u64::MAX); // Saturates, doesn't panic
    }

    // ========================================================================
    // Pending to Queue Item Tests
    // ========================================================================

    #[test]
    fn test_create_queue_item_from_pending() {
        let pending = PendingItem {
            item_id: 123,
            payload: vec![1, 2, 3],
            consumer_id: "consumer".to_string(),
            receipt_handle: "handle".to_string(),
            dequeued_at_ms: 1000,
            visibility_deadline_ms: 2000,
            delivery_attempts: 2,
            enqueued_at_ms: 500,
            message_group_id: Some("group".to_string()),
        };

        let item = create_queue_item_from_pending(&pending, false);

        assert_eq!(item.item_id, 123);
        assert_eq!(item.payload, vec![1, 2, 3]);
        assert_eq!(item.enqueued_at_ms, 500);
        assert_eq!(item.expires_at_ms, 0); // Reset
        assert_eq!(item.delivery_attempts, 2);
        assert_eq!(item.message_group_id, Some("group".to_string()));
        assert_eq!(item.deduplication_id, None); // Cleared
    }

    #[test]
    fn test_create_queue_item_from_pending_decrement() {
        let pending = PendingItem {
            item_id: 123,
            payload: vec![],
            consumer_id: "consumer".to_string(),
            receipt_handle: "handle".to_string(),
            dequeued_at_ms: 1000,
            visibility_deadline_ms: 2000,
            delivery_attempts: 2,
            enqueued_at_ms: 500,
            message_group_id: None,
        };

        let item = create_queue_item_from_pending(&pending, true);
        assert_eq!(item.delivery_attempts, 1); // Decremented
    }

    #[test]
    fn test_create_queue_item_from_pending_decrement_underflow() {
        let pending = PendingItem {
            item_id: 123,
            payload: vec![],
            consumer_id: "consumer".to_string(),
            receipt_handle: "handle".to_string(),
            dequeued_at_ms: 1000,
            visibility_deadline_ms: 2000,
            delivery_attempts: 0, // Already at 0
            enqueued_at_ms: 500,
            message_group_id: None,
        };

        let item = create_queue_item_from_pending(&pending, true);
        assert_eq!(item.delivery_attempts, 0); // Saturates at 0, doesn't underflow
    }

    // ========================================================================
    // Receipt Handle Tests
    // ========================================================================

    #[test]
    fn test_parse_receipt_handle_valid() {
        assert_eq!(parse_receipt_handle("123:456:789"), Some(123));
        assert_eq!(parse_receipt_handle("0:0:0"), Some(0));
        assert_eq!(parse_receipt_handle("999"), Some(999));
    }

    #[test]
    fn test_parse_receipt_handle_invalid() {
        assert_eq!(parse_receipt_handle(""), None);
        assert_eq!(parse_receipt_handle("abc:456:789"), None);
        assert_eq!(parse_receipt_handle(":456:789"), None);
    }

    // ========================================================================
    // Batch Size Tests
    // ========================================================================

    #[test]
    fn test_compute_dequeue_batch_size_under_max() {
        assert_eq!(compute_dequeue_batch_size(5, 10), 5);
    }

    #[test]
    fn test_compute_dequeue_batch_size_at_max() {
        assert_eq!(compute_dequeue_batch_size(10, 10), 10);
    }

    #[test]
    fn test_compute_dequeue_batch_size_over_max() {
        assert_eq!(compute_dequeue_batch_size(20, 10), 10);
    }

    #[test]
    fn test_compute_effective_visibility_timeout() {
        assert_eq!(compute_effective_visibility_timeout(5000, 30000), 5000);
        assert_eq!(compute_effective_visibility_timeout(60000, 30000), 30000);
    }

    // ========================================================================
    // Message Group FIFO Tests
    // ========================================================================

    #[test]
    fn test_can_dequeue_from_group_no_group() {
        let pending: Vec<PendingGroupInfo> = vec![];
        assert!(can_dequeue_from_group(None, &pending, 30000, 1000));
    }

    #[test]
    fn test_can_dequeue_from_group_not_pending() {
        let pending: Vec<PendingGroupInfo> = vec![];
        assert!(can_dequeue_from_group(Some("group-a"), &pending, 30000, 1000));
    }

    #[test]
    fn test_can_dequeue_from_group_is_pending() {
        let pending = vec![PendingGroupInfo {
            group_id: "group-a".to_string(),
            started_at_ms: 1000,
        }];
        // Group is pending, and visibility hasn't expired
        assert!(!can_dequeue_from_group(Some("group-a"), &pending, 30000, 2000));
    }

    #[test]
    fn test_can_dequeue_from_group_pending_expired() {
        let pending = vec![PendingGroupInfo {
            group_id: "group-a".to_string(),
            started_at_ms: 1000,
        }];
        // Visibility expired (1000 + 30000 = 31000, now is 40000)
        assert!(can_dequeue_from_group(Some("group-a"), &pending, 30000, 40000));
    }

    // ========================================================================
    // Requeue Priority Tests
    // ========================================================================

    #[test]
    fn test_requeue_priority_no_limit() {
        assert_eq!(compute_requeue_priority(5, 0), RequeuePriority::Normal);
        assert_eq!(compute_requeue_priority(100, 0), RequeuePriority::Normal);
    }

    #[test]
    fn test_requeue_priority_normal() {
        // 2 attempts, max 5 -> 3 remaining, more than half
        assert_eq!(compute_requeue_priority(2, 5), RequeuePriority::Normal);
    }

    #[test]
    fn test_requeue_priority_elevated() {
        // 4 attempts, max 6 -> 2 remaining, less than half (3)
        assert_eq!(compute_requeue_priority(4, 6), RequeuePriority::Elevated);
    }

    #[test]
    fn test_requeue_priority_high() {
        // 4 attempts, max 5 -> 1 remaining, last attempt
        assert_eq!(compute_requeue_priority(4, 5), RequeuePriority::High);
    }

    #[test]
    fn test_requeue_priority_at_max() {
        // 5 attempts, max 5 -> 0 remaining
        assert_eq!(compute_requeue_priority(5, 5), RequeuePriority::High);
    }

    // ========================================================================
    // Dequeue Eligibility Tests
    // ========================================================================

    #[test]
    fn test_check_dequeue_eligibility_eligible() {
        let pending: Vec<String> = vec![];
        let result = check_dequeue_eligibility(0, 1, 3, None, &pending, 1000);
        assert_eq!(result, DequeueEligibility::Eligible);
    }

    #[test]
    fn test_check_dequeue_eligibility_expired() {
        let pending: Vec<String> = vec![];
        let result = check_dequeue_eligibility(500, 1, 3, None, &pending, 1000);
        assert_eq!(result, DequeueEligibility::Expired);
    }

    #[test]
    fn test_check_dequeue_eligibility_max_attempts() {
        let pending: Vec<String> = vec![];
        let result = check_dequeue_eligibility(0, 3, 3, None, &pending, 1000);
        assert_eq!(result, DequeueEligibility::MaxAttemptsExceeded);
    }

    #[test]
    fn test_check_dequeue_eligibility_group_pending() {
        let pending = vec!["group-a".to_string()];
        let result = check_dequeue_eligibility(0, 1, 3, Some("group-a"), &pending, 1000);
        assert_eq!(result, DequeueEligibility::GroupPending);
    }

    #[test]
    fn test_check_dequeue_eligibility_group_not_pending() {
        let pending = vec!["group-b".to_string()];
        let result = check_dequeue_eligibility(0, 1, 3, Some("group-a"), &pending, 1000);
        assert_eq!(result, DequeueEligibility::Eligible);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_item_expiration_never_less_than_now() {
        check!().with_type::<(u64, u64, u64, u64)>().for_each(|(ttl, default, max, now)| {
            let expires = compute_item_expiration(*ttl, *default, *max, *now);
            if expires > 0 {
                assert!(expires >= *now, "Expiration must be >= now");
            }
        });
    }

    #[test]
    fn prop_dlq_decision_consistent() {
        check!().with_type::<(u32, u32, bool)>().for_each(|(attempts, max, explicit)| {
            let decision = should_move_to_dlq(*attempts, *max, *explicit);
            if decision.should_move {
                assert!(decision.reason.is_some());
            } else {
                assert!(decision.reason.is_none());
            }
        });
    }

    #[test]
    fn prop_parse_receipt_preserves_id() {
        check!().with_type::<u64>().for_each(|id| {
            let handle = format!("{}:123:456", id);
            assert_eq!(parse_receipt_handle(&handle), Some(*id));
        });
    }
}
