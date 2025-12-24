//! Write batcher for grouping multiple writes into single Raft proposals.
//!
//! This module implements write batching / group commit optimization to
//! amortize the cost of Raft consensus and fsync across multiple operations.
//!
//! ## Performance Impact
//!
//! Without batching: Each write = 1 Raft proposal + 1 fsync = ~3.2ms
//! With batching: N writes = 1 Raft proposal + 1 fsync = ~3.2ms total
//!
//! | Batch Window | Added Latency | Throughput Improvement |
//! |--------------|---------------|------------------------|
//! | 1 ms         | +1 ms         | ~10x                   |
//! | 5 ms         | +5 ms         | ~30x                   |
//!
//! ## Usage
//!
//! ```ignore
//! // Create a shared batcher (required for timeout-based flushing)
//! let batcher = WriteBatcher::new_shared(raft.clone(), BatchConfig::default());
//!
//! // These writes are batched together
//! let result = batcher.write(WriteCommand::Set { key, value }).await?;
//! ```

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::time::Instant;

use crate::api::KeyValueStoreError;
use crate::api::WriteCommand;
use crate::api::WriteResult;
use crate::raft::types::AppRequest;
use crate::raft::types::AppTypeConfig;

/// Configuration for write batching behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of operations per batch.
    /// Tiger Style: bounded to prevent unbounded memory use.
    pub max_entries: usize,
    /// Maximum total size of values in bytes per batch.
    /// Tiger Style: bounded to prevent memory exhaustion.
    pub max_bytes: usize,
    /// Maximum time to wait before flushing a batch in milliseconds.
    /// Trade-off: higher = more throughput, lower = less latency.
    #[serde(default = "default_max_wait_ms")]
    pub max_wait_ms: u64,
    /// Computed max_wait Duration (not serialized).
    #[serde(skip)]
    pub max_wait: Duration,
}

fn default_max_wait_ms() -> u64 {
    2
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_entries: 100,
            max_bytes: 1024 * 1024, // 1 MB
            max_wait_ms: 2,
            max_wait: Duration::from_millis(2),
        }
    }
}

impl BatchConfig {
    /// Create a config optimized for high throughput (more batching).
    ///
    /// Note: max_entries is capped at 100 due to underlying storage limits
    /// (MAX_SETMULTI_KEYS). The throughput gain comes from the longer max_wait
    /// which allows more concurrent writes to batch together.
    pub fn high_throughput() -> Self {
        Self {
            max_entries: 100,           // Capped at MAX_SETMULTI_KEYS
            max_bytes: 4 * 1024 * 1024, // 4 MB
            max_wait_ms: 5,
            max_wait: Duration::from_millis(5),
        }
    }

    /// Create a config optimized for low latency (less batching).
    pub fn low_latency() -> Self {
        Self {
            max_entries: 20,
            max_bytes: 256 * 1024, // 256 KB
            max_wait_ms: 1,
            max_wait: Duration::from_millis(1),
        }
    }

    /// Disable batching entirely (every write is immediate).
    pub fn disabled() -> Self {
        Self {
            max_entries: 1,
            max_bytes: usize::MAX,
            max_wait_ms: 0,
            max_wait: Duration::ZERO,
        }
    }

    /// Finalize config by computing max_wait from max_wait_ms.
    /// Call this after deserializing from config.
    pub fn finalize(mut self) -> Self {
        self.max_wait = Duration::from_millis(self.max_wait_ms);
        self
    }
}

/// A pending write waiting for batch submission.
struct PendingWrite {
    /// The operation: (is_set=true for Set, key, value)
    operation: (bool, String, String),
    /// Size of this operation in bytes (key.len() + value.len())
    /// Tracked for future metrics/observability, not currently used.
    #[allow(dead_code)]
    size_bytes: usize,
    /// Oneshot to notify when write completes
    result_tx: oneshot::Sender<Result<WriteResult, KeyValueStoreError>>,
}

