//! Expiry checking functions for queue items and deduplication entries.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_state_spec.rs` for proofs.
//!
//! # Tiger Style
//!
//! Uses saturating arithmetic for all calculations.

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

/// Check if TTL computation would overflow.
#[inline]
pub fn can_compute_ttl(current_time_ms: u64, ttl_ms: u64) -> bool {
    ttl_ms == 0 || current_time_ms <= u64::MAX - ttl_ms
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
