//! Distributed token bucket rate limiter.
//!
//! Provides cluster-wide rate limiting using token bucket algorithm.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_constants::coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
use aspen_constants::coordination::CAS_RETRY_MAX_BACKOFF_MS;
use aspen_constants::coordination::MAX_CAS_RETRIES;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use crate::error::CoordinationError;
use crate::error::RateLimitError;
use crate::types::BucketState;
use crate::types::now_unix_ms;

/// Internal result type for CAS update attempts.
enum TryAcquireResult {
    /// Successfully acquired tokens.
    Success(u64),
    /// Tokens exhausted.
    Exhausted(RateLimitError),
    /// Storage error.
    StorageError(RateLimitError),
    /// CAS conflict, retry.
    Retry,
}

/// Configuration for distributed rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum tokens (burst capacity).
    pub capacity_tokens: u64,
    /// Tokens added per second.
    pub refill_rate: f64,
    /// Initial tokens (defaults to capacity_tokens).
    pub initial_tokens: Option<u64>,
}

impl RateLimiterConfig {
    /// Create a config with the given rate per second and burst capacity.
    pub fn new(rate_per_second: f64, burst: u64) -> Self {
        // Tiger Style: rate must be positive
        debug_assert!(rate_per_second > 0.0, "rate_per_second must be positive, got {}", rate_per_second);
        // Tiger Style: burst capacity must be positive
        debug_assert!(burst > 0, "burst capacity must be positive, got {}", burst);

        Self {
            capacity_tokens: burst,
            refill_rate: rate_per_second,
            initial_tokens: None,
        }
    }

    /// Create a config with rate specified per minute.
    pub fn per_minute(rate_per_minute: u32, burst: u64) -> Self {
        // Tiger Style: rate must be positive
        debug_assert!(rate_per_minute > 0, "rate_per_minute must be positive, got {}", rate_per_minute);
        // Tiger Style: burst capacity must be positive
        debug_assert!(burst > 0, "burst capacity must be positive, got {}", burst);

        Self {
            capacity_tokens: burst,
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
        // Tiger Style: token count must be positive
        debug_assert!(n > 0, "token count must be positive, got {}", n);
        // Tiger Style: token count cannot exceed capacity
        debug_assert!(
            n <= self.config.capacity_tokens,
            "token count {} cannot exceed capacity {}",
            n,
            self.config.capacity_tokens
        );

        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let now_ms = now_unix_ms();
            let current = match self.read_state().await {
                Ok(state) => state,
                Err(e) => {
                    return Err(RateLimitError::StorageUnavailable { reason: e.to_string() });
                }
            };

            let available = self.try_acquire_n_compute_available(&current, now_ms);

            // Check if we can acquire
            if (n as f64) > available {
                return Err(self.try_acquire_n_exhausted_error(n, available));
            }

            // Prepare new state with current config values
            let new_state = BucketState {
                tokens: available - (n as f64),
                last_update_ms: now_ms,
                capacity_tokens: self.config.capacity_tokens,
                refill_rate: self.config.refill_rate,
            };

            // Atomic update
            match self.try_acquire_n_cas_update(n, &current, &new_state, &mut attempt, &mut backoff_ms).await {
                TryAcquireResult::Success(remaining) => return Ok(remaining),
                TryAcquireResult::Exhausted(err) => return Err(err),
                TryAcquireResult::StorageError(err) => return Err(err),
                TryAcquireResult::Retry => continue,
            }
        }
    }

    /// Compute available tokens after replenishment.
    fn try_acquire_n_compute_available(&self, current: &BucketState, now_ms: u64) -> f64 {
        let elapsed_ms = now_ms.saturating_sub(current.last_update_ms);
        let elapsed_secs = elapsed_ms as f64 / 1000.0;
        let replenished = elapsed_secs * self.config.refill_rate;
        (current.tokens + replenished).min(self.config.capacity_tokens as f64)
    }

    /// Build error for tokens exhausted case.
    fn try_acquire_n_exhausted_error(&self, requested: u64, available: f64) -> RateLimitError {
        let deficit = (requested as f64) - available;
        let wait_secs = deficit / self.config.refill_rate;
        RateLimitError::TokensExhausted {
            requested,
            available: available as u64,
            retry_after_ms: (wait_secs * 1000.0).ceil() as u64,
        }
    }

