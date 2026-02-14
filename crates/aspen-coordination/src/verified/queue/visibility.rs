//! Visibility timeout computation functions for queue operations.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_dequeue_spec.rs` for proofs.
//!
//! # Tiger Style
//!
//! Uses saturating arithmetic for all calculations.

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
