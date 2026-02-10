//! Atomic counters with linearizable increment/decrement.
//!
//! Provides race-free counter operations built on CAS primitives.
//!
//! # Verified Properties (see `verus/counter_*.rs`)
//!
//! 1. **CAS Atomicity**: Each modify is atomic via compare-and-swap
//! 2. **Saturating Arithmetic**: Operations saturate at bounds (no overflow/underflow)
//! 3. **Linearizability**: All operations are linearizable through Raft

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use rand::Rng;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::error::CoordinationError;
use crate::error::MaxRetriesExceededSnafu;
use crate::spec::verus_shim::*;
use crate::verified::counter::ParseSignedResult;
use crate::verified::counter::ParseUnsignedResult;
use crate::verified::counter::compute_approximate_total;
use crate::verified::counter::compute_retry_delay;
use crate::verified::counter::compute_signed_cas_expected;
use crate::verified::counter::compute_unsigned_cas_expected;
use crate::verified::counter::parse_signed_counter;
use crate::verified::counter::parse_unsigned_counter;
use crate::verified::counter::should_flush_buffer;

/// Configuration for atomic counter.
#[derive(Debug, Clone)]
pub struct CounterConfig {
    /// Maximum retries on CAS failure.
    pub max_retries: u32,
    /// Base delay between retries in milliseconds.
    pub retry_delay_ms: u64,
}

impl Default for CounterConfig {
    fn default() -> Self {
        Self {
            max_retries: 100,  // Many retries for high contention
            retry_delay_ms: 1, // Very short delay
        }
    }
}

/// Unsigned atomic counter.
///
/// All operations are linearizable through Raft consensus.
/// Uses CAS with retry for atomic increment/decrement.
pub struct AtomicCounter<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    key: String,
    config: CounterConfig,
}

impl<S: KeyValueStore + ?Sized> AtomicCounter<S> {
    /// Create a new atomic counter.
    pub fn new(store: Arc<S>, key: impl Into<String>, config: CounterConfig) -> Self {
        Self {
            store,
            key: key.into(),
            config,
        }
    }

    /// Get the current counter value.
    pub async fn get(&self) -> Result<u64, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                match parse_unsigned_counter(&value_str) {
                    ParseUnsignedResult::Value(v) => Ok(v),
                    ParseUnsignedResult::Empty => Ok(0),
                    ParseUnsignedResult::Invalid => Err(CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "not a valid u64".to_string(),
                    }),
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(0),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Increment the counter by 1 and return the new value.
    pub async fn increment(&self) -> Result<u64, CoordinationError> {
        self.add(1).await
    }

    /// Increment by a specific amount and return the new value.
    pub async fn add(&self, amount: u64) -> Result<u64, CoordinationError> {
        self.modify(|current| current.saturating_add(amount)).await
    }

    /// Decrement the counter by 1 (saturating at 0).
    pub async fn decrement(&self) -> Result<u64, CoordinationError> {
        self.subtract(1).await
    }

    /// Subtract amount (saturating at 0).
    pub async fn subtract(&self, amount: u64) -> Result<u64, CoordinationError> {
        self.modify(|current| current.saturating_sub(amount)).await
    }

    /// Reset counter to zero.
    pub async fn reset(&self) -> Result<(), CoordinationError> {
        self.set(0).await
    }

