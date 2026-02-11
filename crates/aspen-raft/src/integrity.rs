//! Chain hashing for Raft log integrity verification.
//!
//! This module provides the background chain verification task that periodically
//! verifies the integrity of the Raft log. Pure cryptographic functions are in
//! [`crate::verified::integrity`].
//!
//! # Chain Hash Design
//!
//! Each entry's hash is computed as:
//! ```text
//! entry_hash = blake3(prev_hash || log_index || term || entry_data)
//! ```
//!
//! This creates an unbreakable chain where modifying any entry invalidates all
//! subsequent hashes, making tampering detectable.
//!
//! # Tiger Style Compliance
//!
//! - Fixed 32-byte Blake3 hashes (256-bit security)
//! - Constant-time comparison to prevent timing attacks
//! - Bounded verification batches (CHAIN_VERIFY_BATCH_SIZE)
//! - Fail-fast on corruption detection

use std::sync::Arc;
use std::time::Duration;

use crate::constants::CHAIN_VERIFY_BATCH_SIZE;
use crate::constants::CHAIN_VERIFY_INTERVAL_SECS;
use crate::storage::RedbLogStore;
// Re-export pure types and functions for backward compatibility
pub use crate::verified::ChainCorruption;
pub use crate::verified::ChainHash;
pub use crate::verified::ChainTipState;
pub use crate::verified::GENESIS_HASH;
pub use crate::verified::SnapshotIntegrity;
pub use crate::verified::compute_entry_hash;
pub use crate::verified::constant_time_compare;
pub use crate::verified::hash_from_hex;
pub use crate::verified::hash_to_hex;
pub use crate::verified::verify_entry_hash;

// ====================================================================================
// Background Chain Verifier
// ====================================================================================

/// Background chain verification task.
///
/// Periodically verifies the integrity of the Raft log by checking chain hashes.
/// On corruption detection, logs an error and panics (fail-fast).
///
/// # Tiger Style
///
/// - Fixed verification interval (CHAIN_VERIFY_INTERVAL_SECS)
/// - Bounded batch size (CHAIN_VERIFY_BATCH_SIZE)
/// - Fail-fast on corruption
pub struct ChainVerifier {
    log_store: Arc<RedbLogStore>,
    interval: Duration,
    batch_size: u32,
}

impl ChainVerifier {
    /// Create a new chain verifier.
    ///
    /// Uses default interval and batch size from constants.
    pub fn new(log_store: Arc<RedbLogStore>) -> Self {
        Self {
            log_store,
            interval: Duration::from_secs(CHAIN_VERIFY_INTERVAL_SECS),
            batch_size: CHAIN_VERIFY_BATCH_SIZE,
        }
    }

    /// Create a chain verifier with custom settings.
    ///
    /// Useful for testing with shorter intervals.
    pub fn with_settings(log_store: Arc<RedbLogStore>, interval: Duration, batch_size: u32) -> Self {
        Self {
            log_store,
            interval,
            batch_size,
        }
    }

    /// Spawn the background verification task.
    ///
    /// Returns a `JoinHandle` that can be used to abort the task.
    /// The task runs indefinitely until cancelled, or until a chain
    /// integrity violation is detected.
    ///
    /// On corruption detection, the task exits with an error logged at FATAL level.
    /// The caller should monitor the JoinHandle and take appropriate action
    /// (e.g., shutdown the node for manual inspection).
    pub fn spawn(self) -> tokio::task::JoinHandle<Result<(), crate::storage::StorageError>> {
        tokio::spawn(async move { self.run().await })
    }

