//! Batch processing for the DocsExporter.
//!
//! Handles collecting KV operations into batches, flushing them,
//! and spawning the real-time export task.

use std::sync::Arc;

use anyhow::Result;
use aspen_raft::log_subscriber::KvOperation;
use aspen_raft::log_subscriber::LogEntryPayload;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::core::BATCH_FLUSH_INTERVAL;
use super::core::BatchEntry;
use super::core::DocsExporter;
use crate::constants::EXPORT_BATCH_SIZE;
use crate::constants::MAX_DOC_KEY_SIZE;
use crate::constants::MAX_DOC_VALUE_SIZE;

impl DocsExporter {
    /// Start exporting from a log subscriber broadcast channel with batching.
    ///
    /// Spawns a background task that listens to the broadcast channel
    /// and exports KV operations to the docs namespace in batches for efficiency.
    ///
    /// # Batching Behavior
    ///
    /// - Entries are buffered until EXPORT_BATCH_SIZE is reached or BATCH_FLUSH_INTERVAL elapses
    /// - On shutdown, any remaining buffered entries are flushed
    pub fn spawn(self: Arc<Self>, mut receiver: broadcast::Receiver<LogEntryPayload>) -> CancellationToken {
        let cancel = self.cancel.clone();
        let exporter = self.clone();

        tokio::spawn(async move {
            info!(
                batch_size = EXPORT_BATCH_SIZE,
                flush_interval_ms = BATCH_FLUSH_INTERVAL.as_millis(),
                "DocsExporter started with batching"
            );

            let mut batch: Vec<BatchEntry> = Vec::with_capacity(EXPORT_BATCH_SIZE as usize);
            let mut flush_interval = tokio::time::interval(BATCH_FLUSH_INTERVAL);
            flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = exporter.cancel.cancelled() => {
                        // Flush remaining entries on shutdown
                        if !batch.is_empty()
                            && let Err(e) = exporter.flush_batch(&mut batch).await
                        {
                            error!(error = %e, "failed to flush batch on shutdown");
                        }
                        info!("DocsExporter shutting down");
                        break;
                    }
                    _ = flush_interval.tick() => {
                        // Time-based flush to bound latency
                        if !batch.is_empty()
                            && let Err(e) = exporter.flush_batch(&mut batch).await
                        {
                            error!(error = %e, "failed to flush batch on interval");
                        }
                    }
                    result = receiver.recv() => {
                        match result {
                            Ok(payload) => {
                                exporter.collect_payload_to_batch(payload, &mut batch);

                                // Size-based flush when batch is full
                                if batch.len() >= EXPORT_BATCH_SIZE as usize
                                    && let Err(e) = exporter.flush_batch(&mut batch).await
                                {
                                    error!(error = %e, "failed to flush full batch");
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                // Flush remaining entries before exit
                                if !batch.is_empty()
                                    && let Err(e) = exporter.flush_batch(&mut batch).await
                                {
                                    error!(error = %e, "failed to flush batch on channel close");
                                }
                                warn!("log subscriber channel closed");
                                break;
                            }
                            Err(broadcast::error::RecvError::Lagged(count)) => {
                                warn!(lagged = count, "DocsExporter lagged behind");
                                // Continue processing - we'll catch up
                            }
                        }
                    }
                }
            }
        });

