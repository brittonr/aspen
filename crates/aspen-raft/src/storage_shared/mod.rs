//! Single-fsync Redb storage implementation.
//!
//! This module provides a unified storage backend that implements both `RaftLogStorage`
//! and `RaftStateMachine` traits on a single struct, enabling single-fsync writes.
//!
//! # Architecture
//!
//! The key insight is that OpenRaft calls `append()` and `apply()` asynchronously
//! via separate tasks. Simply sharing a database handle does NOT achieve single-fsync:
//!
//! ```text
//! // Two separate transactions = two fsyncs (WRONG)
//! RaftLogStorage::append() -> txn.commit() -> fsync #1
//! RaftStateMachine::apply() -> txn.commit() -> fsync #2
//! ```
//!
//! Instead, we bundle state mutations INTO the log append:
//!
//! ```text
//! // Single transaction = single fsync (CORRECT)
//! RaftLogStorage::append() {
//!     txn.insert(log_entry);
//!     txn.apply(state_mutation);  // Apply state HERE
//!     txn.commit();  // Single fsync
//! }
//! RaftStateMachine::apply() {
//!     // No-op - already applied during append
//! }
//! ```
//!
//! # Why This Is Safe
//!
//! 1. Raft's correctness requires only that committed entries survive crashes
//! 2. Either both log entry and state mutation are durable, or neither is
//! 3. Crash before commit() -> clean rollback, Raft re-proposes
//! 4. Crash after commit() -> fully durable, no replay needed
//!
//! # Performance
//!
//! - Current (SQLite): ~9ms per write (2 fsyncs: redb log + SQLite state)
//! - Target (SharedRedb): ~2-3ms per write (1 fsync)
//!
//! # Tiger Style
//!
//! - Fixed limits on batch sizes (MAX_BATCH_SIZE, MAX_SETMULTI_KEYS)
//! - Explicit error types with actionable context
//! - Chain hashing for integrity verification
//! - Bounded operations prevent unbounded memory use
//!
//! # Module Organization
//!
//! - `types` - Storage types (LeaseEntry, StoredSnapshot, SnapshotEvent) and table definitions
//! - `error` - Error types (SharedStorageError)
//! - `initialization` - Constructors and database setup
//! - `meta` - Raft and state machine metadata operations, accessors, SQL executor
//! - `kv` - KV operations (get, scan, TTL cleanup)
//! - `lease` - Lease operations (query, cleanup, management)
//! - `chain` - Chain integrity (hash verification, integrity checking)
//! - `state_machine` - State machine apply helpers (`apply_*_in_txn` methods)
//! - `log_storage` - `RaftLogStorage` trait implementation
//! - `sm_trait` - `RaftStateMachine` trait implementation
//! - `snapshot` - `SharedRedbSnapshotBuilder` and snapshot-related code
//! - `index` - `IndexQueryExecutor` implementation

mod chain;
mod error;
mod index;
mod initialization;
mod kv;
mod lease;
mod log_storage;
mod meta;
mod sm_trait;
mod snapshot;
mod state_machine;
mod types;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

#[cfg(feature = "coordination")]
use aspen_coordination::now_unix_ms;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::TxnOpResult;
use redb::Database;
use tokio::sync::broadcast;

#[cfg(not(feature = "coordination"))]
#[inline]
fn now_unix_ms() -> u64 {
    aspen_time::current_time_ms()
}

use aspen_core::ensure_disk_space_available;
use aspen_core::layer::IndexQueryExecutor;
use aspen_core::layer::IndexRegistry;
use aspen_core::layer::IndexResult;
use aspen_core::layer::IndexScanResult;
use aspen_core::layer::IndexableEntry;
use aspen_core::layer::Tuple;
use aspen_core::layer::extract_primary_key_from_tuple;
// Re-export types from submodules for public API
pub use error::*;
use openraft::alias::LogIdOf;
pub use snapshot::SharedRedbSnapshotBuilder;
pub use types::*;

use crate::constants::MAX_BATCH_SIZE;
use crate::constants::MAX_SETMULTI_KEYS;
use crate::constants::MAX_SNAPSHOT_ENTRIES;
use crate::integrity::ChainHash;
use crate::integrity::ChainTipState;
use crate::integrity::SnapshotIntegrity;
use crate::integrity::compute_entry_hash;
use crate::log_subscriber::LogEntryPayload;
// Ghost code imports - compile away when verus feature is disabled
use crate::spec::verus_shim::*;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;
use crate::verified::kv::check_cas_condition;
use crate::verified::kv::compute_kv_versions;
use crate::verified::kv::compute_lease_refresh;
use crate::verified::kv::create_lease_entry;

// ====================================================================================
// SharedRedbStorage Implementation
// ====================================================================================

