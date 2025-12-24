//! Distributed sequence number generator.
//!
//! Generates globally unique, monotonically increasing IDs.
//! Uses batch reservation for performance.

use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::debug;

use crate::api::{KeyValueStore, KeyValueStoreError, ReadRequest, WriteCommand, WriteRequest};
use crate::coordination::error::CoordinationError;

/// Configuration for sequence generator.
#[derive(Debug, Clone)]
pub struct SequenceConfig {
    /// Number of IDs to reserve in each batch.
    pub batch_size: u64,
    /// Start value for new sequences.
    pub start_value: u64,
}

impl Default for SequenceConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
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
        Self {
            store,
            key: key.into(),
            state: Mutex::new(SequenceState {
                next: 0,
                batch_end: 0,
            }),
            config,
        }
    }

    /// Get the next sequence number.
    ///
    /// Fast path: returns from local batch.
    /// Slow path: reserves new batch from cluster.
    pub async fn next(&self) -> Result<u64, CoordinationError> {
        // Fast path: check local batch
        {
            let mut state = self.state.lock().await;
            if state.next < state.batch_end {
                let id = state.next;
                state.next += 1;
                return Ok(id);
            }
        }

        // Slow path: reserve new batch
        let batch_start = self.reserve(self.config.batch_size).await?;

        // Update local state
        let mut state = self.state.lock().await;
        state.next = batch_start + 1; // +1 because we return batch_start
        state.batch_end = batch_start + self.config.batch_size;

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
    pub async fn reserve(&self, count: u64) -> Result<u64, CoordinationError> {
        loop {
            // Read current sequence value
            let current = match self.store.read(ReadRequest::new(self.key.clone())).await {
                Ok(result) => {
                    let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                    value
                        .parse::<u64>()
                        .map_err(|_| CoordinationError::CorruptedData {
                            key: self.key.clone(),
                            reason: "not a valid u64".to_string(),
                        })?
                }
                Err(KeyValueStoreError::NotFound { .. }) => self.config.start_value - 1,
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            };

            // Check for overflow
            let new_value =
                current
                    .checked_add(count)
                    .ok_or_else(|| CoordinationError::SequenceExhausted {
                        key: self.key.clone(),
                    })?;

            // Reserve range with CAS
            let expected = if current < self.config.start_value {
                None
            } else {
                Some(current.to_string())
            };

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
                    debug!(
                        key = %self.key,
                        range_start = current + 1,
                        range_end = new_value,
                        count,
                        "reserved sequence range"
                    );
                    return Ok(current + 1); // Return start of reserved range
                }
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    // Contention, retry immediately
                    continue;
                }
                Err(e) => return Err(CoordinationError::Storage { source: e }),
            }
        }
    }

    /// Get current sequence value without incrementing.
    ///
    /// Returns the next value that would be returned by `next()`.
    pub async fn current(&self) -> Result<u64, CoordinationError> {
        // Check local batch first
        {
            let state = self.state.lock().await;
            if state.next < state.batch_end {
                return Ok(state.next);
            }
        }

        // No local batch, read from storage
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                let value =
                    value_str
                        .parse::<u64>()
                        .map_err(|_| CoordinationError::CorruptedData {
                            key: self.key.clone(),
                            reason: "not a valid u64".to_string(),
                        })?;
                Ok(value + 1) // Next would be current + 1
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
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                value
                    .parse::<u64>()
                    .map_err(|_| CoordinationError::CorruptedData {
                        key: self.key.clone(),
                        reason: "not a valid u64".to_string(),
                    })
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(0),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::inmemory::DeterministicKeyValueStore;
    use std::collections::HashSet;

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
        let seq = Arc::new(SequenceGenerator::new(
            store,
            "test_seq",
            SequenceConfig::default(),
        ));

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
            batch_size: 10,
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
        let seq = Arc::new(SequenceGenerator::new(
            store,
            "test_seq",
            SequenceConfig {
                batch_size: 10,
                ..Default::default()
            },
        ));

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
