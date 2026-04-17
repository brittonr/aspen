use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteResult;
use tokio::sync::oneshot;

use super::*;
use crate::verified::FlushDecision;
use crate::verified::check_batch_limits;

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "write batcher records a monotonic batch-start instant for flush timing"
)]
#[inline]
fn current_batch_start_instant() -> Instant {
    Instant::now()
}

#[inline]
fn string_len_u64(value: &str) -> u64 {
    match u64::try_from(value.len()) {
        Ok(length_bytes) => length_bytes,
        Err(_) => u64::MAX,
    }
}

#[inline]
fn pending_len_u32(state: &BatcherState) -> u32 {
    match u32::try_from(state.pending.len()) {
        Ok(pending_count) => pending_count,
        Err(_) => u32::MAX,
    }
}

struct SetPayloadRef<'a> {
    key: &'a str,
    value: &'a str,
}

#[inline]
fn set_operation_size_bytes(payload: SetPayloadRef<'_>) -> u64 {
    string_len_u64(payload.key).saturating_add(string_len_u64(payload.value))
}

#[inline]
fn should_flush_before_enqueue(state: &BatcherState, op_bytes: u64, config: &BatchConfig) -> bool {
    check_batch_limits(pending_len_u32(state), state.current_bytes, op_bytes, config.max_entries, config.max_bytes)
        .would_exceed()
}

impl WriteBatcher {
    /// Batch a Set operation (Arc version with timeout support).
    pub(super) async fn batch_set_shared(
        self: &Arc<Self>,
        key: String,
        value: String,
    ) -> Result<WriteResult, KeyValueStoreError> {
        debug_assert!(!key.is_empty(), "BATCHER: set key must not be empty");

        let (tx, rx) = oneshot::channel();
        let op_bytes = set_operation_size_bytes(SetPayloadRef {
            key: &key,
            value: &value,
        });

        let flush_action = {
            let mut state = self.state.lock().await;
            let should_flush = should_flush_before_enqueue(&state, op_bytes, &self.config);

            if should_flush {
                let batch = self.take_batch(&mut state);
                drop(state);
                self.flush_batch(batch).await;

                let mut state = self.state.lock().await;
                self.add_to_batch(&mut state, (true, key, value), op_bytes, tx);
                self.maybe_schedule_flush_shared(&mut state)
            } else {
                self.add_to_batch(&mut state, (true, key, value), op_bytes, tx);
                self.maybe_schedule_flush_shared(&mut state)
            }
        };

        match flush_action {
            FlushDecision::Immediate => {
                let batch = {
                    let mut state = self.state.lock().await;
                    self.take_batch(&mut state)
                };
                self.flush_batch(batch).await;
            }
            FlushDecision::Delayed => {
                self.schedule_flush_shared().await;
            }
            FlushDecision::None => {}
        }

        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Batch a Delete operation (Arc version with timeout support).
    pub(super) async fn batch_delete_shared(self: &Arc<Self>, key: String) -> Result<WriteResult, KeyValueStoreError> {
        debug_assert!(!key.is_empty(), "BATCHER: delete key must not be empty");

        let (tx, rx) = oneshot::channel();
        let op_bytes = string_len_u64(&key);

        let flush_action = {
            let mut state = self.state.lock().await;
            let should_flush = should_flush_before_enqueue(&state, op_bytes, &self.config);

            if should_flush {
                let batch = self.take_batch(&mut state);
                drop(state);
                self.flush_batch(batch).await;

                let mut state = self.state.lock().await;
                self.add_to_batch(&mut state, (false, key, String::new()), op_bytes, tx);
                self.maybe_schedule_flush_shared(&mut state)
            } else {
                self.add_to_batch(&mut state, (false, key, String::new()), op_bytes, tx);
                self.maybe_schedule_flush_shared(&mut state)
            }
        };

        match flush_action {
            FlushDecision::Immediate => {
                let batch = {
                    let mut state = self.state.lock().await;
                    self.take_batch(&mut state)
                };
                self.flush_batch(batch).await;
            }
            FlushDecision::Delayed => {
                self.schedule_flush_shared().await;
            }
            FlushDecision::None => {}
        }

        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Batch a Set operation (non-Arc version, no timeout).
    pub(super) async fn batch_set(&self, key: String, value: String) -> Result<WriteResult, KeyValueStoreError> {
        debug_assert!(!key.is_empty(), "BATCHER: set key must not be empty (non-Arc)");

        let (tx, rx) = oneshot::channel();
        let op_bytes = set_operation_size_bytes(SetPayloadRef {
            key: &key,
            value: &value,
        });

        let should_flush = {
            let mut state = self.state.lock().await;
            let should_flush = should_flush_before_enqueue(&state, op_bytes, &self.config);

            if should_flush {
                let batch = self.take_batch(&mut state);
                drop(state);
                self.flush_batch(batch).await;

                let mut state = self.state.lock().await;
                self.add_to_batch(&mut state, (true, key, value), op_bytes, tx);
                self.maybe_schedule_flush(&mut state)
            } else {
                self.add_to_batch(&mut state, (true, key, value), op_bytes, tx);
                self.maybe_schedule_flush(&mut state)
            }
        };

        if should_flush {
            let batch = {
                let mut state = self.state.lock().await;
                self.take_batch(&mut state)
            };
            self.flush_batch(batch).await;
        }

        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Batch a Delete operation (non-Arc version, no timeout).
    pub(super) async fn batch_delete(&self, key: String) -> Result<WriteResult, KeyValueStoreError> {
        debug_assert!(!key.is_empty(), "BATCHER: delete key must not be empty (non-Arc)");

        let (tx, rx) = oneshot::channel();
        let op_bytes = string_len_u64(&key);

        let should_flush = {
            let mut state = self.state.lock().await;
            let should_flush = should_flush_before_enqueue(&state, op_bytes, &self.config);

            if should_flush {
                let batch = self.take_batch(&mut state);
                drop(state);
                self.flush_batch(batch).await;

                let mut state = self.state.lock().await;
                self.add_to_batch(&mut state, (false, key, String::new()), op_bytes, tx);
                self.maybe_schedule_flush(&mut state)
            } else {
                self.add_to_batch(&mut state, (false, key, String::new()), op_bytes, tx);
                self.maybe_schedule_flush(&mut state)
            }
        };

        if should_flush {
            let batch = {
                let mut state = self.state.lock().await;
                self.take_batch(&mut state)
            };
            self.flush_batch(batch).await;
        }

        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Add an operation to the pending batch.
    pub(super) fn add_to_batch(
        &self,
        state: &mut BatcherState,
        operation: (bool, String, String),
        size_bytes: u64,
        result_tx: oneshot::Sender<Result<WriteResult, KeyValueStoreError>>,
    ) {
        assert!(!operation.1.is_empty(), "BATCHER: operation key must not be empty");

        if state.batch_start.is_none() {
            state.batch_start = Some(current_batch_start_instant());
        }

        state.current_bytes = state.current_bytes.saturating_add(size_bytes);
        state.pending.push(PendingWrite {
            operation,
            size_bytes,
            result_tx,
        });

        debug_assert!(
            pending_len_u32(state) <= self.config.max_entries,
            "BATCHER: pending count {} exceeds max_entries {}",
            state.pending.len(),
            self.config.max_entries
        );
    }
}