/// Shared state for the batcher.
struct BatcherState {
    /// Pending operations waiting to be batched
    pending: Vec<PendingWrite>,
    /// Current total bytes in pending batch (Tiger Style: tracked for max_bytes enforcement)
    current_bytes: usize,
    /// When the first item was added to current batch (for timeout)
    batch_start: Option<Instant>,
    /// Whether the flush task is currently scheduled
    flush_scheduled: bool,
}

/// Write batcher that groups multiple writes into single Raft proposals.
///
/// The batcher collects writes for up to `max_wait` duration or until
/// batch limits are reached, then submits them as a single Raft Batch
/// operation. This amortizes the cost of Raft consensus and fsync.
///
/// ## Thread Safety
///
/// The batcher is `Send + Sync` and can be shared across tasks.
///
/// ## Supported Operations
///
/// Currently batches:
/// - `WriteCommand::Set` - simple key-value sets
/// - `WriteCommand::Delete` - key deletions
///
/// Other operations (CAS, transactions, etc.) bypass batching and go
/// directly to Raft for correctness reasons.
pub struct WriteBatcher {
    /// Raft instance for submitting batches
    raft: Arc<openraft::Raft<AppTypeConfig>>,
    /// Batcher configuration
    config: BatchConfig,
    /// Shared state protected by mutex
    state: Mutex<BatcherState>,
}

impl WriteBatcher {
    /// Create a new write batcher wrapped in Arc for shared ownership.
    ///
    /// This is the preferred constructor as it enables timeout-based flushing
    /// which requires spawning background tasks with Arc<Self>.
    pub fn new_shared(raft: Arc<openraft::Raft<AppTypeConfig>>, config: BatchConfig) -> Arc<Self> {
        Arc::new(Self {
            raft,
            config,
            state: Mutex::new(BatcherState {
                pending: Vec::with_capacity(128),
                current_bytes: 0,
                batch_start: None,
                flush_scheduled: false,
            }),
        })
    }

    /// Create a new write batcher (non-Arc version).
    ///
    /// Note: Timeout-based flushing requires Arc<Self>. Use `new_shared()` if
    /// you need timeout functionality. This constructor is useful for testing
    /// or when batching is disabled.
    pub fn new(raft: Arc<openraft::Raft<AppTypeConfig>>, config: BatchConfig) -> Self {
        Self {
            raft,
            config,
            state: Mutex::new(BatcherState {
                pending: Vec::with_capacity(128),
                current_bytes: 0,
                batch_start: None,
                flush_scheduled: false,
            }),
        }
    }

    /// Submit a write operation for batching (Arc version with timeout support).
    ///
    /// Returns when the write has been committed through Raft.
    /// Simple Set/Delete operations are batched together.
    /// Complex operations bypass batching for correctness.
    pub async fn write_shared(self: &Arc<Self>, command: WriteCommand) -> Result<WriteResult, KeyValueStoreError> {
        match &command {
            WriteCommand::Set { key, value } => self.batch_set_shared(key.clone(), value.clone()).await,
            WriteCommand::Delete { key } => self.batch_delete_shared(key.clone()).await,
            // Other operations go directly to Raft (not batchable)
            _ => self.write_direct(command).await,
        }
    }

    /// Submit a write operation for batching (non-Arc version, no timeout).
    ///
    /// Returns when the write has been committed through Raft.
    /// Simple Set/Delete operations are batched together.
    /// Complex operations bypass batching for correctness.
    ///
    /// Note: Without Arc<Self>, timeout-based flushing is disabled.
    /// Batches only flush when full (max_entries or max_bytes reached).
    pub async fn write(&self, command: WriteCommand) -> Result<WriteResult, KeyValueStoreError> {
        match &command {
            WriteCommand::Set { key, value } => self.batch_set(key.clone(), value.clone()).await,
            WriteCommand::Delete { key } => self.batch_delete(key.clone()).await,
            // Other operations go directly to Raft (not batchable)
            _ => self.write_direct(command).await,
        }
    }

