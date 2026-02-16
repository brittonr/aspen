//! Distributed read-write lock with fencing tokens.
//!
//! Provides shared read access or exclusive write access with:
//! - Multiple concurrent readers OR a single exclusive writer
//! - Writer-preference fairness to prevent writer starvation
//! - Monotonically increasing fencing tokens for split-brain prevention
//! - TTL-based automatic expiration for crash recovery
//!
//! ## Fairness Policy
//!
//! This implementation uses writer-preference:
//! - When a writer is waiting, new readers are blocked
//! - This prevents writer starvation in read-heavy workloads
//! - Readers already holding the lock can continue until they release
//!
//! ## Lock State
//!
//! The lock state is stored as JSON in the key-value store:
//! ```json
//! {
//!   "mode": "Read" | "Write" | "Free",
//!   "writer": {"holder_id": "...", "fencing_token": 42, "deadline_ms": ...},
//!   "readers": [{"holder_id": "...", "deadline_ms": ...}, ...],
//!   "pending_writers": 0,  // Count of waiting writers
//!   "fencing_token": 42    // Global token, incremented on write acquisition
//! }
//! ```

mod acquisition;
mod downgrade;
mod fairness;
mod release;
pub mod types;

use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_traits::KeyValueStore;
pub use types::RWLockMode;
pub use types::RWLockState;
pub use types::ReaderEntry;
pub use types::WriterEntry;

use crate::verified;

/// Manager for distributed read-write lock operations.
pub struct RWLockManager<S: KeyValueStore + ?Sized> {
    pub(crate) store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> RWLockManager<S> {
    /// Create a new RWLock manager.
    pub fn new(store: Arc<S>) -> Self {
        debug_assert!(Arc::strong_count(&store) >= 1, "RWLOCK: store Arc must have at least 1 strong reference");
        Self { store }
    }

    /// Get lock status.
    ///
    /// Returns (mode, reader_count, writer_holder, fencing_token).
    pub async fn status(&self, name: &str) -> Result<(String, u32, Option<String>, u64)> {
        // Tiger Style: name must not be empty
        debug_assert!(!name.is_empty(), "RWLOCK: name must not be empty for status");

        let key = verified::rwlock_key(name);

        match self.read_state(&key).await? {
            Some(mut state) => {
                state.cleanup_expired_readers();
                state.cleanup_expired_writer();
                let reader_count = state.active_reader_count();
                let writer_holder = state.writer.as_ref().map(|w| w.holder_id.clone());
                Ok((state.mode.as_str().to_string(), reader_count, writer_holder, state.fencing_token))
            }
            None => Ok(("free".to_string(), 0, None, 0)),
        }
    }

    /// Read lock state from the store.
    pub(crate) async fn read_state(&self, key: &str) -> Result<Option<RWLockState>> {
        debug_assert!(!key.is_empty(), "RWLOCK: key must not be empty for read_state");

        match self.store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    let state: RWLockState = serde_json::from_str(&value)
                        .with_context(|| format!("failed to parse rwlock state for key '{}'", key))?;
                    Ok(Some(state))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("rwlock read failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_rwlock_read_acquire_release() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire read lock
        let (token, deadline, count) = manager.try_acquire_read("test", "reader1", 60000).await.unwrap().unwrap();

        assert_eq!(token, 0); // No writes yet
        assert!(deadline > 0);
        assert_eq!(count, 1);

        // Release
        manager.release_read("test", "reader1").await.unwrap();

        // Check status
        let (mode, reader_count, writer, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "free");
        assert_eq!(reader_count, 0);
        assert!(writer.is_none());
    }

    #[tokio::test]
    async fn test_rwlock_write_acquire_release() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire write lock
        let (token, deadline) = manager.try_acquire_write("test", "writer1", 60000).await.unwrap().unwrap();

        assert_eq!(token, 1); // First write
        assert!(deadline > 0);

        // Release
        manager.release_write("test", "writer1", token).await.unwrap();

        // Check status
        let (mode, _, _, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "free");
    }

    #[tokio::test]
    async fn test_rwlock_multiple_readers() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // First reader
        let (_, _, count1) = manager.try_acquire_read("test", "reader1", 60000).await.unwrap().unwrap();
        assert_eq!(count1, 1);

        // Second reader (should succeed)
        let (_, _, count2) = manager.try_acquire_read("test", "reader2", 60000).await.unwrap().unwrap();
        assert_eq!(count2, 2);

        // Third reader
        let (_, _, count3) = manager.try_acquire_read("test", "reader3", 60000).await.unwrap().unwrap();
        assert_eq!(count3, 3);

        // Check status
        let (mode, count, _, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "read");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_rwlock_write_blocks_readers() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire write lock
        let (token, _) = manager.try_acquire_write("test", "writer1", 60000).await.unwrap().unwrap();

        // Try to acquire read (should fail)
        let result = manager.try_acquire_read("test", "reader1", 60000).await.unwrap();
        assert!(result.is_none());

        // Release write lock
        manager.release_write("test", "writer1", token).await.unwrap();

        // Now read should succeed
        let result = manager.try_acquire_read("test", "reader1", 60000).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_rwlock_readers_block_write() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire read locks
        manager.try_acquire_read("test", "reader1", 60000).await.unwrap().unwrap();
        manager.try_acquire_read("test", "reader2", 60000).await.unwrap().unwrap();

        // Try to acquire write (should fail)
        let result = manager.try_acquire_write("test", "writer1", 60000).await.unwrap();
        assert!(result.is_none());

        // Release all readers
        manager.release_read("test", "reader1").await.unwrap();
        manager.release_read("test", "reader2").await.unwrap();

        // Now write should succeed
        let result = manager.try_acquire_write("test", "writer1", 60000).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_rwlock_downgrade() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire write lock
        let (token, _) = manager.try_acquire_write("test", "holder1", 60000).await.unwrap().unwrap();

        // Downgrade to read
        let (new_token, _, count) = manager.downgrade("test", "holder1", token, 60000).await.unwrap();

        assert_eq!(new_token, token); // Token preserved
        assert_eq!(count, 1);

        // Check status
        let (mode, reader_count, writer, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "read");
        assert_eq!(reader_count, 1);
        assert!(writer.is_none());
    }

