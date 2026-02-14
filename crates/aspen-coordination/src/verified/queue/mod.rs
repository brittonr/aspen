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
//!
//! # Sub-modules
//!
//! - [`key_generation`]: Queue key prefix and key/prefix generation functions
//! - [`visibility`]: Visibility timeout computation and extension
//! - [`delivery`]: Delivery attempt tracking, requeue priority
//! - [`expiration`]: TTL and item expiry checks
//! - [`dlq`]: Dead letter queue decision logic
//! - [`dequeue`]: Dequeue eligibility, message group ordering, batch sizing
//! - [`validation`]: Ack/nack/redrive validation, deduplication
//! - [`enqueue`]: Item construction, receipt handles, ID allocation

pub mod delivery;
pub mod dequeue;
pub mod dlq;
pub mod enqueue;
pub mod expiration;
pub mod key_generation;
pub mod validation;
pub mod visibility;

// ============================================================================
// Re-exports: Key Generation
// ============================================================================

// ============================================================================
// Re-exports: Delivery
// ============================================================================
pub use delivery::RequeuePriority;
pub use delivery::can_increment_delivery_count;
pub use delivery::compute_requeue_delivery_attempts;
pub use delivery::compute_requeue_priority;
pub use delivery::decrement_delivery_count_for_release;
pub use delivery::has_exceeded_max_delivery_attempts;
pub use delivery::increment_delivery_count;
pub use delivery::increment_delivery_count_for_dequeue;
// ============================================================================
// Re-exports: Dequeue
// ============================================================================
pub use dequeue::DequeueEligibility;
pub use dequeue::PendingGroupInfo;
pub use dequeue::are_dequeue_params_valid;
pub use dequeue::can_dequeue_from_group;
pub use dequeue::check_dequeue_eligibility;
pub use dequeue::compute_dequeue_batch_size;
pub use dequeue::is_batch_size_valid;
pub use dequeue::should_skip_for_message_group;
// ============================================================================
// Re-exports: DLQ
// ============================================================================
pub use dlq::DLQDecision;
pub use dlq::should_move_to_dlq;
pub use dlq::should_move_to_dlq_exec;
pub use dlq::should_move_to_dlq_with_reason;
pub use dlq::should_nack_to_dlq;
// ============================================================================
// Re-exports: Enqueue
// ============================================================================
pub use enqueue::allocate_item_id;
pub use enqueue::allocate_next_id;
pub use enqueue::can_allocate_id;
pub use enqueue::create_queue_item_from_pending;
pub use enqueue::generate_receipt_handle;
pub use enqueue::parse_receipt_handle;
// ============================================================================
// Re-exports: Expiration
// ============================================================================
pub use expiration::can_compute_ttl;
pub use expiration::compute_item_expiration;
pub use expiration::is_dedup_entry_expired;
pub use expiration::is_dedup_expired;
pub use expiration::is_item_expired;
pub use expiration::is_queue_item_expired;
pub use key_generation::QUEUE_PREFIX;
pub use key_generation::dedup_key;
pub use key_generation::dedup_prefix;
pub use key_generation::dlq_key;
pub use key_generation::dlq_prefix;
pub use key_generation::item_key;
pub use key_generation::items_prefix;
pub use key_generation::pending_key;
pub use key_generation::pending_prefix;
pub use key_generation::queue_metadata_key;
pub use key_generation::sequence_key;
// ============================================================================
// Re-exports: Validation
// ============================================================================
pub use validation::calculate_dedup_expiration;
pub use validation::can_compute_dedup_ttl;
pub use validation::compute_dedup_expiration;
pub use validation::is_ack_valid;
pub use validation::is_acquire_valid;
pub use validation::is_duplicate_message;
pub use validation::is_nack_valid;
pub use validation::is_payload_size_valid;
pub use validation::is_queue_empty;
pub use validation::is_redrive_valid;
// ============================================================================
// Re-exports: Visibility
// ============================================================================
pub use visibility::calculate_extended_deadline;
pub use visibility::calculate_visibility_deadline;
pub use visibility::can_extend_visibility;
pub use visibility::compute_effective_visibility_timeout;
pub use visibility::compute_extended_deadline;
pub use visibility::compute_visibility_deadline;
pub use visibility::is_extend_visibility_valid;
pub use visibility::is_visibility_expired;
pub use visibility::is_visibility_expired_exec;
pub use visibility::is_visibility_timeout_expired;
pub use visibility::time_until_visibility_expires;

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
        assert_eq!(decision.reason, Some(crate::queue::DLQReason::ExplicitlyRejected));
    }

    #[test]
    fn test_dlq_max_attempts_exceeded() {
        let decision = should_move_to_dlq_with_reason(3, 3, false);
        assert!(decision.should_move);
        assert_eq!(decision.reason, Some(crate::queue::DLQReason::MaxDeliveryAttemptsExceeded));
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
        assert_eq!(decision.reason, Some(crate::queue::DLQReason::ExplicitlyRejected));
    }

    #[test]
    fn test_dlq_verus_aligned() {
        // Test the Verus-aligned version (returns bool, no explicit_reject param)
        assert!(should_move_to_dlq(3, 3)); // At limit
        assert!(should_move_to_dlq(4, 3)); // Over limit
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
        let pending = crate::queue::PendingItem {
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
        let pending = crate::queue::PendingItem {
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
        let pending = crate::queue::PendingItem {
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
