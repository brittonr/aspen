//! Ack/Nack/Redrive validation and deduplication functions.
//!
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/queue_ack_spec.rs` for proofs.

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
pub fn is_duplicate_message(has_dedup_entry: bool, dedup_expires_at_ms: u64, current_time_ms: u64) -> bool {
    has_dedup_entry && dedup_expires_at_ms > current_time_ms
}

/// Check if payload size is valid.
#[inline]
pub fn is_payload_size_valid(payload_len: u64) -> bool {
    payload_len <= 1_000_000
}

/// Check if a queue is empty.
#[inline]
pub fn is_queue_empty(pending_count: u32, inflight_count: u32) -> bool {
    pending_count == 0 && inflight_count == 0
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