    /// Run the verification loop.
    ///
    /// This is the main entry point called by `spawn()`.
    /// Returns `Err` on chain corruption detection (fail-fast behavior).
    async fn run(self) -> Result<(), crate::storage::StorageError> {
        let mut verification_index: u64 = 0;
        let mut total_verified: u64 = 0;
        let mut pass_count: u64 = 0;

        tracing::info!(
            interval_secs = CHAIN_VERIFY_INTERVAL_SECS,
            batch_size = self.batch_size,
            "chain integrity verifier started"
        );

        loop {
            tokio::time::sleep(self.interval).await;

            // Get verification range
            let range = match self.log_store.verification_range() {
                Ok(Some((first, last))) => (first, last),
                Ok(None) => {
                    tracing::debug!("no log entries to verify");
                    continue;
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to get verification range");
                    continue;
                }
            };

            let (first_index, last_index) = range;

            // Reset to beginning if we've passed the end or if log was truncated
            if verification_index > last_index || verification_index < first_index {
                verification_index = first_index;
                pass_count += 1;
                tracing::debug!(pass = pass_count, first_index, last_index, "starting new verification pass");
            }

            // Verify batch
            match self.log_store.verify_chain_batch(verification_index, self.batch_size) {
                Ok(verified) => {
                    if verified > 0 {
                        total_verified += verified;
                        verification_index += verified;
                        tracing::debug!(verified, total_verified, next_index = verification_index, "batch verified");
                    }
                }
                Err(e) => {
                    // FAIL-FAST: Chain integrity violation is unrecoverable.
                    // Return error to caller instead of panicking - let orchestration
                    // layer decide how to handle (shutdown, alert, manual inspection).
                    tracing::error!(
                        error = %e,
                        "CHAIN INTEGRITY VIOLATION DETECTED - FATAL - verification task exiting"
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Perform a one-time full chain verification.
    ///
    /// Verifies all entries in the log and returns the count of verified entries.
    /// Useful for startup validation.
    ///
    /// # Returns
    ///
    /// - `Ok(verified_count)` on success
    /// - `Err(StorageError)` on corruption or I/O error
    pub fn verify_full(&self) -> Result<u64, crate::storage::StorageError> {
        let range = match self.log_store.verification_range()? {
            Some((first, last)) => (first, last),
            None => return Ok(0), // Empty log
        };

        let (first_index, last_index) = range;
        let mut current_index = first_index;
        let mut total_verified: u64 = 0;

        tracing::info!(first_index, last_index, "performing full chain verification");

        while current_index <= last_index {
            let verified = self.log_store.verify_chain_batch(current_index, self.batch_size)?;
            if verified == 0 {
                break;
            }
            total_verified += verified;
            current_index += verified;

            // Progress logging for large logs
            if total_verified.is_multiple_of(10000) {
                tracing::info!(
                    verified = total_verified,
                    remaining = last_index.saturating_sub(current_index),
                    "verification progress"
                );
            }
        }

        tracing::info!(total_verified, "full chain verification complete");

        Ok(total_verified)
    }
}

// ====================================================================================
// Tests
// ====================================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use openraft::storage::IOFlushed;
    use openraft::storage::RaftLogStorage;
    use tempfile::TempDir;

    use super::*;
    use crate::storage::RedbLogStore;
    use crate::types::AppRequest;
    use crate::types::AppTypeConfig;
    use crate::types::NodeId;

    // =========================================================================
    // Test Helpers
    // =========================================================================

    /// Create a test log store with a temporary directory.
    fn create_test_store() -> (TempDir, Arc<RedbLogStore>) {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("test-log.redb");
        let store = RedbLogStore::new(&db_path).expect("failed to create store");
        (temp_dir, Arc::new(store))
    }

    /// Create test entries for appending to the log.
    fn create_test_entries(
        count: u64,
        term: u64,
        node_id: u64,
        start_index: u64,
    ) -> Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry> {
        use openraft::entry::RaftEntry;
        use openraft::testing::log_id;

        (0..count)
            .map(|i| {
                let index = start_index + i;
                let log_id = log_id::<AppTypeConfig>(term, NodeId::from(node_id), index);
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(log_id, AppRequest::Set {
                    key: format!("key-{}", index),
                    value: format!("value-{}", index),
                })
            })
            .collect()
    }

    // =========================================================================
    // ChainVerifier Struct Tests
    // =========================================================================

    #[test]
    fn test_chain_verifier_new_uses_defaults() {
        let (_temp_dir, store) = create_test_store();
        let verifier = ChainVerifier::new(store);

        assert_eq!(verifier.interval, Duration::from_secs(CHAIN_VERIFY_INTERVAL_SECS));
        assert_eq!(verifier.batch_size, CHAIN_VERIFY_BATCH_SIZE);
    }

    #[test]
    fn test_chain_verifier_with_settings() {
        let (_temp_dir, store) = create_test_store();
        let custom_interval = Duration::from_millis(100);
        let custom_batch_size = 50;

        let verifier = ChainVerifier::with_settings(store, custom_interval, custom_batch_size);

        assert_eq!(verifier.interval, custom_interval);
        assert_eq!(verifier.batch_size, custom_batch_size);
    }

    // =========================================================================
    // verify_full() Tests
    // =========================================================================

    #[test]
    fn test_verify_full_empty_log_returns_zero() {
        let (_temp_dir, store) = create_test_store();
        let verifier = ChainVerifier::new(store);

        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 0);
    }

    #[tokio::test]
    async fn test_verify_full_single_entry() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("verify-single.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        let entries = create_test_entries(1, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        let verifier = ChainVerifier::new(Arc::new(store));
        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 1);
    }

    #[tokio::test]
    async fn test_verify_full_multiple_entries() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("verify-multiple.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        let entries = create_test_entries(25, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        let verifier = ChainVerifier::new(Arc::new(store));
        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 25);
    }

    #[tokio::test]
    async fn test_verify_full_large_log() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("verify-large.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        // Create a log larger than the default batch size
        let entries = create_test_entries(150, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        let verifier = ChainVerifier::with_settings(Arc::new(store), Duration::from_secs(1), 50);
        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 150);
    }

    #[tokio::test]
    async fn test_verify_full_with_custom_batch_size() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("verify-batch.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        let entries = create_test_entries(100, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        // Use small batch size to verify batching works correctly
        let verifier = ChainVerifier::with_settings(Arc::new(store), Duration::from_secs(1), 10);
        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 100);
    }