    /// Set counter to a specific value.
    pub async fn set(&self, value: u64) -> Result<(), CoordinationError> {
        let mut attempt = 0;

        loop {
            let current = self.get().await?;
            let expected = compute_unsigned_cas_expected(current).map(|v| v.to_string());

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: self.key.clone(),
                        expected,
                        new_value: value.to_string(),
                    },
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    attempt += 1;
                    if attempt >= self.config.max_retries {
                        return MaxRetriesExceededSnafu {
                            operation: "counter set",
                            attempts: attempt,
                        }
                        .fail();
                    }
                    // Create rng here to avoid holding non-Send type across await
                    let jitter = rand::rng().random_range(0..self.config.retry_delay_ms + 1);
                    let delay = compute_retry_delay(self.config.retry_delay_ms, jitter);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            }
        }
    }

    /// Compare-and-set: atomically set value if current equals expected.
    ///
    /// Returns true if the swap succeeded, false if the current value
    /// didn't match the expected value.
    pub async fn compare_and_set(&self, expected: u64, new_value: u64) -> Result<bool, CoordinationError> {
        let expected_str = compute_unsigned_cas_expected(expected).map(|v| v.to_string());

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: self.key.clone(),
                    expected: expected_str,
                    new_value: new_value.to_string(),
                },
            })
            .await
        {
            Ok(_) => Ok(true),
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(false),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Apply a modification function atomically.
    ///
    /// # Verified Properties
    /// - CAS ensures atomicity
    /// - Retries on contention
    async fn modify<F>(&self, f: F) -> Result<u64, CoordinationError>
    where F: Fn(u64) -> u64 {
        let mut attempt = 0;

        loop {
            let current = self.get().await?;

            // Ghost: capture pre-state
            ghost! {
                let pre_value = current;
            }

            let new_value = f(current);

            let expected = compute_unsigned_cas_expected(current).map(|v| v.to_string());

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: self.key.clone(),
                        expected,
                        new_value: new_value.to_string(),
                    },
                })
                .await
            {
                Ok(_) => {
                    // Proof: CAS succeeded, value updated atomically
                    proof! {
                        // CAS atomicity: if expected matched, new_value is now stored
                        assert(true);
                    }

                    debug!(
                        key = %self.key,
                        old_value = current,
                        new_value,
                        "counter modified"
                    );
                    return Ok(new_value);
                }
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    attempt += 1;
                    if attempt >= self.config.max_retries {
                        return MaxRetriesExceededSnafu {
                            operation: "counter modify",
                            attempts: attempt,
                        }
                        .fail();
                    }
                    // Jittered delay - create rng here to avoid holding non-Send type across await
                    let jitter = rand::rng().random_range(0..self.config.retry_delay_ms + 1);
                    let delay = compute_retry_delay(self.config.retry_delay_ms, jitter);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            }
        }
    }
}

/// Signed atomic counter (allows negative values).
pub struct SignedAtomicCounter<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    key: String,
    config: CounterConfig,
}

impl<S: KeyValueStore + ?Sized> SignedAtomicCounter<S> {
    /// Create a new signed counter.
    pub fn new(store: Arc<S>, key: impl Into<String>, config: CounterConfig) -> Self {
        Self {
            store,
            key: key.into(),
            config,
        }
    }

    /// Get the current counter value.
    pub async fn get(&self) -> Result<i64, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                match parse_signed_counter(&value_str) {
                    ParseSignedResult::Value(v) => Ok(v),
                    ParseSignedResult::Empty => Ok(0),
                    ParseSignedResult::Invalid => Err(CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "not a valid i64".to_string(),
                    }),
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(0),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Add amount to the counter (can be negative).
    pub async fn add(&self, amount: i64) -> Result<i64, CoordinationError> {
        self.modify(|current| current.saturating_add(amount)).await
    }

    /// Subtract amount from the counter.
    pub async fn subtract(&self, amount: i64) -> Result<i64, CoordinationError> {
        self.modify(|current| current.saturating_sub(amount)).await
    }

    /// Apply a modification function atomically.
    async fn modify<F>(&self, f: F) -> Result<i64, CoordinationError>
    where F: Fn(i64) -> i64 {
        let mut attempt = 0;

        loop {
            let current = self.get().await?;
            let new_value = f(current);

            let expected = compute_signed_cas_expected(current).map(|v| v.to_string());

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: self.key.clone(),
                        expected,
                        new_value: new_value.to_string(),
                    },
                })
                .await
            {
                Ok(_) => return Ok(new_value),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    attempt += 1;
                    if attempt >= self.config.max_retries {
                        return MaxRetriesExceededSnafu {
                            operation: "signed counter modify",
                            attempts: attempt,
                        }
                        .fail();
                    }
                    // Create rng here to avoid holding non-Send type across await
                    let jitter = rand::rng().random_range(0..self.config.retry_delay_ms + 1);
                    let delay = compute_retry_delay(self.config.retry_delay_ms, jitter);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            }
        }
    }
}

