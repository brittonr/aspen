//! Dequeue eligibility and message group ordering functions.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_dequeue_spec.rs` for proofs.
//!
//! # Tiger Style
//!
//! Uses saturating arithmetic for all calculations.

use super::delivery::has_exceeded_max_delivery_attempts;
use super::expiration::is_queue_item_expired;

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

/// Information about pending message groups with timestamps.
#[derive(Debug, Clone)]
pub struct PendingGroupInfo {
    /// Group ID.
    pub group_id: String,
    /// When this group started processing (Unix ms).
    pub started_at_ms: u64,
}

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
    assert!(max_batch_size > 0, "QUEUE: max_batch_size must be > 0");
    let result = requested.min(max_batch_size);
    assert!(result <= max_batch_size, "QUEUE: batch size must be <= max: {result} > {max_batch_size}");
    result
}

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

/// Check if batch size is valid.
#[inline]
pub fn is_batch_size_valid(batch_size: u32) -> bool {
    batch_size <= 100
}