    // =========================================================================
    // spawn() Lifecycle Tests
    // =========================================================================

    #[tokio::test]
    async fn test_spawn_returns_joinhandle() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("spawn-handle.redb");
        let store = RedbLogStore::new(&db_path).expect("failed to create store");

        // Use very short interval for test
        let verifier = ChainVerifier::with_settings(Arc::new(store), Duration::from_millis(10), 100);

        let handle = verifier.spawn();

        // Handle should not be finished immediately
        assert!(!handle.is_finished());

        // Abort the task
        handle.abort();

        // Wait for abort to complete
        let result = handle.await;
        assert!(result.is_err()); // JoinError from abort
    }

    #[tokio::test]
    async fn test_spawn_can_be_aborted() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("spawn-abort.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        // Add some entries so there's something to verify
        let entries = create_test_entries(10, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        let verifier = ChainVerifier::with_settings(Arc::new(store), Duration::from_millis(5), 100);

        let handle = verifier.spawn();

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Abort
        handle.abort();

        // Should complete after abort
        let result = handle.await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Batch Processing Tests
    // =========================================================================

    #[tokio::test]
    async fn test_verify_full_batch_size_one() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("verify-batch-one.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        let entries = create_test_entries(5, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        // Batch size of 1 should still work
        let verifier = ChainVerifier::with_settings(Arc::new(store), Duration::from_secs(1), 1);
        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 5);
    }

    #[tokio::test]
    async fn test_verify_full_batch_size_larger_than_log() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("verify-batch-large.redb");
        let mut store = RedbLogStore::new(&db_path).expect("failed to create store");

        let entries = create_test_entries(10, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.expect("append should succeed");

        // Batch size larger than log size
        let verifier = ChainVerifier::with_settings(Arc::new(store), Duration::from_secs(1), 1000);
        let verified = verifier.verify_full().expect("verify should succeed");
        assert_eq!(verified, 10);
    }
}
