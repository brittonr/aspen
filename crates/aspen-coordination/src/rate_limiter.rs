//! Distributed token bucket rate limiter.
//!
//! Provides cluster-wide rate limiting using token bucket algorithm.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use tracing::debug;

use crate::error::CoordinationError;
use crate::error::RateLimitError;
use crate::types::BucketState;
use crate::types::now_unix_ms;
use aspen_core::CAS_RETRY_INITIAL_BACKOFF_MS;
use aspen_core::CAS_RETRY_MAX_BACKOFF_MS;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::MAX_CAS_RETRIES;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;

/// Configuration for distributed rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum tokens (burst capacity).
    pub capacity: u64,
    /// Tokens added per second.
    pub refill_rate: f64,
    /// Initial tokens (defaults to capacity).
    pub initial_tokens: Option<u64>,
}

impl RateLimiterConfig {
    /// Create a config with the given rate per second and burst capacity.
    pub fn new(rate_per_second: f64, burst: u64) -> Self {
        Self {
            capacity: burst,
            refill_rate: rate_per_second,
            initial_tokens: None,
        }
    }

    /// Create a config with rate specified per minute.
    pub fn per_minute(rate_per_minute: u32, burst: u64) -> Self {
        Self {
            capacity: burst,
            refill_rate: rate_per_minute as f64 / 60.0,
            initial_tokens: None,
        }
    }
}

/// Distributed token bucket rate limiter.
///
/// Uses cluster-wide state to enforce rate limits across all nodes.
/// The token bucket algorithm allows controlled bursting while
/// maintaining a steady-state rate limit.
pub struct DistributedRateLimiter<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    key: String,
    config: RateLimiterConfig,
}

impl<S: KeyValueStore + ?Sized> DistributedRateLimiter<S> {
    /// Create a new rate limiter.
    pub fn new(store: Arc<S>, key: impl Into<String>, config: RateLimiterConfig) -> Self {
        Self {
            store,
            key: key.into(),
            config,
        }
    }

    /// Try to consume one token.
    ///
    /// Returns `Ok(remaining)` if allowed, `Err(RateLimitError)` if rate limited.
    pub async fn try_acquire(&self) -> Result<u64, RateLimitError> {
        self.try_acquire_n(1).await
    }

