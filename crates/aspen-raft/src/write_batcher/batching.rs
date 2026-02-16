use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteResult;
use tokio::sync::oneshot;

use super::*;
use crate::verified::FlushDecision;
use crate::verified::check_batch_limits;

impl WriteBatcher {
    /// Batch a Set operation (Arc version with timeout support).
    pub(super) async fn batch_set_shared(
        self: &Arc<Self>,
        key: String,
        value: String,
    ) -> Result<WriteResult, KeyValueStoreError> {
        // Tiger Style: key must not be empty
        debug_assert!(!key.is_empty(), "BATCHER: set key must not be empty");

        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len() + value.len();

        let flush_action = {
            let mut state = self.state.lock().await;

            // Check if adding this would exceed limits (pure function)
            let limit_check = check_batch_limits(
                state.pending.len(),
                state.current_bytes,
                op_bytes,
                self.config.max_entries,
                self.config.max_bytes,
            );

            if limit_check.would_exceed() {
                // Flush existing batch first, then add this to new batch
                let batch = self.take_batch(&mut state);
                drop(state);
                self.flush_batch(batch).await;

                // Re-acquire lock and add to fresh batch
                let mut state = self.state.lock().await;
                self.add_to_batch(&mut state, (true, key, value), op_bytes, tx);
                self.maybe_schedule_flush_shared(&mut state)
            } else {
                self.add_to_batch(&mut state, (true, key, value), op_bytes, tx);
                self.maybe_schedule_flush_shared(&mut state)
            }
        };

        // Handle flush action outside of lock
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

        // Wait for result
        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Batch a Delete operation (Arc version with timeout support).
    pub(super) async fn batch_delete_shared(self: &Arc<Self>, key: String) -> Result<WriteResult, KeyValueStoreError> {
        // Tiger Style: key must not be empty
        debug_assert!(!key.is_empty(), "BATCHER: delete key must not be empty");

        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len();

        let flush_action = {
            let mut state = self.state.lock().await;

            // Check if adding this would exceed limits (pure function)
            let limit_check = check_batch_limits(
                state.pending.len(),
                state.current_bytes,
                op_bytes,
                self.config.max_entries,
                self.config.max_bytes,
            );

            if limit_check.would_exceed() {
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
        // Tiger Style: key must not be empty
        debug_assert!(!key.is_empty(), "BATCHER: set key must not be empty (non-Arc)");

        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len() + value.len();

        let should_flush = {
            let mut state = self.state.lock().await;

            // Check if adding this would exceed limits (pure function)
            let limit_check = check_batch_limits(
                state.pending.len(),
                state.current_bytes,
                op_bytes,
                self.config.max_entries,
                self.config.max_bytes,
            );

            if limit_check.would_exceed() {
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
        // Tiger Style: key must not be empty
        debug_assert!(!key.is_empty(), "BATCHER: delete key must not be empty (non-Arc)");

        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len();

        let should_flush = {
            let mut state = self.state.lock().await;

            // Check if adding this would exceed limits (pure function)
            let limit_check = check_batch_limits(
                state.pending.len(),
                state.current_bytes,
                op_bytes,
                self.config.max_entries,
                self.config.max_bytes,
            );

            if limit_check.would_exceed() {
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
        size_bytes: usize,
        result_tx: oneshot::Sender<Result<WriteResult, KeyValueStoreError>>,
    ) {
        // Tiger Style: operation key must not be empty
        assert!(!operation.1.is_empty(), "BATCHER: operation key must not be empty");

        if state.batch_start.is_none() {
            state.batch_start = Some(Instant::now());
        }

        state.current_bytes += size_bytes;
        state.pending.push(PendingWrite {
            operation,
            size_bytes,
            result_tx,
        });

        // Tiger Style: pending batch must not exceed configured max_entries
        debug_assert!(
            state.pending.len() <= self.config.max_entries,
            "BATCHER: pending count {} exceeds max_entries {}",
            state.pending.len(),
            self.config.max_entries
        );
    }
}
