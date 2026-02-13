//! Helper functions for Pijul handlers.

/// Get current Unix timestamp in milliseconds (Tiger Style: safe fallback to 0).
#[inline]
pub(crate) fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