/// Unified Raft log and state machine storage using single-fsync Redb.
///
/// This struct implements both `RaftLogStorage` and `RaftStateMachine` traits,
/// bundling state mutations into log appends for single-fsync durability.
///
/// # Architecture
///
/// ```text
/// append() {
///     1. Insert log entry into RAFT_LOG_TABLE
///     2. Compute chain hash
///     3. Parse entry payload
///     4. Apply state mutation to SM_KV_TABLE
///     5. Update SM_META_TABLE.last_applied
///     6. Handle membership changes
///     7. Single commit() -> single fsync
/// }
///
/// apply() {
///     // No-op - state already applied during append
/// }
/// ```
#[derive(Clone)]
pub struct SharedRedbStorage {
    /// The underlying Redb database.
    db: Arc<Database>,
    /// Path to the database file.
    path: PathBuf,
    /// Cached chain tip state for efficient appends.
    chain_tip: Arc<StdRwLock<ChainTipState>>,
    /// Optional broadcast sender for log entry notifications.
    /// Broadcasts committed KV operations for DocsExporter and other subscribers.
    log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
    /// Optional broadcast sender for snapshot event notifications.
    /// Broadcasts snapshot created/installed events for hook integration.
    snapshot_broadcast: Option<broadcast::Sender<SnapshotEvent>>,
    /// Pending responses computed during append() to be sent in apply().
    /// Key is log index, value is the computed AppResponse.
    pending_responses: Arc<StdRwLock<BTreeMap<u64, AppResponse>>>,
    /// Hybrid Logical Clock for deterministic timestamp ordering.
    hlc: Arc<aspen_core::hlc::HLC>,
    /// Secondary index registry for maintaining indexes.
    index_registry: Arc<IndexRegistry>,
    /// Last applied log ID confirmed by openraft's apply() callback.
    /// This may lag behind the eagerly-applied value in SM_META_TABLE
    /// because entries are applied during append() but only confirmed
    /// during apply(). build_snapshot() must use this value to avoid
    /// the TOCTOU race with openraft's apply_progress tracking.
    confirmed_last_applied: Arc<StdRwLock<Option<LogIdOf<AppTypeConfig>>>>,
}

impl std::fmt::Debug for SharedRedbStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedRedbStorage")
            .field("path", &self.path)
            .field("chain_tip", &self.chain_tip)
            .finish_non_exhaustive()
    }
}

// Implementation methods are organized in submodules:
// - initialization: constructors and database setup
// - meta: metadata operations, accessors, and SQL executor

#[cfg(test)]
mod tests {
    use openraft::alias::LogIdOf;
    use tempfile::TempDir;

    use super::*;

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn create_test_storage(temp_dir: &TempDir) -> SharedRedbStorage {
        let db_path = temp_dir.path().join("test.redb");
        SharedRedbStorage::new(&db_path, "test-node-1").unwrap()
    }