        cancel
    }

    /// Process a single log entry payload (non-batched, for testing).
    #[allow(dead_code)]
    pub(crate) async fn process_payload(&self, payload: LogEntryPayload) -> Result<()> {
        match &payload.operation {
            KvOperation::Set { key, value }
            | KvOperation::SetWithTTL { key, value, .. }
            | KvOperation::SetWithLease { key, value, .. } => {
                self.export_set(key, value, payload.index).await?;
            }
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => {
                for (key, value) in pairs {
                    self.export_set(key, value, payload.index).await?;
                }
            }
            KvOperation::Delete { key } => {
                self.export_delete(key, payload.index).await?;
            }
            KvOperation::DeleteMulti { keys } => {
                for key in keys {
                    self.export_delete(key, payload.index).await?;
                }
            }
            KvOperation::CompareAndSwap { key, new_value, .. } => {
                // CAS is a conditional set - export the new value if the operation succeeded
                self.export_set(key, new_value, payload.index).await?;
            }
            KvOperation::CompareAndDelete { key, .. } => {
                // CAS delete - export the deletion if the operation succeeded
                self.export_delete(key, payload.index).await?;
            }
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                // Process each operation in the batch
                for (is_set, key, value) in operations {
                    if *is_set {
                        self.export_set(key, value, payload.index).await?;
                    } else {
                        self.export_delete(key, payload.index).await?;
                    }
                }
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => {
                // Skip non-KV operations
                debug!(log_index = payload.index, "skipping non-KV entry");
            }
            KvOperation::Transaction { success, failure, .. } => {
                // Process all put/delete operations from both branches
                // Note: Only one branch executes at runtime, but log subscriber
                // doesn't track which succeeded, so we export both conservatively
                let process_ops = |ops: &[(u8, Vec<u8>, Vec<u8>)]| {
                    for (op_type, key, value) in ops {
                        match *op_type {
                            0 => { /* Put */ }
                            1 => { /* Delete */ }
                            _ => { /* Get/Range - no export needed */ }
                        }
                        // For now, skip transaction ops in single-entry path
                        // They'll be handled via batch processing
                        let _ = (key, value);
                    }
                };
                process_ops(success);
                process_ops(failure);
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Process all write operations
                for (is_set, key, value) in write_set {
                    if *is_set {
                        self.export_set(key, value, payload.index).await?;
                    } else {
                        self.export_delete(key, payload.index).await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Collect entries from a log payload into the batch buffer.
    ///
    /// Filters out entries that exceed size limits.
    pub(crate) fn collect_payload_to_batch(&self, payload: LogEntryPayload, batch: &mut Vec<BatchEntry>) {
        match payload.operation {
            KvOperation::Set { key, value }
            | KvOperation::SetWithTTL { key, value, .. }
            | KvOperation::SetWithLease { key, value, .. } => {
                push_set_entry_if_valid(batch, key, value);
            }
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => {
                for (key, value) in pairs {
                    push_set_entry_if_valid(batch, key, value);
                }
            }
            KvOperation::Delete { key } => {
                push_delete_entry(batch, key);
            }
            KvOperation::DeleteMulti { keys } => {
                for key in keys {
                    push_delete_entry(batch, key);
                }
            }
            KvOperation::CompareAndSwap { key, new_value, .. } => {
                push_set_entry_if_valid(batch, key, new_value);
            }
            KvOperation::CompareAndDelete { key, .. } => {
                push_delete_entry(batch, key);
            }
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                collect_batch_operations(batch, operations);
            }
            KvOperation::Transaction { success, failure, .. } => {
                collect_transaction_operations(batch, success.into_iter().chain(failure));
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                collect_batch_operations(batch, write_set);
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => {
                // Skip non-KV operations
            }
        }
    }

    /// Flush the batch buffer to the docs writer.
    pub(crate) async fn flush_batch(&self, batch: &mut Vec<BatchEntry>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let count = batch.len();
        debug!(count, "flushing export batch");

        // Take ownership of batch contents
        let entries = std::mem::take(batch);

        self.writer.write_batch(entries).await?;

        // Emit event for hook integration
        if let Some(broadcaster) = &self.event_broadcaster {
            broadcaster.emit_entry_exported(count as u32, count as u64);
        }

        debug!(count, "batch flushed successfully");
        Ok(())
    }

    /// Export a Set operation to docs (non-batched, for testing).
    #[allow(dead_code)]
    pub(crate) async fn export_set(&self, key: &[u8], value: &[u8], log_index: u64) -> Result<()> {
        // Validate sizes
        if key.len() > MAX_DOC_KEY_SIZE {
            warn!(key_len = key.len(), max = MAX_DOC_KEY_SIZE, "key too large for docs export, skipping");
            return Ok(());
        }
        if value.len() > MAX_DOC_VALUE_SIZE {
            warn!(value_len = value.len(), max = MAX_DOC_VALUE_SIZE, "value too large for docs export, skipping");
            return Ok(());
        }

        self.writer.set_entry(key.to_vec(), value.to_vec()).await?;

        debug!(log_index, key_len = key.len(), "exported Set to docs");
        Ok(())
    }

    /// Export a Delete operation to docs (non-batched, for testing).
    #[allow(dead_code)]
    pub(crate) async fn export_delete(&self, key: &[u8], log_index: u64) -> Result<()> {
        self.writer.delete_entry(key.to_vec()).await?;

        debug!(log_index, key_len = key.len(), "exported Delete to docs");
        Ok(())
    }
}

// ============================================================================
// Helper Functions for Batch Collection
// ============================================================================

/// Push a set entry to the batch if it passes size validation.
///
/// Logs a warning and skips entries that exceed size limits.
#[inline]
fn push_set_entry_if_valid(batch: &mut Vec<BatchEntry>, key: Vec<u8>, value: Vec<u8>) {
    if key.len() <= MAX_DOC_KEY_SIZE && value.len() <= MAX_DOC_VALUE_SIZE {
        batch.push(BatchEntry {
            key,
            value,
            is_delete: false,
        });
    } else {
        warn!(key_len = key.len(), value_len = value.len(), "entry too large for docs export, skipping");
    }
}

/// Push a delete entry to the batch (tombstone).
#[inline]
fn push_delete_entry(batch: &mut Vec<BatchEntry>, key: Vec<u8>) {
    batch.push(BatchEntry {
        key,
        value: vec![],
        is_delete: true,
    });
}

/// Collect batch operations (is_set, key, value) tuples into the batch.
///
/// Used by Batch, ConditionalBatch, and OptimisticTransaction operations.
fn collect_batch_operations(batch: &mut Vec<BatchEntry>, operations: Vec<(bool, Vec<u8>, Vec<u8>)>) {
    for (is_set, key, value) in operations {
        if is_set {
            push_set_entry_if_valid(batch, key, value);
        } else {
            push_delete_entry(batch, key);
        }
    }
}

/// Collect transaction operations (op_type, key, value) tuples into the batch.
///
/// op_type: 0 = Put, 1 = Delete, other = skip (Get/Range)
fn collect_transaction_operations(
    batch: &mut Vec<BatchEntry>,
    operations: impl Iterator<Item = (u8, Vec<u8>, Vec<u8>)>,
) {
    for (op_type, key, value) in operations {
        match op_type {
            0 => push_set_entry_if_valid(batch, key, value), // Put
            1 => push_delete_entry(batch, key),              // Delete
            _ => {}                                          // Get/Range - no export needed
        }
    }
}