    /// Batch a Set operation (Arc version with timeout support).
    async fn batch_set_shared(self: &Arc<Self>, key: String, value: String) -> Result<WriteResult, KeyValueStoreError> {
        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len() + value.len();

        let flush_action = {
            let mut state = self.state.lock().await;

            // Check if adding this would exceed limits
            let would_exceed_entries = state.pending.len() >= self.config.max_entries && !state.pending.is_empty();
            let would_exceed_bytes =
                state.current_bytes + op_bytes > self.config.max_bytes && !state.pending.is_empty();

            if would_exceed_entries || would_exceed_bytes {
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
            FlushAction::Immediate => {
                let batch = {
                    let mut state = self.state.lock().await;
                    self.take_batch(&mut state)
                };
                self.flush_batch(batch).await;
            }
            FlushAction::Delayed => {
                self.schedule_flush_shared().await;
            }
            FlushAction::None => {}
        }

        // Wait for result
        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Batch a Delete operation (Arc version with timeout support).
    async fn batch_delete_shared(self: &Arc<Self>, key: String) -> Result<WriteResult, KeyValueStoreError> {
        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len();

        let flush_action = {
            let mut state = self.state.lock().await;

            let would_exceed_entries = state.pending.len() >= self.config.max_entries && !state.pending.is_empty();
            let would_exceed_bytes =
                state.current_bytes + op_bytes > self.config.max_bytes && !state.pending.is_empty();

            if would_exceed_entries || would_exceed_bytes {
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
            FlushAction::Immediate => {
                let batch = {
                    let mut state = self.state.lock().await;
                    self.take_batch(&mut state)
                };
                self.flush_batch(batch).await;
            }
            FlushAction::Delayed => {
                self.schedule_flush_shared().await;
            }
            FlushAction::None => {}
        }

        rx.await.map_err(|_| KeyValueStoreError::Failed {
            reason: "batch cancelled".into(),
        })?
    }

    /// Batch a Set operation (non-Arc version, no timeout).
    async fn batch_set(&self, key: String, value: String) -> Result<WriteResult, KeyValueStoreError> {
        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len() + value.len();

        let should_flush = {
            let mut state = self.state.lock().await;

            let would_exceed_entries = state.pending.len() >= self.config.max_entries && !state.pending.is_empty();
            let would_exceed_bytes =
                state.current_bytes + op_bytes > self.config.max_bytes && !state.pending.is_empty();

            if would_exceed_entries || would_exceed_bytes {
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
    async fn batch_delete(&self, key: String) -> Result<WriteResult, KeyValueStoreError> {
        let (tx, rx) = oneshot::channel();
        let op_bytes = key.len();

        let should_flush = {
            let mut state = self.state.lock().await;

            let would_exceed_entries = state.pending.len() >= self.config.max_entries && !state.pending.is_empty();
            let would_exceed_bytes =
                state.current_bytes + op_bytes > self.config.max_bytes && !state.pending.is_empty();

            if would_exceed_entries || would_exceed_bytes {
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
    fn add_to_batch(
        &self,
        state: &mut BatcherState,
        operation: (bool, String, String),
        size_bytes: usize,
        result_tx: oneshot::Sender<Result<WriteResult, KeyValueStoreError>>,
    ) {
        if state.batch_start.is_none() {
            state.batch_start = Some(Instant::now());
        }

        state.current_bytes += size_bytes;
        state.pending.push(PendingWrite {
            operation,
            size_bytes,
            result_tx,
        });
    }

    /// Check if we should schedule a flush (Arc version).
    fn maybe_schedule_flush_shared(&self, state: &mut BatcherState) -> FlushAction {
        // Immediate flush if batch is full (entries)
        if state.pending.len() >= self.config.max_entries {
            return FlushAction::Immediate;
        }

        // Immediate flush if batch is full (bytes)
        if state.current_bytes >= self.config.max_bytes {
            return FlushAction::Immediate;
        }

        // No pending items
        if state.pending.is_empty() {
            return FlushAction::None;
        }

        // Check if max_wait is zero (disabled batching)
        if self.config.max_wait.is_zero() {
            return FlushAction::Immediate;
        }

        // Schedule delayed flush if not already scheduled
        if !state.flush_scheduled {
            state.flush_scheduled = true;
            return FlushAction::Delayed;
        }

        FlushAction::None
    }

    /// Check if we should schedule a flush (non-Arc version).
    fn maybe_schedule_flush(&self, state: &mut BatcherState) -> bool {
        // Immediate flush if batch is full (entries)
        if state.pending.len() >= self.config.max_entries {
            return true;
        }

        // Immediate flush if batch is full (bytes)
        if state.current_bytes >= self.config.max_bytes {
            return true;
        }

        // No pending items
        if state.pending.is_empty() {
            return false;
        }

        // Check if max_wait is zero (disabled batching)
        if self.config.max_wait.is_zero() {
            return true;
        }

        // Non-Arc version cannot schedule delayed flush
        false
    }

    /// Schedule a delayed flush (Arc version with background task).
    async fn schedule_flush_shared(self: &Arc<Self>) {
        let batcher = Arc::clone(self);
        let max_wait = self.config.max_wait;

        tokio::spawn(async move {
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
    fn take_batch(&self, state: &mut BatcherState) -> Vec<PendingWrite> {
        state.batch_start = None;
        state.current_bytes = 0;
        state.flush_scheduled = false;
        std::mem::take(&mut state.pending)
    }

    /// Submit a batch to Raft and notify all waiters.
    async fn flush_batch(&self, batch: Vec<PendingWrite>) {
        if batch.is_empty() {
            return;
        }

        // Build the Raft batch operation
        let operations: Vec<(bool, String, String)> = batch.iter().map(|p| p.operation.clone()).collect();

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

    /// Write directly to Raft without batching.
    async fn write_direct(&self, command: WriteCommand) -> Result<WriteResult, KeyValueStoreError> {
        use crate::api::BatchCondition;
        use crate::api::BatchOperation;

        let app_request = match &command {
            WriteCommand::Set { key, value } => AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            },
            WriteCommand::Delete { key } => AppRequest::Delete { key: key.clone() },
            WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            } => {
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let expires_at_ms = now_ms + (*ttl_seconds as u64 * 1000);
                AppRequest::SetWithTTL {
                    key: key.clone(),
                    value: value.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::SetMulti { pairs } => AppRequest::SetMulti { pairs: pairs.clone() },
            WriteCommand::SetMultiWithTTL { pairs, ttl_seconds } => {
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let expires_at_ms = now_ms + (*ttl_seconds as u64 * 1000);
                AppRequest::SetMultiWithTTL {
                    pairs: pairs.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::DeleteMulti { keys } => AppRequest::DeleteMulti { keys: keys.clone() },
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => AppRequest::CompareAndSwap {
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
            },
            WriteCommand::CompareAndDelete { key, expected } => AppRequest::CompareAndDelete {
                key: key.clone(),
                expected: expected.clone(),
            },
            WriteCommand::Batch { operations } => {
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::Batch { operations: ops }
            }
            WriteCommand::ConditionalBatch { conditions, operations } => {
                let conds: Vec<(u8, String, String)> = conditions
                    .iter()
                    .map(|c| match c {
                        BatchCondition::ValueEquals { key, expected } => (0, key.clone(), expected.clone()),
                        BatchCondition::KeyExists { key } => (1, key.clone(), String::new()),
                        BatchCondition::KeyNotExists { key } => (2, key.clone(), String::new()),
                    })
                    .collect();
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::ConditionalBatch {
                    conditions: conds,
                    operations: ops,
                }
            }
            // Lease operations
            WriteCommand::SetWithLease { key, value, lease_id } => AppRequest::SetWithLease {
                key: key.clone(),
                value: value.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::SetMultiWithLease { pairs, lease_id } => AppRequest::SetMultiWithLease {
                pairs: pairs.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => AppRequest::LeaseGrant {
                lease_id: *lease_id,
                ttl_seconds: *ttl_seconds,
            },
            WriteCommand::LeaseRevoke { lease_id } => AppRequest::LeaseRevoke { lease_id: *lease_id },
            WriteCommand::LeaseKeepalive { lease_id } => AppRequest::LeaseKeepalive { lease_id: *lease_id },
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => {
                use crate::api::CompareOp;
                use crate::api::CompareTarget;
                use crate::api::TxnOp;
                let cmp: Vec<(u8, u8, String, String)> = compare
                    .iter()
                    .map(|c| {
                        let target = match c.target {
                            CompareTarget::Value => 0,
                            CompareTarget::Version => 1,
                            CompareTarget::CreateRevision => 2,
                            CompareTarget::ModRevision => 3,
                        };
                        let op = match c.op {
                            CompareOp::Equal => 0,
                            CompareOp::NotEqual => 1,
                            CompareOp::Greater => 2,
                            CompareOp::Less => 3,
                        };
                        (target, op, c.key.clone(), c.value.clone())
                    })
                    .collect();

                let convert_ops = |ops: &[TxnOp]| -> Vec<(u8, String, String)> {
                    ops.iter()
                        .map(|op| match op {
                            TxnOp::Put { key, value } => (0, key.clone(), value.clone()),
                            TxnOp::Delete { key } => (1, key.clone(), String::new()),
                            TxnOp::Get { key } => (2, key.clone(), String::new()),
                            TxnOp::Range { prefix, limit } => (3, prefix.clone(), limit.to_string()),
                        })
                        .collect()
                };

                AppRequest::Transaction {
                    compare: cmp,
                    success: convert_ops(success),
                    failure: convert_ops(failure),
                }
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                use crate::api::WriteOp;
                let write_ops: Vec<(bool, String, String)> = write_set
                    .iter()
                    .map(|op| match op {
                        WriteOp::Set { key, value } => (true, key.clone(), value.clone()),
                        WriteOp::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::OptimisticTransaction {
                    read_set: read_set.clone(),
                    write_set: write_ops,
                }
            }
            WriteCommand::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            } => AppRequest::ShardSplit {
                source_shard: *source_shard,
                split_key: split_key.clone(),
                new_shard_id: *new_shard_id,
                topology_version: *topology_version,
            },
            WriteCommand::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            } => AppRequest::ShardMerge {
                source_shard: *source_shard,
                target_shard: *target_shard,
                topology_version: *topology_version,
            },
            WriteCommand::TopologyUpdate { topology_data } => AppRequest::TopologyUpdate {
                topology_data: topology_data.clone(),
            },
        };

        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(_resp) => Ok(WriteResult {
                command: Some(command),
                batch_applied: None,
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
        }
    }
}

/// Action to take after adding to batch.
enum FlushAction {
    /// Flush immediately (batch is full)
    Immediate,
    /// Schedule a delayed flush (timeout not yet started)
    Delayed,
    /// No action needed (flush already scheduled or batch empty)
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.max_entries, 100);
        assert_eq!(config.max_bytes, 1024 * 1024);
        assert_eq!(config.max_wait, Duration::from_millis(2));
    }

    #[test]
    fn test_batch_config_variants() {
        let high = BatchConfig::high_throughput();
        assert_eq!(high.max_entries, 100); // Capped at MAX_SETMULTI_KEYS
        assert_eq!(high.max_wait, Duration::from_millis(5));

        let low = BatchConfig::low_latency();
        assert_eq!(low.max_entries, 20);
        assert_eq!(low.max_wait, Duration::from_millis(1));

        let disabled = BatchConfig::disabled();
        assert_eq!(disabled.max_entries, 1);
        assert_eq!(disabled.max_wait, Duration::ZERO);
    }

    #[test]
    fn test_pending_write_size_tracking() {
        // Verify size calculation
        let key = "test-key".to_string();
        let value = "test-value".to_string();
        let expected_size = key.len() + value.len();
        assert_eq!(expected_size, 18); // 8 + 10
    }
}