    /// Try to consume N tokens.
    ///
    /// Returns `Ok(remaining)` if allowed, `Err(RateLimitError)` if rate limited.
    pub async fn try_acquire_n(&self, n: u64) -> Result<u64, RateLimitError> {
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let now_ms = now_unix_ms();
            let current = match self.read_state().await {
                Ok(state) => state,
                Err(e) => {
                    // Storage unavailable - return distinct error for caller to handle
                    return Err(RateLimitError::StorageUnavailable { reason: e.to_string() });
                }
            };

            // Calculate replenished tokens using CONFIG values (not stored values)
            // This ensures config changes take effect immediately
            let elapsed_ms = now_ms.saturating_sub(current.last_update_ms);
            let elapsed_secs = elapsed_ms as f64 / 1000.0;
            let replenished = elapsed_secs * self.config.refill_rate;
            let available = (current.tokens + replenished).min(self.config.capacity as f64);

            // Check if we can acquire
            if (n as f64) > available {
                let deficit = (n as f64) - available;
                let wait_secs = deficit / self.config.refill_rate;
                return Err(RateLimitError::TokensExhausted {
                    requested: n,
                    available: available as u64,
                    retry_after_ms: (wait_secs * 1000.0).ceil() as u64,
                });
            }

            // Prepare new state with current config values
            let new_state = BucketState {
                tokens: available - (n as f64),
                last_update_ms: now_ms,
                capacity: self.config.capacity,
                refill_rate: self.config.refill_rate,
            };

            // Atomic update
            match self.cas_state(&current, &new_state).await {
                Ok(_) => {
                    debug!(
                        key = %self.key,
                        tokens_consumed = n,
                        remaining = new_state.tokens as u64,
                        "rate limit tokens acquired"
                    );
                    return Ok(new_state.tokens as u64);
                }
                Err(CoordinationError::CasConflict) => {
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        // CAS contention is still rate limiting - use TokensExhausted
                        return Err(RateLimitError::TokensExhausted {
                            requested: n,
                            available: 0,
                            retry_after_ms: 1000,
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => {
                    // Storage error during CAS - return distinct error
                    return Err(RateLimitError::StorageUnavailable { reason: e.to_string() });
                }
            }
        }
    }

    /// Block until a token is available (with timeout).
    pub async fn acquire(&self, timeout: Duration) -> Result<u64, RateLimitError> {
        self.acquire_n(1, timeout).await
    }

    /// Block until N tokens are available (with timeout).
    pub async fn acquire_n(&self, n: u64, timeout: Duration) -> Result<u64, RateLimitError> {
        let deadline = Instant::now() + timeout;

        loop {
            match self.try_acquire_n(n).await {
                Ok(remaining) => return Ok(remaining),
                Err(ref e) => {
                    // For storage errors, immediately propagate (no retry would help)
                    // For token exhaustion, wait and retry
                    let retry_ms = match e.retry_after_ms() {
                        Some(ms) => ms,
                        None => return Err(e.clone()), // StorageUnavailable - no point retrying
                    };
                    let wait = Duration::from_millis(retry_ms.min(100));
                    if Instant::now() + wait > deadline {
                        return Err(e.clone());
                    }
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    /// Get current available tokens without consuming.
    pub async fn available(&self) -> Result<u64, CoordinationError> {
        let state = self.read_state().await?;
        Ok(state.available_tokens() as u64)
    }

    /// Reset the bucket to full capacity.
    pub async fn reset(&self) -> Result<(), CoordinationError> {
        let new_state = BucketState::new(self.config.capacity, self.config.refill_rate);
        let json = serde_json::to_string(&new_state)?;

        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        // Try to overwrite with CAS loop
        loop {
            let current = self.read_state().await.ok();
            let expected = match current {
                Some(ref s) => Some(serde_json::to_string(s)?),
                None => None,
            };

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: self.key.clone(),
                        expected,
                        new_value: json.clone(),
                    },
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        return Err(CoordinationError::MaxRetriesExceeded {
                            operation: "rate limiter reset".to_string(),
                            attempts: attempt,
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            }
        }
    }

    /// Read the current bucket state from storage.
    async fn read_state(&self) -> Result<BucketState, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                let state: BucketState =
                    serde_json::from_str(&value).map_err(|_| CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "invalid bucket state JSON".to_string(),
                    })?;
                Ok(state)
            }
            Err(KeyValueStoreError::NotFound { .. }) => {
                // Initialize with config
                let initial_tokens = self.config.initial_tokens.unwrap_or(self.config.capacity);
                Ok(BucketState {
                    tokens: initial_tokens as f64,
                    last_update_ms: now_unix_ms(),
                    capacity: self.config.capacity,
                    refill_rate: self.config.refill_rate,
                })
            }
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Compare-and-swap bucket state.
    async fn cas_state(&self, current: &BucketState, new: &BucketState) -> Result<(), CoordinationError> {
        let current_json = serde_json::to_string(current)?;
        let new_json = serde_json::to_string(new)?;

        // Handle initial state (key doesn't exist yet)
        let expected = match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(_) => Some(current_json),
            Err(KeyValueStoreError::NotFound { .. }) => None,
            Err(e) => return Err(CoordinationError::Storage { source: e }),
        };

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: self.key.clone(),
                    expected,
                    new_value: new_json,
                },
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Err(CoordinationError::CasConflict),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspen_core::inmemory::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(
            store,
            "test_limiter",
            RateLimiterConfig::new(10.0, 5), // 10/sec, burst 5
        );

        // Should allow burst
        for i in 0..5 {
            let result = limiter.try_acquire().await;
            assert!(result.is_ok(), "Acquire {} should succeed", i);
        }

        // 6th should fail
        let result = limiter.try_acquire().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_replenishment() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(
            store,
            "test_limiter",
            RateLimiterConfig::new(100.0, 1), // 100/sec, burst 1
        );

        // Exhaust
        limiter.try_acquire().await.unwrap();
        assert!(limiter.try_acquire().await.is_err());

        // Wait for replenishment (~10ms for 1 token at 100/sec)
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should be available again
        assert!(limiter.try_acquire().await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire_with_timeout() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(store, "test_limiter", RateLimiterConfig::new(100.0, 1));

        // Exhaust
        limiter.try_acquire().await.unwrap();

        // Acquire with timeout should wait and succeed
        let result = limiter.acquire(Duration::from_millis(50)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_timeout() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(
            store,
            "test_limiter",
            RateLimiterConfig::new(1.0, 1), // 1/sec, burst 1
        );

        // Exhaust
        limiter.try_acquire().await.unwrap();

        // Acquire with very short timeout should fail
        let result = limiter.acquire(Duration::from_millis(10)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(store, "test_limiter", RateLimiterConfig::new(10.0, 5));

        // Exhaust
        for _ in 0..5 {
            limiter.try_acquire().await.unwrap();
        }
        assert!(limiter.try_acquire().await.is_err());

        // Reset
        limiter.reset().await.unwrap();

        // Should be full again
        for _ in 0..5 {
            limiter.try_acquire().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_available() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(store, "test_limiter", RateLimiterConfig::new(10.0, 10));

        assert_eq!(limiter.available().await.unwrap(), 10);

        limiter.try_acquire_n(3).await.unwrap();

        assert_eq!(limiter.available().await.unwrap(), 7);
    }

    #[tokio::test]
    async fn test_rate_limiter_tokens_exhausted_error() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(
            store,
            "test_limiter",
            RateLimiterConfig::new(10.0, 5), // 10/sec, burst 5
        );

        // Exhaust all tokens
        for _ in 0..5 {
            limiter.try_acquire().await.unwrap();
        }

        // Should get TokensExhausted error
        let result = limiter.try_acquire().await;
        match result {
            Err(RateLimitError::TokensExhausted {
                requested,
                available,
                retry_after_ms,
            }) => {
                assert_eq!(requested, 1);
                assert_eq!(available, 0);
                assert!(retry_after_ms > 0);
            }
            _ => panic!("Expected TokensExhausted error"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_error_helper_methods() {
        // Test TokensExhausted retry_after_ms helper
        let exhausted_err = RateLimitError::TokensExhausted {
            requested: 1,
            available: 0,
            retry_after_ms: 100,
        };
        assert_eq!(exhausted_err.retry_after_ms(), Some(100));

        // Test StorageUnavailable has no retry_after_ms
        let storage_err = RateLimitError::StorageUnavailable {
            reason: "test error".to_string(),
        };
        assert_eq!(storage_err.retry_after_ms(), None);
    }
}
