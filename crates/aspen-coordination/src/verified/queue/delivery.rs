//! Delivery attempt tracking and requeue priority functions.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_dequeue_spec.rs` for proofs.
//!
//! # Tiger Style
//!
//! Uses saturating arithmetic for all calculations.

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

/// Increment delivery count for dequeue.
#[inline]
pub fn increment_delivery_count_for_dequeue(current_count: u32) -> u32 {
    current_count.saturating_add(1)
}

/// Check if delivery count can be incremented.
#[inline]
pub fn can_increment_delivery_count(delivery_count: u32) -> bool {
    delivery_count < u32::MAX
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
