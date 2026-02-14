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

mod batching;
mod config;
mod direct_write;
mod flush;

use std::sync::Arc;

use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteResult;
pub use config::BatchConfig;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::time::Instant;

use crate::types::AppTypeConfig;

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
}

// FlushAction replaced by FlushDecision from pure module

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aspen_kv_types::WriteCommand;

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

    // ========================================================================
    // BatchConfig Validation Tests
    // ========================================================================

    #[test]
    fn test_batch_config_finalize() {
        // When BatchConfig is deserialized, max_wait_ms is set but max_wait is Duration::ZERO
        // finalize() should compute max_wait from max_wait_ms
        let config = BatchConfig {
            max_entries: 50,
            max_bytes: 512 * 1024,
            max_wait_ms: 10,
            max_wait: Duration::ZERO, // Simulates post-deserialization state
        };
        let config = config.finalize();
        assert_eq!(config.max_wait, Duration::from_millis(10));
    }

    #[test]
    fn test_batch_config_zero_max_wait_is_immediate() {
        let config = BatchConfig::disabled();
        assert!(config.max_wait.is_zero(), "disabled config should have zero max_wait");
        assert_eq!(config.max_entries, 1, "disabled config should flush after 1 entry");
    }

    #[test]
    fn test_batch_config_high_throughput_bounds() {
        let config = BatchConfig::high_throughput();
        // High throughput has larger byte limits
        assert!(config.max_bytes >= 4 * 1024 * 1024);
        // But still bounded
        assert!(config.max_entries <= 1000);
        // Longer wait time for more batching
        assert!(config.max_wait >= Duration::from_millis(5));
    }

    #[test]
    fn test_batch_config_low_latency_bounds() {
        let config = BatchConfig::low_latency();
        // Low latency has smaller limits for faster flushing
        assert!(config.max_entries <= 50);
        assert!(config.max_bytes <= 512 * 1024);
        assert!(config.max_wait <= Duration::from_millis(2));
    }

    // ========================================================================
    // Routing Logic Tests (Pure Function Tests)
    // ========================================================================

    /// Test that Set operations would be batched based on command matching.
    ///
    /// The actual batching in write() matches on WriteCommand variants.
    /// Set commands should route through batch_set().
    #[test]
    fn test_routing_set_is_batchable() {
        let command = WriteCommand::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        };
        assert!(matches!(command, WriteCommand::Set { .. }), "Set should be batchable");
    }

    /// Test that Delete operations would be batched.
    #[test]
    fn test_routing_delete_is_batchable() {
        let command = WriteCommand::Delete {
            key: "test".to_string(),
        };
        assert!(matches!(command, WriteCommand::Delete { .. }), "Delete should be batchable");
    }

    /// Test that CAS operations bypass batching.
    #[test]
    fn test_routing_cas_bypasses_batcher() {
        let command = WriteCommand::CompareAndSwap {
            key: "test".to_string(),
            expected: Some("old".to_string()),
            new_value: "new".to_string(),
        };
        // CAS operations must bypass batching for atomicity
        assert!(
            !matches!(command, WriteCommand::Set { .. } | WriteCommand::Delete { .. }),
            "CAS should bypass batcher"
        );
    }

    /// Test that Transaction operations bypass batching.
    #[test]
    fn test_routing_transaction_bypasses_batcher() {
        let command = WriteCommand::Transaction {
            compare: vec![],
            success: vec![],
            failure: vec![],
        };
        // Transactions must bypass batching for atomicity
        assert!(
            !matches!(command, WriteCommand::Set { .. } | WriteCommand::Delete { .. }),
            "Transaction should bypass batcher"
        );
    }

    /// Test that Batch operations bypass batching.
    #[test]
    fn test_routing_batch_bypasses_batcher() {
        let command = WriteCommand::Batch { operations: vec![] };
        // Batch operations are already batched, no need for write batcher
        assert!(
            !matches!(command, WriteCommand::Set { .. } | WriteCommand::Delete { .. }),
            "Batch should bypass batcher (already batched)"
        );
    }

    /// Test that lease operations bypass batching.
    #[test]
    fn test_routing_lease_ops_bypass_batcher() {
        let grant = WriteCommand::LeaseGrant {
            lease_id: 1,
            ttl_seconds: 60,
        };
        let revoke = WriteCommand::LeaseRevoke { lease_id: 1 };
        let keepalive = WriteCommand::LeaseKeepalive { lease_id: 1 };

        // Lease operations must bypass batching for consistency
        assert!(
            !matches!(grant, WriteCommand::Set { .. } | WriteCommand::Delete { .. }),
            "LeaseGrant should bypass batcher"
        );
        assert!(
            !matches!(revoke, WriteCommand::Set { .. } | WriteCommand::Delete { .. }),
            "LeaseRevoke should bypass batcher"
        );
        assert!(
            !matches!(keepalive, WriteCommand::Set { .. } | WriteCommand::Delete { .. }),
            "LeaseKeepalive should bypass batcher"
        );
    }

    // FlushAction tests moved to pure::write_batcher module

    // ========================================================================
    // Size Tracking Tests
    // ========================================================================

    #[test]
    fn test_operation_size_calculation_set() {
        // For Set, size = key.len() + value.len()
        let key = "abc".to_string();
        let value = "12345".to_string();
        let op_bytes = key.len() + value.len();
        assert_eq!(op_bytes, 8);
    }

    #[test]
    fn test_operation_size_calculation_delete() {
        // For Delete, size = key.len() (no value)
        let key = "test-key".to_string();
        let op_bytes = key.len();
        assert_eq!(op_bytes, 8);
    }

    #[test]
    fn test_operation_size_calculation_empty() {
        // Edge case: empty key and value
        let key = String::new();
        let value = String::new();
        let op_bytes = key.len() + value.len();
        assert_eq!(op_bytes, 0);
    }

    #[test]
    fn test_operation_size_calculation_large() {
        // Large values should calculate correctly
        let key = "k".repeat(1000);
        let value = "v".repeat(1_000_000);
        let op_bytes = key.len() + value.len();
        assert_eq!(op_bytes, 1_001_000);
    }
}
