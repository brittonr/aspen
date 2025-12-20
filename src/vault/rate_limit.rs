//! Per-vault rate limiting using token buckets.
//!
//! Implements global per-vault rate limiting with an LRU cache of token buckets.
//!
//! # Design
//!
//! - Each vault has its own token bucket
//! - Separate limits for reads and writes
//! - LRU eviction when cache exceeds MAX_RATE_LIMITER_VAULTS
//! - Thread-safe via parking_lot mutex
//!
//! # Tiger Style
//!
//! - Bounded LRU cache (MAX_RATE_LIMITER_VAULTS = 1000)
//! - Fixed rate limits (configurable per deployment)

use std::time::Instant;

use lru::LruCache;
use parking_lot::Mutex;

use crate::api::vault::{
    MAX_RATE_LIMITER_VAULTS, MAX_VAULT_READS_PER_SECOND, MAX_VAULT_WRITES_PER_SECOND, VaultError,
    parse_vault_key,
};

/// Token bucket for rate limiting.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Available tokens.
    tokens: f64,
    /// Last time tokens were refilled.
    last_refill: Instant,
    /// Maximum tokens (bucket capacity).
    max_tokens: f64,
    /// Tokens added per second.
    refill_rate: f64,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            last_refill: Instant::now(),
            max_tokens,
            refill_rate,
        }
    }

    /// Try to consume one token.
    ///
    /// Returns Ok(()) if token was available, or Err with retry-after hint.
    fn try_consume(&mut self) -> Result<(), u64> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate time until one token is available
            let tokens_needed = 1.0 - self.tokens;
            let seconds_until_available = tokens_needed / self.refill_rate;
            let retry_after_ms = (seconds_until_available * 1000.0).ceil() as u64;
            Err(retry_after_ms.max(1))
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

/// Operation type for rate limiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Read,
    Write,
}

/// Per-vault rate limiter using LRU cache of token buckets.
pub struct VaultRateLimiter {
    /// Write rate limiters per vault.
    write_buckets: Mutex<LruCache<String, TokenBucket>>,
    /// Read rate limiters per vault.
    read_buckets: Mutex<LruCache<String, TokenBucket>>,
    /// Write rate limit (tokens per second).
    write_rate: f64,
    /// Read rate limit (tokens per second).
    read_rate: f64,
}

impl VaultRateLimiter {
    /// Create a new rate limiter with default limits.
    pub fn new() -> Self {
        Self::with_limits(
            MAX_VAULT_WRITES_PER_SECOND as f64,
            MAX_VAULT_READS_PER_SECOND as f64,
        )
    }

    /// Create a rate limiter with custom limits.
    pub fn with_limits(write_rate: f64, read_rate: f64) -> Self {
        Self {
            write_buckets: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(MAX_RATE_LIMITER_VAULTS).unwrap(),
            )),
            read_buckets: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(MAX_RATE_LIMITER_VAULTS).unwrap(),
            )),
            write_rate,
            read_rate,
        }
    }

    /// Check if an operation is allowed under rate limits.
    ///
    /// Returns Ok(()) if allowed, or Err(VaultError::RateLimited) if not.
    ///
    /// Non-vault keys bypass rate limiting.
    pub fn check(&self, key: &str, operation: Operation) -> Result<(), VaultError> {
        let Some((vault_name, _)) = parse_vault_key(key) else {
            // Non-vault keys are not rate limited
            return Ok(());
        };

        self.check_vault(&vault_name, operation)
    }

    /// Check rate limit for a specific vault.
    pub fn check_vault(&self, vault_name: &str, operation: Operation) -> Result<(), VaultError> {
        match operation {
            Operation::Write => {
                let mut buckets = self.write_buckets.lock();
                let bucket = buckets.get_or_insert_mut(vault_name.to_string(), || {
                    TokenBucket::new(self.write_rate, self.write_rate)
                });
                bucket
                    .try_consume()
                    .map_err(|retry_after_ms| VaultError::RateLimited {
                        vault: vault_name.to_string(),
                        retry_after_ms,
                    })
            }
            Operation::Read => {
                let mut buckets = self.read_buckets.lock();
                let bucket = buckets.get_or_insert_mut(vault_name.to_string(), || {
                    TokenBucket::new(self.read_rate, self.read_rate)
                });
                bucket
                    .try_consume()
                    .map_err(|retry_after_ms| VaultError::RateLimited {
                        vault: vault_name.to_string(),
                        retry_after_ms,
                    })
            }
        }
    }

    /// Get the number of vaults currently tracked.
    pub fn tracked_vault_count(&self) -> (usize, usize) {
        (
            self.write_buckets.lock().len(),
            self.read_buckets.lock().len(),
        )
    }
}

impl Default for VaultRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_allows_initial_requests() {
        let mut bucket = TokenBucket::new(10.0, 10.0);

        // Should allow first 10 requests
        for _ in 0..10 {
            assert!(bucket.try_consume().is_ok());
        }

        // 11th should fail
        assert!(bucket.try_consume().is_err());
    }

    #[test]
    fn test_token_bucket_returns_retry_after() {
        let mut bucket = TokenBucket::new(1.0, 10.0);

        // Use the single token
        assert!(bucket.try_consume().is_ok());

        // Next request should return retry-after
        let result = bucket.try_consume();
        assert!(result.is_err());
        let retry_after = result.unwrap_err();
        // At 10 tokens/sec, should be ~100ms for 1 token
        assert!(retry_after > 0 && retry_after <= 200);
    }

    #[test]
    fn test_rate_limiter_allows_non_vault_keys() {
        let limiter = VaultRateLimiter::with_limits(1.0, 1.0);

        // Non-vault keys bypass rate limiting
        for _ in 0..100 {
            assert!(limiter.check("regular_key", Operation::Write).is_ok());
            assert!(limiter.check("any:other:format", Operation::Read).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_limits_vault_writes() {
        let limiter = VaultRateLimiter::with_limits(5.0, 100.0);

        // First 5 writes should succeed
        for _ in 0..5 {
            assert!(limiter.check("vault:myapp:key", Operation::Write).is_ok());
        }

        // 6th should be rate limited
        let result = limiter.check("vault:myapp:key", Operation::Write);
        assert!(matches!(result, Err(VaultError::RateLimited { .. })));
    }

    #[test]
    fn test_rate_limiter_separate_vault_buckets() {
        let limiter = VaultRateLimiter::with_limits(2.0, 100.0);

        // Use up vault1's tokens
        assert!(limiter.check("vault:vault1:key", Operation::Write).is_ok());
        assert!(limiter.check("vault:vault1:key", Operation::Write).is_ok());
        assert!(limiter.check("vault:vault1:key", Operation::Write).is_err());

        // vault2 should still have tokens
        assert!(limiter.check("vault:vault2:key", Operation::Write).is_ok());
        assert!(limiter.check("vault:vault2:key", Operation::Write).is_ok());
    }

    #[test]
    fn test_rate_limiter_tracks_vault_count() {
        let limiter = VaultRateLimiter::with_limits(100.0, 100.0);

        limiter.check("vault:app1:key", Operation::Write).unwrap();
        limiter.check("vault:app2:key", Operation::Read).unwrap();
        limiter.check("vault:app3:key", Operation::Write).unwrap();

        let (write_count, read_count) = limiter.tracked_vault_count();
        assert_eq!(write_count, 2); // app1 and app3
        assert_eq!(read_count, 1); // app2
    }
}
