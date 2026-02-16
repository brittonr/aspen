//! Distributed sequence number generator.
//!
//! Generates globally unique, monotonically increasing IDs.
//! Uses batch reservation for performance.
//!
//! # Verified Properties (see `verus/sequence_*.rs`)
//!
//! 1. **Uniqueness**: No two next() calls return the same value
//! 2. **Monotonicity**: Each value is strictly greater than the previous
//! 3. **Batch Disjointness**: Reserve operations return non-overlapping ranges
//! 4. **Overflow Safety**: Operations fail before overflow

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
use tokio::sync::Mutex;
use tracing::debug;

use crate::error::CoordinationError;
use crate::spec::verus_shim::*;
use crate::verified::sequence::ParseSequenceResult;
use crate::verified::sequence::SequenceReservationResult;
use crate::verified::sequence::compute_batch_end;
use crate::verified::sequence::compute_initial_current;
use crate::verified::sequence::compute_new_sequence_value;
use crate::verified::sequence::compute_next_after_refill;
use crate::verified::sequence::compute_range_start;
use crate::verified::sequence::is_initial_reservation;
use crate::verified::sequence::parse_sequence_value;
use crate::verified::sequence::should_refill_batch;

/// Configuration for sequence generator.
#[derive(Debug, Clone)]
pub struct SequenceConfig {
    /// Number of IDs to reserve in each batch.
    pub batch_size_ids: u64,
    /// Start value for new sequences.
    pub start_value: u64,
}

impl Default for SequenceConfig {
    fn default() -> Self {
        Self {
            batch_size_ids: 100,
            start_value: 1,
        }
    }
}

/// Local batch state.
struct SequenceState {
    /// Next ID to return.
    next: u64,
    /// End of current batch (exclusive).
    batch_end: u64,
}

/// Distributed sequence number generator.
///
/// Generates globally unique, monotonically increasing IDs.
/// Uses batch reservation for performance - reserves a range of IDs
/// from the cluster and hands them out locally.
pub struct SequenceGenerator<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    key: String,
    config: SequenceConfig,
    /// Local batch state.
    state: Mutex<SequenceState>,
}

impl<S: KeyValueStore + ?Sized> SequenceGenerator<S> {
    /// Create a new sequence generator.
    pub fn new(store: Arc<S>, key: impl Into<String>, config: SequenceConfig) -> Self {
        let key_str = key.into();
        // Tiger Style: argument validation
        assert!(!key_str.is_empty(), "SEQUENCE: key must not be empty");
        assert!(config.batch_size_ids > 0, "SEQUENCE: batch_size must be positive");
        assert!(config.start_value > 0, "SEQUENCE: start_value must be positive");

        Self {
            store,
            key: key_str,
            state: Mutex::new(SequenceState { next: 0, batch_end: 0 }),
            config,
        }
    }

    /// Get the next sequence number.
    ///
    /// Fast path: returns from local batch.
    /// Slow path: reserves new batch from cluster.
    pub async fn next(&self) -> Result<u64, CoordinationError> {
        // Fast path: check local batch using pure function
        {
            let mut state = self.state.lock().await;
            if !should_refill_batch(state.next, state.batch_end) {
                let id = state.next;
                debug_assert!(id > 0, "SEQUENCE: generated ID must be positive");
                state.next = state.next.saturating_add(1);
                debug_assert!(
                    state.next <= state.batch_end,
                    "SEQUENCE: next must not exceed batch_end after increment"
                );
                return Ok(id);
            }
        }

        // Slow path: reserve new batch
        let batch_start = self.reserve(self.config.batch_size_ids).await?;

        // Update local state using pure functions
        let mut state = self.state.lock().await;
        state.next = compute_next_after_refill(batch_start)
            .ok_or_else(|| CoordinationError::SequenceExhausted { key: self.key.clone() })?;
        state.batch_end = compute_batch_end(batch_start, self.config.batch_size_ids)
            .ok_or_else(|| CoordinationError::SequenceExhausted { key: self.key.clone() })?;

        debug!(
            key = %self.key,
            batch_start,
            batch_end = state.batch_end,
            "reserved new sequence batch"
        );

        Ok(batch_start)
    }