    #[tokio::test]
    async fn test_rwlock_fencing_token_increments() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // First write
        let (token1, _) = manager.try_acquire_write("test", "writer1", 60000).await.unwrap().unwrap();
        manager.release_write("test", "writer1", token1).await.unwrap();

        // Second write
        let (token2, _) = manager.try_acquire_write("test", "writer2", 60000).await.unwrap().unwrap();

        assert!(token2 > token1, "fencing token should increment");
    }

    #[tokio::test]
    async fn test_rwlock_reader_limit_enforced() {
        use aspen_constants::coordination::MAX_RWLOCK_READERS;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire MAX_RWLOCK_READERS readers
        for i in 0..MAX_RWLOCK_READERS {
            let result = manager.try_acquire_read("test", &format!("reader{}", i), 60000).await.unwrap();
            assert!(result.is_some(), "reader {} should acquire lock", i);
        }

        // One more reader should fail with TooManyReaders error
        let result = manager.try_acquire_read("test", "one_too_many", 60000).await;
        assert!(result.is_err(), "should fail when exceeding reader limit");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too many readers"), "error should mention 'too many readers', got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_rwlock_concurrent_readers() {
        use tokio::task::JoinSet;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = Arc::new(RWLockManager::new(store));

        let mut tasks = JoinSet::new();
        let num_readers = 10;

        // Spawn concurrent readers
        for i in 0..num_readers {
            let mgr = Arc::clone(&manager);
            let holder_id = format!("reader{}", i);
            tasks.spawn(async move { mgr.try_acquire_read("concurrent_test", &holder_id, 60000).await });
        }

        // Wait for all to complete
        let mut success_count = 0;
        while let Some(result) = tasks.join_next().await {
            let inner = result.unwrap();
            if inner.is_ok() && inner.unwrap().is_some() {
                success_count += 1;
            }
        }

        assert_eq!(success_count, num_readers, "all readers should acquire lock");

        // Verify status
        let (mode, count, _, _) = manager.status("concurrent_test").await.unwrap();
        assert_eq!(mode, "read");
        assert_eq!(count, num_readers as u32);
    }
}