    fn insert_kv_entry(storage: &SharedRedbStorage, key: &str, value: &str, version: i64) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).unwrap();
            let entry = KvEntry {
                value: value.to_string(),
                version,
                create_revision: version,
                mod_revision: version,
                expires_at_ms: None,
                lease_id: None,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    fn insert_kv_entry_with_ttl(storage: &SharedRedbStorage, key: &str, value: &str, version: i64, expires_at_ms: u64) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).unwrap();
            let entry = KvEntry {
                value: value.to_string(),
                version,
                create_revision: version,
                mod_revision: version,
                expires_at_ms: Some(expires_at_ms),
                lease_id: None,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    fn insert_lease_entry(storage: &SharedRedbStorage, lease_id: u64, ttl_seconds: u32, expires_at_ms: u64) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).unwrap();
            let entry = LeaseEntry {
                ttl_seconds,
                expires_at_ms,
                keys: Vec::new(),
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            leases_table.insert(lease_id, entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    fn insert_lease_entry_with_keys(
        storage: &SharedRedbStorage,
        lease_id: u64,
        ttl_seconds: u32,
        expires_at_ms: u64,
        keys: Vec<String>,
    ) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).unwrap();
            let entry = LeaseEntry {
                ttl_seconds,
                expires_at_ms,
                keys,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            leases_table.insert(lease_id, entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    // =========================================================================
    // Basic Storage Tests
    // =========================================================================

    #[tokio::test]
    async fn test_shared_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Test basic KV operations
        assert!(storage.get("test_key").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_shared_storage_set_get() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        insert_kv_entry(&storage, "test_key", "test_value", 1);

        // Verify we can read it
        let entry = storage.get("test_key").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, "test_value");
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_with_revision() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        insert_kv_entry(&storage, "test_key", "test_value", 5);

        let result = storage.get_with_revision("test_key").unwrap();
        assert!(result.is_some());
        let kv = result.unwrap();
        assert_eq!(kv.key, "test_key");
        assert_eq!(kv.value, "test_value");
        assert_eq!(kv.version, 5);
        assert_eq!(kv.create_revision, 5);
        assert_eq!(kv.mod_revision, 5);
    }

    #[tokio::test]
    async fn test_get_with_revision_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get_with_revision("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_storage_path() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let storage = SharedRedbStorage::new(&db_path, "test-node-1").unwrap();

        assert_eq!(storage.path(), db_path.as_path());
    }

    #[tokio::test]
    async fn test_storage_db_handle() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Verify we can get the database handle
        let db = storage.db();
        assert!(Arc::strong_count(&db) >= 2); // storage + returned handle
    }

    #[tokio::test]
    async fn test_storage_debug_format() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let debug_str = format!("{:?}", storage);
        assert!(debug_str.contains("SharedRedbStorage"));
        assert!(debug_str.contains("path"));
    }

    // =========================================================================
    // TTL Expiration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_expired_key_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert a key that has already expired
        let expired_time = now_unix_ms() - 1000; // 1 second ago
        insert_kv_entry_with_ttl(&storage, "expired_key", "value", 1, expired_time);

        // Should return None for expired key
        let result = storage.get("expired_key").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_non_expired_key_returns_value() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert a key that expires in the future
        let future_time = now_unix_ms() + 60000; // 1 minute from now
        insert_kv_entry_with_ttl(&storage, "valid_key", "value", 1, future_time);

        // Should return the value
        let result = storage.get("valid_key").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "value");
    }

    #[tokio::test]
    async fn test_delete_expired_keys_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired keys
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);
        insert_kv_entry_with_ttl(&storage, "expired2", "value2", 2, expired_time);

        // Insert non-expired key
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid", "value3", 3, future_time);

        // Delete expired keys
        let deleted = storage.delete_expired_keys(100).unwrap();
        assert_eq!(deleted, 2);

        // Verify non-expired key still exists
        assert!(storage.get("valid").unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_keys_respects_batch_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert 10 expired keys
        let expired_time = now_unix_ms() - 1000;
        for i in 0..10 {
            let key = format!("expired_{}", i);
            insert_kv_entry_with_ttl(&storage, &key, "value", i as i64, expired_time);
        }

        // Delete with batch limit of 3
        let deleted = storage.delete_expired_keys(3).unwrap();
        assert_eq!(deleted, 3);

        // There should still be expired keys
        let remaining = storage.count_expired_keys().unwrap();
        assert_eq!(remaining, 7);
    }

    #[tokio::test]
    async fn test_delete_expired_keys_no_expired() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert only non-expired keys
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid1", "value1", 1, future_time);
        insert_kv_entry_with_ttl(&storage, "valid2", "value2", 2, future_time);

        // Delete expired keys - should delete none
        let deleted = storage.delete_expired_keys(100).unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn test_count_expired_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired keys
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);
        insert_kv_entry_with_ttl(&storage, "expired2", "value2", 2, expired_time);

        // Insert non-expired key
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid", "value3", 3, future_time);

        let count = storage.count_expired_keys().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_keys_with_ttl() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired key (not counted as "with TTL" because it's expired)
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);

        // Insert non-expired keys with TTL
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid1", "value2", 2, future_time);
        insert_kv_entry_with_ttl(&storage, "valid2", "value3", 3, future_time);

        // Insert key without TTL
        insert_kv_entry(&storage, "no_ttl", "value4", 4);

        let count = storage.count_keys_with_ttl().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_expired_keys_with_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired keys
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);
        insert_kv_entry_with_ttl(&storage, "expired2", "value2", 2, expired_time);

        let expired_keys = storage.get_expired_keys_with_metadata(10).unwrap();
        assert_eq!(expired_keys.len(), 2);

        // Verify keys are present
        let key_names: Vec<&str> = expired_keys.iter().map(|(k, _)| k.as_str()).collect();
        assert!(key_names.contains(&"expired1"));
        assert!(key_names.contains(&"expired2"));
    }

    #[tokio::test]
    async fn test_get_expired_keys_with_metadata_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert 10 expired keys
        let expired_time = now_unix_ms() - 1000;
        for i in 0..10 {
            let key = format!("expired_{}", i);
            insert_kv_entry_with_ttl(&storage, &key, "value", i as i64, expired_time);
        }

        let expired_keys = storage.get_expired_keys_with_metadata(3).unwrap();
        assert_eq!(expired_keys.len(), 3);
    }

    // =========================================================================
    // Scan Tests
    // =========================================================================

    #[tokio::test]
    async fn test_shared_storage_scan() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert multiple values
        for i in 0..10 {
            let key = format!("prefix/{}", i);
            insert_kv_entry(&storage, &key, &format!("value_{}", i), i as i64);
        }

        // Scan with prefix
        let results = storage.scan("prefix/", None, Some(5)).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_scan_with_after_key() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert keys
        for i in 0..10 {
            let key = format!("key/{}", i);
            insert_kv_entry(&storage, &key, &format!("value_{}", i), i as i64);
        }

        // Scan starting after key/3
        let results = storage.scan("key/", Some("key/3"), Some(10)).unwrap();

        // Should not include key/0, key/1, key/2, key/3
        for result in &results {
            assert!(result.key.as_str() > "key/3");
        }
    }

    #[tokio::test]
    async fn test_scan_empty_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert keys with different prefixes
        insert_kv_entry(&storage, "aaa", "value1", 1);
        insert_kv_entry(&storage, "bbb", "value2", 2);
        insert_kv_entry(&storage, "ccc", "value3", 3);

        // Scan with empty prefix - should return all keys
        let results = storage.scan("", None, Some(10)).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_scan_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        insert_kv_entry(&storage, "other/key", "value", 1);

        // Scan with non-matching prefix
        let results = storage.scan("prefix/", None, Some(10)).unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_scan_excludes_expired_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired key
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "prefix/expired", "value1", 1, expired_time);

        // Insert valid key
        insert_kv_entry(&storage, "prefix/valid", "value2", 2);

        let results = storage.scan("prefix/", None, Some(10)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "prefix/valid");
    }

    #[tokio::test]
    async fn test_scan_respects_max_batch_size() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert many keys
        for i in 0..100 {
            let key = format!("key/{:03}", i);
            insert_kv_entry(&storage, &key, &format!("value_{}", i), i as i64);
        }

        // Scan with limit much larger than MAX_BATCH_SIZE
        let results = storage.scan("key/", None, Some(10000)).unwrap();

        // Should be capped at MAX_BATCH_SIZE
        assert!(results.len() <= MAX_BATCH_SIZE as usize);
    }

    // =========================================================================
    // Lease Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_lease_active() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert active lease (expires in 60 seconds)
        let expires_at = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1001, 60, expires_at);

        let result = storage.get_lease(1001).unwrap();
        assert!(result.is_some());
        let (granted_ttl, remaining_ttl) = result.unwrap();
        assert_eq!(granted_ttl, 60);
        assert!(remaining_ttl > 0);
        assert!(remaining_ttl <= 60);
    }

    #[tokio::test]
    async fn test_get_lease_expired() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired lease
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);

        let result = storage.get_lease(1001).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_lease_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get_lease(9999).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_lease_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let expires_at = now_unix_ms() + 60000;
        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        insert_lease_entry_with_keys(&storage, 1001, 60, expires_at, keys.clone());

        let result = storage.get_lease_keys(1001).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"key1".to_string()));
        assert!(result.contains(&"key2".to_string()));
        assert!(result.contains(&"key3".to_string()));
    }

    #[tokio::test]
    async fn test_get_lease_keys_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get_lease_keys(9999).unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_leases_active_only() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert active leases
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1001, 60, future_time);
        insert_lease_entry(&storage, 1002, 120, future_time);

        // Insert expired lease
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1003, 60, expired_time);

        let leases = storage.list_leases().unwrap();
        assert_eq!(leases.len(), 2);

        let lease_ids: Vec<u64> = leases.iter().map(|(id, _, _)| *id).collect();
        assert!(lease_ids.contains(&1001));
        assert!(lease_ids.contains(&1002));
        assert!(!lease_ids.contains(&1003));
    }

    #[tokio::test]
    async fn test_delete_expired_leases_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired leases
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);
        insert_lease_entry(&storage, 1002, 60, expired_time);

        // Insert active lease
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1003, 60, future_time);

        let deleted = storage.delete_expired_leases(100).unwrap();
        assert_eq!(deleted, 2);

        // Active lease should still exist
        assert!(storage.get_lease(1003).unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_leases_with_attached_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert keys
        insert_kv_entry(&storage, "attached_key1", "value1", 1);
        insert_kv_entry(&storage, "attached_key2", "value2", 2);

        // Insert expired lease with attached keys
        let expired_time = now_unix_ms() - 1000;
        let keys = vec!["attached_key1".to_string(), "attached_key2".to_string()];
        insert_lease_entry_with_keys(&storage, 1001, 60, expired_time, keys);

        // Delete expired leases
        let deleted = storage.delete_expired_leases(100).unwrap();
        assert_eq!(deleted, 1);

        // Attached keys should be deleted
        assert!(storage.get("attached_key1").unwrap().is_none());
        assert!(storage.get("attached_key2").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_expired_leases_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert 10 expired leases
        let expired_time = now_unix_ms() - 1000;
        for i in 0..10 {
            insert_lease_entry(&storage, 1000 + i, 60, expired_time);
        }

        // Delete with limit of 3
        let deleted = storage.delete_expired_leases(3).unwrap();
        assert_eq!(deleted, 3);

        let remaining = storage.count_expired_leases().unwrap();
        assert_eq!(remaining, 7);
    }

    #[tokio::test]
    async fn test_count_expired_leases() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired leases
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);
        insert_lease_entry(&storage, 1002, 60, expired_time);

        // Insert active lease
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1003, 60, future_time);

        let count = storage.count_expired_leases().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_active_leases() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired lease
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);

        // Insert active leases
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1002, 60, future_time);
        insert_lease_entry(&storage, 1003, 120, future_time);

        let count = storage.count_active_leases().unwrap();
        assert_eq!(count, 2);
    }

    // =========================================================================
    // Chain Hash Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chain_tip_for_verification() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // New storage should have default chain tip
        let (index, hash) = storage.chain_tip_for_verification().unwrap();
        assert_eq!(index, 0);
        assert_eq!(hash, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        // Create storage and insert data
        {
            let storage = SharedRedbStorage::new(&db_path, "test-node-1").unwrap();
            insert_kv_entry(&storage, "persistent_key", "persistent_value", 1);
        }

        // Reopen storage and verify data
        {
            let storage = SharedRedbStorage::new(&db_path, "test-node-1").unwrap();
            let entry = storage.get("persistent_key").unwrap();
            assert!(entry.is_some());
            assert_eq!(entry.unwrap().value, "persistent_value");
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[tokio::test]
    async fn test_error_display() {
        let err = SharedStorageError::BatchTooLarge { size: 2000, max: 1000 };
        let display = err.to_string();
        assert!(display.contains("2000"));
        assert!(display.contains("1000"));
    }

    #[tokio::test]
    async fn test_lock_poisoned_error() {
        let err = SharedStorageError::LockPoisoned {
            context: "test context".into(),
        };
        let display = err.to_string();
        assert!(display.contains("test context"));
        assert!(display.contains("poisoned"));
    }

    #[tokio::test]
    async fn test_storage_error_to_io_error() {
        let err = SharedStorageError::BatchTooLarge { size: 2000, max: 1000 };
        let io_err: std::io::Error = err.into();
        assert!(io_err.to_string().contains("2000"));
    }

    // =========================================================================
    // Broadcast Tests
    // =========================================================================

    #[tokio::test]
    async fn test_log_broadcast_on_apply() {
        use n0_future::stream;
        use openraft::entry::RaftEntry;
        use openraft::storage::RaftStateMachine;
        use openraft::testing::log_id;

        use crate::log_subscriber::KvOperation;
        use crate::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
        use crate::log_subscriber::LogEntryPayload;
        use crate::types::AppRequest;
        use crate::types::AppTypeConfig;
        use crate::types::NodeId;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_broadcast.redb");

        // Create broadcast channel
        let (sender, mut receiver) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE as usize);

        // Create storage with broadcast
        let mut storage = SharedRedbStorage::with_broadcast(&db_path, Some(sender), "test-node-1").unwrap();

        // Create a test entry using the helper function from openraft::testing
        let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), 1);
        let entry: openraft::Entry<AppTypeConfig> = openraft::Entry::new_normal(log_id, AppRequest::Set {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        });

        // Simulate apply() being called with the entry
        // Note: In real usage, apply() receives EntryResponder tuples
        // For this test, we directly call the broadcast logic
        let entries = stream::iter(vec![Ok((entry, None))]);
        storage.apply(entries).await.unwrap();

        // Verify broadcast was received
        let payload = receiver.try_recv().expect("should receive broadcast");
        assert_eq!(payload.index, 1);
        assert_eq!(payload.term, 1);

        // Verify the operation matches
        match payload.operation {
            KvOperation::Set { key, value } => {
                assert_eq!(key, b"test_key".to_vec());
                assert_eq!(value, b"test_value".to_vec());
            }
            _ => panic!("expected Set operation"),
        }
    }

    #[tokio::test]
    async fn test_snapshot_broadcast_channel() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_snapshot.redb");

        // Create broadcast channels
        let (log_tx, _log_rx) = broadcast::channel::<crate::log_subscriber::LogEntryPayload>(16);
        let (snapshot_tx, _snapshot_rx) = broadcast::channel::<SnapshotEvent>(16);

        // Create storage with both broadcast channels
        let storage =
            SharedRedbStorage::with_broadcasts(&db_path, Some(log_tx), Some(snapshot_tx), "test-node-1").unwrap();

        // Verify storage was created successfully
        assert!(storage.get("test").unwrap().is_none());
    }

    // =========================================================================
    // Raft Metadata Tests
    // =========================================================================

    #[tokio::test]
    async fn test_raft_meta_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Write metadata
        storage.write_raft_meta("test_key", &42u64).unwrap();

        // Read metadata
        let result: Option<u64> = storage.read_raft_meta("test_key").unwrap();
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_raft_meta_read_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result: Option<u64> = storage.read_raft_meta("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_raft_meta_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Write then delete
        storage.write_raft_meta("test_key", &42u64).unwrap();
        storage.delete_raft_meta("test_key").unwrap();

        let result: Option<u64> = storage.read_raft_meta("test_key").unwrap();
        assert!(result.is_none());
    }

    // =========================================================================
    // State Machine Metadata Tests
    // =========================================================================

    #[tokio::test]
    async fn test_sm_meta_read() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Write SM metadata directly
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(SM_META_TABLE).unwrap();
            let data = bincode::serialize(&Some(100u64)).unwrap();
            table.insert("test_sm_meta", data.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();

        // Read via helper
        let result: Option<Option<u64>> = storage.read_sm_meta("test_sm_meta").unwrap();
        assert_eq!(result, Some(Some(100)));
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[tokio::test]
    async fn test_storage_clone_shares_db() {
        let temp_dir = TempDir::new().unwrap();
        let storage1 = create_test_storage(&temp_dir);
        let storage2 = storage1.clone();

        // Insert via storage1
        insert_kv_entry(&storage1, "shared_key", "shared_value", 1);

        // Read via storage2
        let entry = storage2.get("shared_key").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, "shared_value");
    }

    // =========================================================================
    // Index Query Tests
    // =========================================================================

    #[tokio::test]
    async fn test_scan_by_index_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Try to scan by non-existent index
        let result = storage.scan_by_index("nonexistent_index", b"value", 10);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_range_by_index_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Try to range by non-existent index
        let result = storage.range_by_index("nonexistent_index", b"start", b"end", 10);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scan_index_lt_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Try to scan by non-existent index
        let result = storage.scan_index_lt("nonexistent_index", b"threshold", 10);
        assert!(result.is_err());
    }

    // =========================================================================
    // KvEntry Tests
    // =========================================================================

    #[tokio::test]
    async fn test_kv_entry_serialization() {
        let entry = KvEntry {
            value: "test_value".to_string(),
            version: 5,
            create_revision: 10,
            mod_revision: 15,
            expires_at_ms: Some(1234567890),
            lease_id: Some(1001),
        };

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: KvEntry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(entry.value, deserialized.value);
        assert_eq!(entry.version, deserialized.version);
        assert_eq!(entry.create_revision, deserialized.create_revision);
        assert_eq!(entry.mod_revision, deserialized.mod_revision);
        assert_eq!(entry.expires_at_ms, deserialized.expires_at_ms);
        assert_eq!(entry.lease_id, deserialized.lease_id);
    }

    // =========================================================================
    // LeaseEntry Tests
    // =========================================================================

    #[tokio::test]
    async fn test_lease_entry_serialization() {
        let entry = LeaseEntry {
            ttl_seconds: 60,
            expires_at_ms: 1234567890,
            keys: vec!["key1".to_string(), "key2".to_string()],
        };

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: LeaseEntry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(entry.ttl_seconds, deserialized.ttl_seconds);
        assert_eq!(entry.expires_at_ms, deserialized.expires_at_ms);
        assert_eq!(entry.keys, deserialized.keys);
    }

    #[test]
    fn test_lease_entry_debug() {
        let entry = LeaseEntry {
            ttl_seconds: 60,
            expires_at_ms: 1234567890,
            keys: vec!["key1".to_string()],
        };

        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("LeaseEntry"));
        assert!(debug_str.contains("ttl_seconds"));
        assert!(debug_str.contains("60"));
    }

    #[test]
    fn test_lease_entry_clone() {
        let entry = LeaseEntry {
            ttl_seconds: 60,
            expires_at_ms: 1234567890,
            keys: vec!["key1".to_string()],
        };

        let cloned = entry.clone();
        assert_eq!(entry.ttl_seconds, cloned.ttl_seconds);
        assert_eq!(entry.expires_at_ms, cloned.expires_at_ms);
        assert_eq!(entry.keys, cloned.keys);
    }

    // =========================================================================
    // StoredSnapshot Tests
    // =========================================================================

    #[test]
    fn test_stored_snapshot_debug() {
        let snapshot = StoredSnapshot {
            meta: openraft::SnapshotMeta {
                last_log_id: None,
                last_membership: openraft::StoredMembership::default(),
                snapshot_id: "test-snapshot".to_string(),
            },
            data: vec![1, 2, 3],
            integrity: None,
        };

        let debug_str = format!("{:?}", snapshot);
        assert!(debug_str.contains("StoredSnapshot"));
        assert!(debug_str.contains("test-snapshot"));
    }

    #[test]
    fn test_stored_snapshot_clone() {
        let snapshot = StoredSnapshot {
            meta: openraft::SnapshotMeta {
                last_log_id: None,
                last_membership: openraft::StoredMembership::default(),
                snapshot_id: "test-snapshot".to_string(),
            },
            data: vec![1, 2, 3],
            integrity: None,
        };

        let cloned = snapshot.clone();
        assert_eq!(snapshot.meta.snapshot_id, cloned.meta.snapshot_id);
        assert_eq!(snapshot.data, cloned.data);
    }

    // =========================================================================
    // SnapshotEvent Tests
    // =========================================================================

    #[test]
    fn test_snapshot_event_created_debug() {
        let event = SnapshotEvent::Created {
            snapshot_id: "snap-1".to_string(),
            last_log_index: 100,
            term: 5,
            entry_count: 1000,
            size_bytes: 4096,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Created"));
        assert!(debug_str.contains("snap-1"));
        assert!(debug_str.contains("100"));
    }

    #[test]
    fn test_snapshot_event_installed_debug() {
        let event = SnapshotEvent::Installed {
            snapshot_id: "snap-2".to_string(),
            last_log_index: 200,
            term: 10,
            entry_count: 2000,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Installed"));
        assert!(debug_str.contains("snap-2"));
        assert!(debug_str.contains("200"));
    }

    #[test]
    fn test_snapshot_event_clone() {
        let event = SnapshotEvent::Created {
            snapshot_id: "snap-1".to_string(),
            last_log_index: 100,
            term: 5,
            entry_count: 1000,
            size_bytes: 4096,
        };

        let cloned = event.clone();
        match cloned {
            SnapshotEvent::Created {
                snapshot_id,
                last_log_index,
                ..
            } => {
                assert_eq!(snapshot_id, "snap-1");
                assert_eq!(last_log_index, 100);
            }
            _ => panic!("expected Created event"),
        }
    }

    // =========================================================================
    // Directory Creation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_storage_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested/deep/path/test.redb");

        // Parent directories don't exist yet
        assert!(!nested_path.parent().unwrap().exists());

        // Creating storage should create parent directories
        let storage = SharedRedbStorage::new(&nested_path, "test-node-1").unwrap();
        assert!(nested_path.exists());

        // Storage should work
        insert_kv_entry(&storage, "key", "value", 1);
        let entry = storage.get("key").unwrap();
        assert!(entry.is_some());
    }

    // =========================================================================
    // Multiple Node Tests
    // =========================================================================

    #[tokio::test]
    async fn test_different_node_ids_create_different_hlc() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let storage1 = SharedRedbStorage::new(temp_dir1.path().join("test1.redb"), "node-1").unwrap();
        let storage2 = SharedRedbStorage::new(temp_dir2.path().join("test2.redb"), "node-2").unwrap();

        // Both storages should be independent
        insert_kv_entry(&storage1, "key1", "value1", 1);
        insert_kv_entry(&storage2, "key2", "value2", 1);

        assert!(storage1.get("key1").unwrap().is_some());
        assert!(storage1.get("key2").unwrap().is_none());

        assert!(storage2.get("key2").unwrap().is_some());
        assert!(storage2.get("key1").unwrap().is_none());
    }

    // =========================================================================
    // Confirmed Last Applied Tests (Snapshot Race Fix)
    // =========================================================================

    #[tokio::test]
    async fn test_confirmed_last_applied_initialized_to_none() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // confirmed_last_applied should start as None
        let confirmed = storage.confirmed_last_applied.read().unwrap();
        assert!(confirmed.is_none());
    }

    #[tokio::test]
    async fn test_confirmed_last_applied_not_updated_by_append() {
        use openraft::entry::RaftEntry;
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        use crate::types::AppRequest;
        use crate::types::NodeId;

        let temp_dir = TempDir::new().unwrap();
        let mut storage = create_test_storage(&temp_dir);

        // confirmed_last_applied starts as None
        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert!(confirmed.is_none());
        }

        // Append an entry (this eagerly applies to state machine)
        let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), 1);
        let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(log_id, AppRequest::Set {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        });
        storage.append([entry], IOFlushed::<AppTypeConfig>::noop()).await.unwrap();

        // confirmed_last_applied should still be None (not updated by append)
        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert!(confirmed.is_none());
        }

        // But SM_META_TABLE should have last_applied_log updated
        let last_applied_in_db: Option<Option<LogIdOf<AppTypeConfig>>> =
            storage.read_sm_meta("last_applied_log").unwrap();
        assert!(last_applied_in_db.is_some());
        assert_eq!(last_applied_in_db.unwrap().unwrap().index, 1);
    }

    #[tokio::test]
    async fn test_confirmed_last_applied_updated_by_apply() {
        use n0_future::stream;
        use openraft::entry::RaftEntry;
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::storage::RaftStateMachine;
        use openraft::testing::log_id;

        use crate::types::AppRequest;
        use crate::types::NodeId;

        let temp_dir = TempDir::new().unwrap();
        let mut storage = create_test_storage(&temp_dir);

        // Append entries
        let entries = vec![
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 1),
                AppRequest::Set {
                    key: "key1".to_string(),
                    value: "value1".to_string(),
                },
            ),
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 2),
                AppRequest::Set {
                    key: "key2".to_string(),
                    value: "value2".to_string(),
                },
            ),
        ];
        storage.append(entries, IOFlushed::<AppTypeConfig>::noop()).await.unwrap();

        // confirmed_last_applied should still be None before apply()
        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert!(confirmed.is_none());
        }

        // Simulate apply() with the entries
        let apply_entries = vec![
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 1),
                AppRequest::Set {
                    key: "key1".to_string(),
                    value: "value1".to_string(),
                },
            ),
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 2),
                AppRequest::Set {
                    key: "key2".to_string(),
                    value: "value2".to_string(),
                },
            ),
        ];
        let entries_stream = stream::iter(apply_entries.into_iter().map(|e| Ok((e, None))));
        storage.apply(entries_stream).await.unwrap();

        // confirmed_last_applied should now be updated to the last entry
        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert!(confirmed.is_some());
            assert_eq!(confirmed.as_ref().unwrap().index, 2);
        }
    }

    #[tokio::test]
    async fn test_confirmed_last_applied_monotonic() {
        use n0_future::stream;
        use openraft::entry::RaftEntry;
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::storage::RaftStateMachine;
        use openraft::testing::log_id;

        use crate::types::AppRequest;
        use crate::types::NodeId;

        let temp_dir = TempDir::new().unwrap();
        let mut storage = create_test_storage(&temp_dir);

        // Append and apply first batch
        let entries1 = vec![<AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, NodeId::from(1), 5),
            AppRequest::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
        )];
        storage.append(entries1, IOFlushed::<AppTypeConfig>::noop()).await.unwrap();

        let apply_entries1 = vec![<AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, NodeId::from(1), 5),
            AppRequest::Set {
                key: "key1".to_string(),
                value: "value1".to_string(),
            },
        )];
        let stream1 = stream::iter(apply_entries1.into_iter().map(|e| Ok((e, None))));
        storage.apply(stream1).await.unwrap();

        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert_eq!(confirmed.as_ref().unwrap().index, 5);
        }

        // Try to apply an older entry (index 3) - should not regress
        let apply_entries2 = vec![<AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, NodeId::from(1), 3),
            AppRequest::Set {
                key: "key2".to_string(),
                value: "value2".to_string(),
            },
        )];
        let stream2 = stream::iter(apply_entries2.into_iter().map(|e| Ok((e, None))));
        storage.apply(stream2).await.unwrap();

        // confirmed_last_applied should remain at 5 (monotonic)
        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert_eq!(confirmed.as_ref().unwrap().index, 5);
        }
    }

    #[tokio::test]
    async fn test_confirmed_last_applied_lags_behind_sm_meta() {
        use openraft::entry::RaftEntry;
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        use crate::types::AppRequest;
        use crate::types::NodeId;

        let temp_dir = TempDir::new().unwrap();
        let mut storage = create_test_storage(&temp_dir);

        // Append multiple entries (eagerly applies to SM_META_TABLE)
        let entries = vec![
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 1),
                AppRequest::Set {
                    key: "key1".to_string(),
                    value: "value1".to_string(),
                },
            ),
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 2),
                AppRequest::Set {
                    key: "key2".to_string(),
                    value: "value2".to_string(),
                },
            ),
            <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), 3),
                AppRequest::Set {
                    key: "key3".to_string(),
                    value: "value3".to_string(),
                },
            ),
        ];
        storage.append(entries, IOFlushed::<AppTypeConfig>::noop()).await.unwrap();

        // SM_META_TABLE should have last_applied = 3
        let last_applied_in_db: Option<Option<LogIdOf<AppTypeConfig>>> =
            storage.read_sm_meta("last_applied_log").unwrap();
        assert_eq!(last_applied_in_db.unwrap().unwrap().index, 3);

        // But confirmed_last_applied should still be None (not yet confirmed by apply)
        {
            let confirmed = storage.confirmed_last_applied.read().unwrap();
            assert!(confirmed.is_none());
        }

        // This demonstrates the lag: SM_META_TABLE has index 3, confirmed has None
        // This is the TOCTOU race that build_snapshot must account for
    }
}