    /// Attempt CAS update and handle result.
    async fn try_acquire_n_cas_update(
        &self,
        n: u64,
        current: &BucketState,
        new_state: &BucketState,
        attempt: &mut u32,
        backoff_ms: &mut u64,
    ) -> TryAcquireResult {
        match self.cas_state(current, new_state).await {
            Ok(_) => {
                debug!(
                    key = %self.key,
                    tokens_consumed = n,
                    remaining = new_state.tokens as u64,
                    "rate limit tokens acquired"
                );
                TryAcquireResult::Success(new_state.tokens as u64)
            }
            Err(CoordinationError::CasConflict) => {
                *attempt += 1;
                if *attempt >= MAX_CAS_RETRIES {
                    return TryAcquireResult::Exhausted(RateLimitError::TokensExhausted {
                        requested: n,
                        available: 0,
                        retry_after_ms: 1000,
                    });
                }
                // Sleep happens in caller after returning Retry
                let sleep_ms = *backoff_ms;
                *backoff_ms = (*backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                TryAcquireResult::Retry
            }
            Err(CoordinationError::Storage { source }) => self.try_acquire_n_handle_storage_error(n, new_state, source),
            Err(e) => TryAcquireResult::StorageError(RateLimitError::StorageUnavailable { reason: e.to_string() }),
        }
    }

    /// Handle storage errors, with special case for follower nodes.
    fn try_acquire_n_handle_storage_error(
        &self,
        n: u64,
        new_state: &BucketState,
        source: KeyValueStoreError,
    ) -> TryAcquireResult {
        // Check if this is a "not leader" error from Raft write
        let is_not_leader =
            matches!(&source, KeyValueStoreError::NotLeader { .. }) || source.to_string().contains("forward");

        if is_not_leader {
            // Follower node - allow request (fail-open), leader manages state
            debug!(
                key = %self.key,
                tokens_consumed = n,
                remaining = new_state.tokens as u64,
                "rate limit check passed (follower mode - state update deferred to leader)"
            );
            TryAcquireResult::Success(new_state.tokens as u64)
        } else {
            TryAcquireResult::StorageError(RateLimitError::StorageUnavailable {
                reason: source.to_string(),
            })
        }
    }

    /// Block until a token is available (with timeout).
    pub async fn acquire(&self, timeout: Duration) -> Result<u64, RateLimitError> {
        self.acquire_n(1, timeout).await
    }

    /// Block until N tokens are available (with timeout).
    pub async fn acquire_n(&self, n: u64, timeout: Duration) -> Result<u64, RateLimitError> {
        // Tiger Style: token count must be positive
        debug_assert!(n > 0, "token count must be positive, got {}", n);
        // Tiger Style: timeout must not be zero
        debug_assert!(!timeout.is_zero(), "timeout must not be zero");

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
        let new_state = BucketState::new(self.config.capacity_tokens, self.config.refill_rate);
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
    ///
    /// Uses stale consistency because rate limiting is "soft" state where slight
    /// staleness is acceptable. This allows follower nodes to check rate limits
    /// without contacting the leader. The CAS update still requires consensus
    /// for authoritative state changes.
    async fn read_state(&self) -> Result<BucketState, CoordinationError> {
        match self.store.read(ReadRequest::stale(self.key.clone())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                let state: BucketState =
                    serde_json::from_str(&value).map_err(|e| CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: format!("invalid bucket state JSON: {}", e),
                    })?;
                Ok(state)
            }
            Err(KeyValueStoreError::NotFound { .. }) => {
                // Initialize with config
                let initial_tokens = self.config.initial_tokens.unwrap_or(self.config.capacity_tokens);
                Ok(BucketState {
                    tokens: initial_tokens as f64,
                    last_update_ms: now_unix_ms(),
                    capacity_tokens: self.config.capacity_tokens,
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
        // Use stale read - we're about to CAS anyway, staleness is acceptable
        let expected = match self.store.read(ReadRequest::stale(self.key.clone())).await {
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
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

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
