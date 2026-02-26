//! Shared types for coordination primitives.

use serde::Deserialize;
use serde::Serialize;

/// Lock entry stored in the KV store.
///
/// Serialized as JSON for human readability and debugging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockEntry {
    /// Unique identifier of the lock holder.
    pub holder_id: String,
    /// Monotonically increasing token for fencing.
    pub fencing_token: u64,
    /// When the lock was acquired (Unix timestamp milliseconds).
    pub acquired_at_ms: u64,
    /// TTL in milliseconds.
    pub ttl_ms: u64,
    /// Deadline = acquired_at_ms + ttl_ms.
    pub deadline_ms: u64,
}

impl LockEntry {
    /// Create a new lock entry.
    pub fn new(holder_id: String, fencing_token: u64, ttl_ms: u64) -> Self {
        let acquired_at_ms = now_unix_ms();
        Self {
            holder_id,
            fencing_token,
            acquired_at_ms,
            ttl_ms,
            deadline_ms: acquired_at_ms + ttl_ms,
        }
    }

    /// Create a released lock entry (preserves fencing token for history).
    pub fn released(&self) -> Self {
        Self {
            holder_id: String::new(),
            fencing_token: self.fencing_token,
            acquired_at_ms: self.acquired_at_ms,
            ttl_ms: 0,
            deadline_ms: 0, // 0 means released/expired
        }
    }

    /// Check if this lock entry has expired.
    pub fn is_expired(&self) -> bool {
        crate::verified::is_lock_expired(self.deadline_ms, now_unix_ms())
    }

    /// Get remaining TTL in milliseconds (0 if expired).
    pub fn remaining_ttl_ms(&self) -> u64 {
        crate::verified::remaining_ttl_ms(self.deadline_ms, now_unix_ms())
    }
}

/// Fencing token returned on successful lock acquisition.
///
/// Include this token in all operations protected by the lock.
/// External services should validate that the token is not stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FencingToken(pub u64);

impl FencingToken {
    /// Create a new fencing token.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw token value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for FencingToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FencingToken({})", self.0)
    }
}

/// Rate limiter bucket state stored in KV store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketState {
    /// Current token count (fractional for precision).
    pub tokens: f64,
    /// Last update timestamp (Unix milliseconds).
    pub last_update_ms: u64,
    /// Maximum capacity (in tokens).
    pub capacity_tokens: u64,
    /// Refill rate (tokens per second).
    pub refill_rate: f64,
}

impl BucketState {
    /// Create a new bucket state with full capacity.
    pub fn new(capacity_tokens: u64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity_tokens as f64,
            last_update_ms: now_unix_ms(),
            capacity_tokens,
            refill_rate,
        }
    }

    /// Calculate current available tokens after replenishment.
    pub fn available_tokens(&self) -> f64 {
        crate::verified::calculate_replenished_tokens(
            self.tokens,
            self.last_update_ms,
            now_unix_ms(),
            self.refill_rate,
            self.capacity_tokens,
        )
    }
}

/// Get current Unix timestamp in milliseconds.
///
/// Re-exports from aspen-time for convenience. Backward compatible.
#[inline]
pub fn now_unix_ms() -> u64 {
    aspen_time::current_time_ms()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_entry_expiry() {
        let entry = LockEntry {
            holder_id: "test".to_string(),
            fencing_token: 1,
            acquired_at_ms: now_unix_ms() - 10_000, // 10 seconds ago
            ttl_ms: 5_000,                          // 5 second TTL
            deadline_ms: now_unix_ms() - 5_000,     // expired 5 seconds ago
        };
        assert!(entry.is_expired());
        assert_eq!(entry.remaining_ttl_ms(), 0);
    }

    #[test]
    fn test_lock_entry_not_expired() {
        let entry = LockEntry::new("test".to_string(), 1, 30_000);
        assert!(!entry.is_expired());
        assert!(entry.remaining_ttl_ms() > 29_000);
    }

    #[test]
    fn test_fencing_token_ordering() {
        let t1 = FencingToken::new(1);
        let t2 = FencingToken::new(2);
        assert!(t1 < t2);
    }

    #[test]
    fn test_bucket_state_replenishment() {
        let state = BucketState {
            tokens: 0.0,
            last_update_ms: now_unix_ms() - 1000, // 1 second ago
            capacity_tokens: 10,
            refill_rate: 5.0, // 5 tokens per second
        };
        let available = state.available_tokens();
        // Should have ~5 tokens after 1 second
        assert!((4.5..=5.5).contains(&available));
    }
}
