use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteResult;

use super::*;
use crate::types::AppRequest;
use crate::verified::FlushDecision;
use crate::verified::determine_flush_action;

impl WriteBatcher {
    /// Check if we should schedule a flush (Arc version).
    pub(super) fn maybe_schedule_flush_shared(&self, state: &mut BatcherState) -> FlushDecision {
        // Use pure function for flush decision logic
        let decision = determine_flush_action(
            state.pending.len() as u32,
            state.current_bytes,
            self.config.max_entries,
            self.config.max_bytes,
            self.config.max_wait.is_zero(),
            state.flush_scheduled,
        );

        // Update state if scheduling a delayed flush
        if decision == FlushDecision::Delayed {
            state.flush_scheduled = true;
        }

        decision
    }

    /// Check if we should schedule a flush (non-Arc version).
    pub(super) fn maybe_schedule_flush(&self, state: &mut BatcherState) -> bool {
        // Use pure function for flush decision logic
        // Non-Arc version treats Delayed as no-flush (can't spawn background task)
        let decision = determine_flush_action(
            state.pending.len() as u32,
            state.current_bytes,
            self.config.max_entries,
            self.config.max_bytes,
            self.config.max_wait.is_zero(),
            state.flush_scheduled,
        );

        decision == FlushDecision::Immediate
    }

    /// Schedule a delayed flush (Arc version with background task).
    pub(super) async fn schedule_flush_shared(self: &Arc<Self>) {
        let batcher = Arc::clone(self);
        let max_wait = self.config.max_wait;

        let mut flush_tasks = self.flush_tasks.lock().await;

        // Reap completed flush tasks to prevent unbounded growth
        while flush_tasks.try_join_next().is_some() {}

        flush_tasks.spawn(async move {
            tokio::time::sleep(max_wait).await;

            let batch = {
                let mut state = batcher.state.lock().await;

                // Check if batch was already flushed
                if state.pending.is_empty() {
                    state.flush_scheduled = false;
                    return;
                }

                state.flush_scheduled = false;
                batcher.take_batch(&mut state)
            };

            batcher.flush_batch(batch).await;
        });
    }

    /// Take the current batch from state.
    pub(super) fn take_batch(&self, state: &mut BatcherState) -> Vec<PendingWrite> {
        let batch = std::mem::take(&mut state.pending);
        state.batch_start = None;
        state.current_bytes = 0;
        state.flush_scheduled = false;

        // Tiger Style: state must be fully reset after taking batch
        debug_assert!(
            state.pending.is_empty() && state.current_bytes == 0,
            "TAKE_BATCH: state must be reset after take"
        );

        batch
    }

    /// Submit a batch to Raft and notify all waiters.
    pub(super) async fn flush_batch(&self, batch: Vec<PendingWrite>) {
        if batch.is_empty() {
            return;
        }

        // Tiger Style: batch size must be bounded
        assert!(
            batch.len() as u32 <= self.config.max_entries,
            "FLUSH: batch size {} exceeds max_entries {}",
            batch.len(),
            self.config.max_entries
        );

        // Build the Raft batch operation
        let operations: Vec<(bool, String, String)> = batch.iter().map(|p| p.operation.clone()).collect();

        // Tiger Style: all operation keys in batch must be non-empty
        debug_assert!(
            operations.iter().all(|(_, k, _)| !k.is_empty()),
            "FLUSH: all batch operation keys must be non-empty"
        );

        let app_request = AppRequest::Batch { operations };

        let batch_size = batch.len();

        // Submit to Raft
        let result = self.raft.client_write(app_request).await;

        // Convert to WriteResult and notify all waiters
        let write_result = match result {
            Ok(_resp) => Ok(WriteResult {
                command: None,
                batch_applied: Some(batch_size as u32),
                conditions_met: None,
                failed_condition_index: None,
                lease_id: None,
                ttl_seconds: None,
                keys_deleted: None,
                succeeded: None,
                txn_results: None,
                header_revision: None,
                occ_conflict: None,
                conflict_key: None,
                conflict_expected_version: None,
                conflict_actual_version: None,
            }),
            Err(e) => Err(KeyValueStoreError::Failed {
                reason: format!("raft error: {}", e),
            }),
        };

        // Clone result for each waiter
        for pending in batch {
            let _ = pending.result_tx.send(write_result.clone());
        }
    }
}
