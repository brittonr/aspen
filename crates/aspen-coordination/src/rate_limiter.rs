//! Distributed token bucket rate limiter.
//!
//! Provides cluster-wide rate limiting using token bucket algorithm.

use std::sync::Arc;
use std::time::Duration;

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
use crate::runtime_clock;
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
    /// CAS conflict, retry after backoff.
    RetryAfterMs(u64),
}

struct RetryState {
    attempt: u32,
    backoff_ms: u64,
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
        assert!(rate_per_second > 0.0, "rate_per_second must be positive, got {}", rate_per_second);
        assert!(burst > 0, "burst capacity must be positive, got {}", burst);

        Self {
            capacity_tokens: burst,
            refill_rate: rate_per_second,
            initial_tokens: None,
        }
    }

    /// Create a config with rate specified per minute.
    pub fn per_minute(rate_per_minute: u32, burst: u64) -> Self {
        assert!(rate_per_minute > 0, "rate_per_minute must be positive, got {}", rate_per_minute);
        assert!(burst > 0, "burst capacity must be positive, got {}", burst);

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
        Self::assert_valid_config(&config);

        let key = key.into();
        assert!(!key.is_empty(), "rate limiter key must not be empty");

        Self { store, key, config }
    }

    fn assert_valid_config(config: &RateLimiterConfig) {
        assert!(config.capacity_tokens > 0, "capacity_tokens must be positive, got {}", config.capacity_tokens);
        assert!(config.refill_rate > 0.0, "refill_rate must be positive, got {}", config.refill_rate);

        if let Some(initial_tokens) = config.initial_tokens {
            assert!(
                initial_tokens <= config.capacity_tokens,
                "initial_tokens {} cannot exceed capacity_tokens {}",
                initial_tokens,
                config.capacity_tokens
            );
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
        assert!(n > 0, "token count must be positive, got {}", n);
        assert!(
            n <= self.config.capacity_tokens,
            "token count {} cannot exceed capacity {}",
            n,
            self.config.capacity_tokens
        );

        let mut retry_state = RetryState {
            attempt: 0,
            backoff_ms: CAS_RETRY_INITIAL_BACKOFF_MS,
        };

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
            match self
                .try_acquire_n_cas_update(n, &current, &new_state, &mut retry_state)
                .await
            {
                TryAcquireResult::Success(remaining) => return Ok(remaining),
                TryAcquireResult::Exhausted(err) => return Err(err),
                TryAcquireResult::StorageError(err) => return Err(err),
                TryAcquireResult::RetryAfterMs(sleep_ms) => {
                    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                    continue;
                }
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
        assert!(self.config.refill_rate > 0.0, "refill_rate must be positive");

        let deficit = (requested as f64) - available;
        let wait_secs = deficit / self.config.refill_rate;
        let retry_after_ms = self.try_acquire_n_retry_after_ms(wait_secs);

        RateLimitError::TokensExhausted {
            requested,
            available: available as u64,
            retry_after_ms,
        }
    }

    fn try_acquire_n_retry_after_ms(&self, wait_secs: f64) -> u64 {
        assert!(wait_secs >= 0.0, "wait_secs must not be negative, got {}", wait_secs);

        let retry_after_ms = wait_secs * 1000.0;
        if !retry_after_ms.is_finite() {
            return u64::MAX;
        }
        if retry_after_ms >= u64::MAX as f64 {
            return u64::MAX;
        }
        retry_after_ms.ceil() as u64
    }

    /// Attempt CAS update and handle result.
    async fn try_acquire_n_cas_update(
        &self,
        n: u64,
        current: &BucketState,
        new_state: &BucketState,
        retry_state: &mut RetryState,
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
                retry_state.attempt += 1;
                if retry_state.attempt >= MAX_CAS_RETRIES {
                    return TryAcquireResult::Exhausted(RateLimitError::TokensExhausted {
                        requested: n,
                        available: 0,
                        retry_after_ms: 1000,
                    });
                }
                let sleep_ms = retry_state.backoff_ms;
                retry_state.backoff_ms = (retry_state.backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                TryAcquireResult::RetryAfterMs(sleep_ms)
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
        let is_leader_redirect_error =
            matches!(&source, KeyValueStoreError::NotLeader { .. }) || source.to_string().contains("forward");

        if is_leader_redirect_error {
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
        assert!(n > 0, "token count must be positive, got {}", n);
        assert!(!timeout.is_zero(), "timeout must not be zero");

        let deadline = runtime_clock::deadline_after(timeout);

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
                    if runtime_clock::wait_would_exceed_deadline(deadline, wait) {
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
        let new_json = serde_json::to_string(&new_state)?;
        let mut expected = self.read_state_json_for_reset().await?;
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: self.key.clone(),
                        expected: expected.clone(),
                        new_value: new_json.clone(),
                    },
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
                    if actual.as_ref().is_some_and(|value| value == &new_json) {
                        return Ok(());
                    }
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        return Err(CoordinationError::MaxRetriesExceeded {
                            operation: "rate limiter reset".to_string(),
                            attempts: attempt,
                        });
                    }
                    expected = actual;
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            }
        }
    }

    async fn read_state_json_for_reset(&self) -> Result<Option<String>, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => Ok(result.kv.map(|kv| kv.value)),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(CoordinationError::Storage { source: e }),
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
    use std::sync::Mutex;

    use aspen_kv_types::DeleteRequest;
    use aspen_kv_types::DeleteResult;
    use aspen_kv_types::KeyValueStoreError;
    use aspen_kv_types::KeyValueWithRevision;
    use aspen_kv_types::ReadRequest;
    use aspen_kv_types::ReadResult;
    use aspen_kv_types::ScanRequest;
    use aspen_kv_types::ScanResult;
    use aspen_kv_types::WriteCommand;
    use aspen_kv_types::WriteRequest;
    use aspen_kv_types::WriteResult;
    use aspen_testing::DeterministicKeyValueStore;
    use async_trait::async_trait;

    use super::*;

    struct ConflictThenSuccessStore {
        state: Mutex<ConflictThenSuccessState>,
    }

    struct ConflictThenSuccessState {
        bucket_json: String,
        remaining_conflicts: u32,
        write_calls: u32,
    }

    impl ConflictThenSuccessStore {
        fn new(bucket_state: BucketState, remaining_conflicts: u32) -> Self {
            let bucket_json = serde_json::to_string(&bucket_state).expect("serialize bucket state");
            Self {
                state: Mutex::new(ConflictThenSuccessState {
                    bucket_json,
                    remaining_conflicts,
                    write_calls: 0,
                }),
            }
        }

        fn write_calls(&self) -> u32 {
            self.state.lock().expect("lock conflict store").write_calls
        }
    }

    #[async_trait]
    impl KeyValueStore for ConflictThenSuccessStore {
        async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
            let mut state = self.state.lock().expect("lock conflict store");
            state.write_calls = state.write_calls.saturating_add(1);

            match request.command {
                WriteCommand::CompareAndSwap { key, new_value, .. } => {
                    if state.remaining_conflicts > 0 {
                        state.remaining_conflicts -= 1;
                        return Err(KeyValueStoreError::CompareAndSwapFailed {
                            key,
                            expected: None,
                            actual: Some(state.bucket_json.clone()),
                        });
                    }

                    state.bucket_json = new_value;
                    Ok(WriteResult::default())
                }
                other => Err(KeyValueStoreError::Failed {
                    reason: format!("unexpected write command in rate limiter test: {other:?}"),
                }),
            }
        }

        async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
            let state = self.state.lock().expect("lock conflict store");
            Ok(ReadResult {
                kv: Some(KeyValueWithRevision {
                    key: request.key,
                    value: state.bucket_json.clone(),
                    version: 1,
                    create_revision: 1,
                    mod_revision: 1,
                }),
            })
        }

        async fn delete(&self, _request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
            Err(KeyValueStoreError::Failed {
                reason: "delete must not be called in rate limiter test".to_string(),
            })
        }

        async fn scan(&self, _request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            Err(KeyValueStoreError::Failed {
                reason: "scan must not be called in rate limiter test".to_string(),
            })
        }
    }

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

    /// Regression test: reset must succeed even before the bucket key exists.
    ///
    /// Previously `reset()` called `read_state()`, which synthesized a full
    /// in-memory bucket for a missing key. The subsequent CAS used
    /// `expected=Some(full_bucket_json)` against an absent key, so it could only
    /// fail with CompareAndSwapFailed until MaxRetriesExceeded.
    #[tokio::test]
    async fn test_rate_limiter_reset_on_fresh_bucket() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(store, "fresh_limiter", RateLimiterConfig::new(10.0, 5));

        limiter.reset().await.unwrap();

        for _ in 0..5 {
            limiter.try_acquire().await.unwrap();
        }
        assert!(limiter.try_acquire().await.is_err());
    }

    #[test]
    #[should_panic(expected = "rate limiter key must not be empty")]
    fn test_rate_limiter_rejects_empty_key() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let _ = DistributedRateLimiter::new(store, "", RateLimiterConfig::new(10.0, 5));
    }

    #[test]
    #[should_panic(expected = "capacity_tokens must be positive")]
    fn test_rate_limiter_rejects_zero_capacity() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let config = RateLimiterConfig {
            capacity_tokens: 0,
            refill_rate: 1.0,
            initial_tokens: Some(0),
        };

        let _ = DistributedRateLimiter::new(store, "invalid_limiter", config);
    }

    #[test]
    #[should_panic(expected = "refill_rate must be positive")]
    fn test_rate_limiter_rejects_zero_refill_rate() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let config = RateLimiterConfig {
            capacity_tokens: 5,
            refill_rate: 0.0,
            initial_tokens: Some(5),
        };

        let _ = DistributedRateLimiter::new(store, "invalid_limiter", config);
    }

    #[test]
    #[should_panic(expected = "initial_tokens 6 cannot exceed capacity_tokens 5")]
    fn test_rate_limiter_rejects_initial_tokens_above_capacity() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let config = RateLimiterConfig {
            capacity_tokens: 5,
            refill_rate: 1.0,
            initial_tokens: Some(6),
        };

        let _ = DistributedRateLimiter::new(store, "invalid_limiter", config);
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
        let error = limiter.try_acquire().await.expect_err("expected TokensExhausted error");
        assert!(matches!(&error, RateLimitError::TokensExhausted { .. }));
        if let RateLimitError::TokensExhausted {
            requested,
            available,
            retry_after_ms,
        } = error
        {
            assert_eq!(requested, 1);
            assert_eq!(available, 0);
            assert!(retry_after_ms > 0);
        }
    }

    #[test]
    fn test_rate_limiter_retry_after_ms_clamps_non_finite() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(store, "test_limiter", RateLimiterConfig::new(10.0, 5));

        let retry_after_ms = limiter.try_acquire_n_retry_after_ms(f64::INFINITY);
        assert_eq!(retry_after_ms, u64::MAX);
    }

    #[test]
    fn test_rate_limiter_retry_after_ms_clamps_large_finite_value() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let limiter = DistributedRateLimiter::new(store, "test_limiter", RateLimiterConfig::new(10.0, 5));

        let retry_after_ms = limiter.try_acquire_n_retry_after_ms(f64::MAX);
        assert_eq!(retry_after_ms, u64::MAX);
    }

    #[tokio::test]
    async fn test_rate_limiter_try_acquire_sleeps_before_retrying_cas_conflict() {
        let initial_state = BucketState::new(1, 1.0);
        let store = Arc::new(ConflictThenSuccessStore::new(initial_state, 3));
        let limiter = DistributedRateLimiter::new(store.clone(), "retry_limiter", RateLimiterConfig::new(1.0, 1));

        let start = crate::runtime_clock::measurement_start();
        let remaining = limiter.try_acquire_n(1).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(remaining, 0);
        assert_eq!(store.write_calls(), 4);
        assert!(elapsed >= Duration::from_millis(7), "expected at least 7ms of retry backoff, got {:?}", elapsed);
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
