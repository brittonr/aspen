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
        Self { should_move: false, reason: None }
    }

    /// Create a decision to move to DLQ with the given reason.
    #[inline]
    pub fn move_to_dlq(reason: DLQReason) -> Self {
        Self { should_move: true, reason: Some(reason) }
    }
}

/// Determine whether an item should be moved to the dead letter queue.
///
/// An item is moved to DLQ if:
/// - It is explicitly rejected by the consumer, OR
/// - It has exceeded the maximum delivery attempts
///
/// # Arguments
///
/// * `delivery_attempts` - Number of times the item has been delivered
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
/// let decision = should_move_to_dlq(1, 3, true);
/// assert!(decision.should_move);
/// assert_eq!(decision.reason, Some(DLQReason::ExplicitlyRejected));
///
/// // Max attempts exceeded
/// let decision = should_move_to_dlq(3, 3, false);
/// assert!(decision.should_move);
///
/// // Still has attempts remaining
/// let decision = should_move_to_dlq(2, 3, false);
/// assert!(!decision.should_move);
/// ```
#[inline]
pub fn should_move_to_dlq(delivery_attempts: u32, max_delivery_attempts: u32, explicit_reject: bool) -> DLQDecision {
    if explicit_reject {
        return DLQDecision::move_to_dlq(DLQReason::ExplicitlyRejected);
    }

    if max_delivery_attempts > 0 && delivery_attempts >= max_delivery_attempts {
        return DLQDecision::move_to_dlq(DLQReason::MaxDeliveryAttemptsExceeded);
    }

    DLQDecision::keep()
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let decision = should_move_to_dlq(1, 3, true);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(DLQReason::ExplicitlyRejected));
    }

    #[test]
    fn test_dlq_max_attempts_exceeded() {
        let decision = should_move_to_dlq(3, 3, false);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(DLQReason::MaxDeliveryAttemptsExceeded));
    }

    #[test]
    fn test_dlq_attempts_remaining() {
        let decision = should_move_to_dlq(2, 3, false);
        assert!(!decision.should_move);
        assert_eq!(decision.reason, None);
    }

    #[test]
    fn test_dlq_no_limit() {
        // max_delivery_attempts = 0 means no limit
        let decision = should_move_to_dlq(100, 0, false);
        assert!(!decision.should_move);
    }

    #[test]
    fn test_dlq_explicit_reject_overrides_attempts() {
        // Even with attempts remaining, explicit reject moves to DLQ
        let decision = should_move_to_dlq(1, 10, true);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(DLQReason::ExplicitlyRejected));
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
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_item_expiration_never_less_than_now() {
        check!()
            .with_type::<(u64, u64, u64, u64)>()
            .for_each(|(ttl, default, max, now)| {
                let expires = compute_item_expiration(*ttl, *default, *max, *now);
                if expires > 0 {
                    assert!(expires >= *now, "Expiration must be >= now");
                }
            });
    }

    #[test]
    fn prop_dlq_decision_consistent() {
        check!()
            .with_type::<(u32, u32, bool)>()
            .for_each(|(attempts, max, explicit)| {
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
