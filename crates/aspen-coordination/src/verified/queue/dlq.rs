//! Dead letter queue (DLQ) decision functions.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_ack_spec.rs` for proofs.

use crate::queue::DLQReason;

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

/// Check if item should be moved to DLQ (exec version, Verus-aligned).
#[inline]
pub fn should_move_to_dlq_exec(delivery_count: u32, max_delivery_attempts: u32) -> bool {
    max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts
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