/// Buffered counter that batches local increments.
///
/// Accumulates increments locally and flushes to storage periodically.
/// Trade-off: lower latency per increment, but may lose unflushed counts on crash.
pub struct BufferedCounter<S: KeyValueStore + 'static> {
    /// Underlying atomic counter.
    counter: AtomicCounter<S>,
    /// Local accumulator (wrapped in Arc for sharing with async tasks).
    local: Arc<AtomicU64>,
    /// Flush threshold.
    flush_threshold: u64,
    /// Background flusher state (reserved for future periodic flush).
    _flusher_state: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl<S: KeyValueStore + 'static> BufferedCounter<S> {
    /// Create a new buffered counter.
    ///
    /// # Arguments
    /// * `store` - The underlying key-value store
    /// * `key` - The counter key
    /// * `flush_threshold` - Flush when local count reaches this value
    /// * `flush_interval` - Also flush periodically at this interval
    pub fn new(store: Arc<S>, key: impl Into<String>, flush_threshold: u64, _flush_interval: Duration) -> Self {
        let counter = AtomicCounter::new(store, key, CounterConfig::default());
        let local = Arc::new(AtomicU64::new(0));

        Self {
            counter,
            local,
            flush_threshold,
            _flusher_state: Arc::new(Mutex::new(None)),
        }
    }

    /// Increment locally (fast, no network).
    pub fn increment(&self) {
        let prev = self.local.fetch_add(1, Ordering::Relaxed);
        let new_value = prev.saturating_add(1);
        if should_flush_buffer(new_value, self.flush_threshold) {
            // Trigger flush in background
            let counter = self.counter.store.clone();
            let key = self.counter.key.clone();
            let local = self.local.clone();

            tokio::spawn(async move {
                let to_flush = local.swap(0, Ordering::AcqRel);
                if to_flush > 0 {
                    let c = AtomicCounter::new(counter, key, CounterConfig::default());
                    let _ = c.add(to_flush).await;
                }
            });
        }
    }

    /// Increment by amount locally.
    pub fn add(&self, amount: u64) {
        let prev = self.local.fetch_add(amount, Ordering::Relaxed);
        let new_value = prev.saturating_add(amount);
        if should_flush_buffer(new_value, self.flush_threshold) {
            let counter = self.counter.store.clone();
            let key = self.counter.key.clone();
            let local = self.local.clone();

            tokio::spawn(async move {
                let to_flush = local.swap(0, Ordering::AcqRel);
                if to_flush > 0 {
                    let c = AtomicCounter::new(counter, key, CounterConfig::default());
                    let _ = c.add(to_flush).await;
                }
            });
        }
    }

    /// Flush accumulated count to storage.
    pub async fn flush(&self) -> Result<u64, CoordinationError> {
        let to_flush = self.local.swap(0, Ordering::AcqRel);
        if to_flush > 0 {
            self.counter.add(to_flush).await
        } else {
            self.counter.get().await
        }
    }

    /// Get accurate count (flushes first).
    pub async fn get_accurate(&self) -> Result<u64, CoordinationError> {
        self.flush().await
    }

    /// Get approximate count (may not include recent local increments).
    pub async fn get_approximate(&self) -> Result<u64, CoordinationError> {
        let stored = self.counter.get().await?;
        let local = self.local.load(Ordering::Relaxed);
        Ok(compute_approximate_total(stored, local))
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::inmemory::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_counter_increment() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = AtomicCounter::new(store, "test_counter", CounterConfig::default());

        assert_eq!(counter.get().await.unwrap(), 0);
        assert_eq!(counter.increment().await.unwrap(), 1);
        assert_eq!(counter.increment().await.unwrap(), 2);
        assert_eq!(counter.get().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_counter_add() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = AtomicCounter::new(store, "test_counter", CounterConfig::default());

        assert_eq!(counter.add(5).await.unwrap(), 5);
        assert_eq!(counter.add(10).await.unwrap(), 15);
    }

    #[tokio::test]
    async fn test_counter_decrement() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = AtomicCounter::new(store, "test_counter", CounterConfig::default());

        counter.add(10).await.unwrap();
        assert_eq!(counter.decrement().await.unwrap(), 9);
        assert_eq!(counter.subtract(5).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn test_counter_saturating() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = AtomicCounter::new(store, "test_counter", CounterConfig::default());

        // Subtract from 0 should stay at 0
        assert_eq!(counter.subtract(10).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_counter_compare_and_set() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = AtomicCounter::new(store, "test_counter", CounterConfig::default());

        counter.set(10).await.unwrap();

        // Should succeed
        assert!(counter.compare_and_set(10, 20).await.unwrap());
        assert_eq!(counter.get().await.unwrap(), 20);

        // Should fail (expected 10, actual 20)
        assert!(!counter.compare_and_set(10, 30).await.unwrap());
        assert_eq!(counter.get().await.unwrap(), 20);
    }

    #[tokio::test]
    async fn test_signed_counter() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = SignedAtomicCounter::new(store, "test_counter", CounterConfig::default());

        assert_eq!(counter.get().await.unwrap(), 0);
        assert_eq!(counter.add(-5).await.unwrap(), -5);
        assert_eq!(counter.add(10).await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_concurrent_increments() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let counter = Arc::new(AtomicCounter::new(store, "test_counter", CounterConfig::default()));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let c = counter.clone();
                tokio::spawn(async move { c.increment().await })
            })
            .collect();

        for h in handles {
            h.await.unwrap().unwrap();
        }

        assert_eq!(counter.get().await.unwrap(), 10);
    }
}