    /// Reserve a range of N sequence numbers.
    ///
    /// Returns the start of the range (inclusive).
    /// Caller gets [start, start + count).
    ///
    /// # Verified Properties
    /// - Returned range is [current + 1, current + count + 1)
    /// - Range is disjoint from all previously reserved ranges
    /// - current_value strictly increases
    pub async fn reserve(&self, count: u64) -> Result<u64, CoordinationError> {
        requires! { count > 0 }

        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let current = self.reserve_read_current().await?;

            ghost! { let pre_current = current; }

            let new_value = match compute_new_sequence_value(current, count) {
                SequenceReservationResult::Success { new_value } => new_value,
                SequenceReservationResult::Overflow => {
                    return Err(CoordinationError::SequenceExhausted { key: self.key.clone() });
                }
            };

            let expected = if is_initial_reservation(current, self.config.start_value) {
                None
            } else {
                Some(current.to_string())
            };

            match self.reserve_cas(expected, new_value).await {
                Ok(()) => {
                    return self.reserve_finalize(current, new_value, count);
                }
                Err(CoordinationError::CasConflict) => {
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        return Err(CoordinationError::MaxRetriesExceeded {
                            operation: "sequence reserve".to_string(),
                            attempts: attempt,
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Read current sequence value from store.
    async fn reserve_read_current(&self) -> Result<u64, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                match parse_sequence_value(&value_str) {
                    ParseSequenceResult::Value(v) => Ok(v),
                    ParseSequenceResult::Empty => Ok(compute_initial_current(self.config.start_value)),
                    ParseSequenceResult::Invalid => Err(CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "not a valid u64".to_string(),
                    }),
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(compute_initial_current(self.config.start_value)),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Execute CAS for sequence reservation.
    async fn reserve_cas(&self, expected: Option<String>, new_value: u64) -> Result<(), CoordinationError> {
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
            Ok(_) => Ok(()),
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Err(CoordinationError::CasConflict),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Finalize reservation and return range start.
    fn reserve_finalize(&self, current: u64, new_value: u64, count: u64) -> Result<u64, CoordinationError> {
        proof! {
            assert(new_value > current);
            assert(current + 1 > current);
        }

        debug_assert!(
            new_value > current,
            "SEQ-2: sequence values must be monotonically increasing: new={}, current={}",
            new_value,
            current
        );

        let range_start = compute_range_start(current)
            .ok_or_else(|| CoordinationError::SequenceExhausted { key: self.key.clone() })?;

        debug_assert!(range_start >= 1, "SEQ-1: range start must be positive, got {}", range_start);

        debug!(key = %self.key, range_start, range_end = new_value, count, "reserved sequence range");

        ensures! { true }

        Ok(range_start)
    }

    /// Get current sequence value without incrementing.
    ///
    /// Returns the next value that would be returned by `next()`.
    pub async fn current(&self) -> Result<u64, CoordinationError> {
        // Check local batch first using pure function
        {
            let state = self.state.lock().await;
            if !should_refill_batch(state.next, state.batch_end) {
                return Ok(state.next);
            }
        }

        // No local batch, read from storage using pure parsing
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                match parse_sequence_value(&value_str) {
                    ParseSequenceResult::Value(v) => compute_range_start(v)
                        .ok_or_else(|| CoordinationError::SequenceExhausted { key: self.key.clone() }),
                    ParseSequenceResult::Empty => Ok(self.config.start_value),
                    ParseSequenceResult::Invalid => Err(CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "not a valid u64".to_string(),
                    }),
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(self.config.start_value),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Get the last allocated sequence number.
    ///
    /// This is the highest ID that has been reserved (but maybe not yet returned).
    pub async fn last_allocated(&self) -> Result<u64, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                match parse_sequence_value(&value_str) {
                    ParseSequenceResult::Value(v) => Ok(v),
                    ParseSequenceResult::Empty => Ok(0),
                    ParseSequenceResult::Invalid => Err(CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "not a valid u64".to_string(),
                    }),
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(0),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_sequence_basic() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let seq = SequenceGenerator::new(store, "test_seq", SequenceConfig::default());

        let id1 = seq.next().await.unwrap();
        let id2 = seq.next().await.unwrap();
        let id3 = seq.next().await.unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[tokio::test]
    async fn test_sequence_uniqueness() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let seq = Arc::new(SequenceGenerator::new(store, "test_seq", SequenceConfig::default()));

        let mut ids = HashSet::new();
        for _ in 0..200 {
            let id = seq.next().await.unwrap();
            assert!(!ids.contains(&id), "Duplicate ID: {}", id);
            ids.insert(id);
        }
    }

    #[tokio::test]
    async fn test_sequence_monotonic() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let seq = SequenceGenerator::new(store, "test_seq", SequenceConfig::default());

        let mut prev = 0;
        for _ in 0..200 {
            let id = seq.next().await.unwrap();
            assert!(id > prev, "ID {} should be > {}", id, prev);
            prev = id;
        }
    }

    #[tokio::test]
    async fn test_sequence_reserve() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let seq = SequenceGenerator::new(store, "test_seq", SequenceConfig::default());

        let start = seq.reserve(50).await.unwrap();
        assert_eq!(start, 1);

        let start2 = seq.reserve(50).await.unwrap();
        assert_eq!(start2, 51);
    }

    #[tokio::test]
    async fn test_sequence_batch_efficiency() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let config = SequenceConfig {
            batch_size_ids: 10,
            start_value: 1,
        };
        let seq = SequenceGenerator::new(store.clone(), "test_seq", config);

        // Get 15 IDs - should require 2 batch reservations
        for _ in 0..15 {
            seq.next().await.unwrap();
        }

        // Check the stored value - should be 20 (2 batches of 10)
        let stored = seq.last_allocated().await.unwrap();
        assert_eq!(stored, 20);
    }

    #[tokio::test]
    async fn test_sequence_concurrent() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let seq = Arc::new(SequenceGenerator::new(store, "test_seq", SequenceConfig {
            batch_size_ids: 10,
            ..Default::default()
        }));

        let handles: Vec<_> = (0..5)
            .map(|_| {
                let s = seq.clone();
                tokio::spawn(async move {
                    let mut ids = Vec::new();
                    for _ in 0..20 {
                        ids.push(s.next().await.unwrap());
                    }
                    ids
                })
            })
            .collect();

        let mut all_ids = HashSet::new();
        for h in handles {
            let ids = h.await.unwrap();
            for id in ids {
                assert!(!all_ids.contains(&id), "Duplicate ID: {}", id);
                all_ids.insert(id);
            }
        }

        // Should have 100 unique IDs
        assert_eq!(all_ids.len(), 100);
    }
}
