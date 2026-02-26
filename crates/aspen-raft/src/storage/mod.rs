//! Raft log and state machine storage backends.
//!
//! Provides pluggable storage implementations for openraft's log and state machine,
//! supporting both in-memory (for testing) and persistent (redb) backends.
//! The unified Redb backend implements both log storage and state machine in a single
//! database file, enabling single-fsync writes for optimal performance (~2-3ms latency).
//!
//! # Key Components
//!
//! - `StorageBackend`: Enum selecting backend (InMemory or Redb)
//! - `SharedRedbStorage`: Unified log+state machine with single-fsync writes (default production)
//! - `RedbLogStore`: Standalone log store (for legacy/testing)
//! - `InMemoryLogStore`: Non-durable log for testing and development
//! - `InMemoryStateMachine`: In-memory state machine implementation
//! - Snapshot management with bounded in-memory snapshots
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_BATCH_SIZE (1024 entries), MAX_SETMULTI_KEYS (100 keys)
//! - Explicit types: u64 for log indices (portable across architectures)
//! - Resource bounds: Snapshot data capped to prevent unbounded memory growth
//! - Disk space checks: Pre-flight validation before writing snapshots
//! - Error handling: Explicit SNAFU errors for each failure mode
//!
//! # Test Coverage
//!
//! RedbLogStore persistence tests are comprehensive (50+ tests):
//! - Log append/read across process restarts (12 tests)
//! - Vote persistence and recovery (8 tests)
//! - Committed index persistence (6 tests)
//! - Chain integrity and hash verification (17 tests)
//! - Truncation and purge edge cases (9 tests)
//!
//! InMemoryStateMachine supports all AppRequest variants including:
//!       - Basic KV: Set, Delete, SetWithTTL, SetMulti, DeleteMulti
//!       - Compare-and-swap: CompareAndSwap, CompareAndDelete
//!       - Batch operations: Batch, ConditionalBatch
//!       - Transactions: Transaction (etcd-style), OptimisticTransaction
//!       - Leases: LeaseGrant, LeaseRevoke, LeaseKeepalive (stub responses)
//!       Coverage: Tested via router tests and 85+ unit tests
//!
//! # Example
//!
//! ```ignore
//! use aspen::raft::storage::{RedbLogStore, StorageBackend};
//! use aspen::raft::storage_shared::SharedRedbStorage;
//!
//! // Create unified Redb storage (single-fsync, production default)
//! let storage = SharedRedbStorage::new("./data/shared.redb")?;
//!
//! // Or use backend selection
//! let backend = StorageBackend::Redb;
//! ```

mod in_memory;
mod redb_store;

// Re-export all public types
use std::io;
use std::path::PathBuf;

pub use in_memory::InMemoryLogStore;
pub use in_memory::InMemoryStateMachine;
use redb::TableDefinition;
pub use redb_store::RedbLogStore;
use serde::Deserialize;
use serde::Serialize;
use snafu::Snafu;

use crate::integrity::SnapshotIntegrity;
use crate::types::AppTypeConfig;

// ====================================================================================
// Storage Backend Configuration
// ====================================================================================

/// Storage backend selection for Raft log and state machine.
///
/// Aspen supports two storage backends:
/// - **Redb**: Single-fsync storage using shared redb for both log and state machine (default)
/// - **InMemory**: Fast, deterministic storage for testing and simulations
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema
)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StorageBackend {
    /// In-memory storage using BTreeMap. Data is lost on restart.
    /// Use for: unit tests, madsim simulations, development.
    InMemory,
    /// Single-fsync storage using shared redb for both log and state machine.
    /// Default storage backend for production deployments.
    /// Bundles state mutations into log appends for single-fsync durability.
    /// Write latency: ~2-3ms (single fsync)
    #[default]
    Redb,
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "inmemory" | "in-memory" | "memory" => Ok(StorageBackend::InMemory),
            "redb" | "single-fsync" | "fast" | "persistent" | "disk" => Ok(StorageBackend::Redb),
            _ => Err(format!("Invalid storage backend '{}'. Valid options: inmemory, redb", s)),
        }
    }
}

impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageBackend::InMemory => write!(f, "inmemory"),
            StorageBackend::Redb => write!(f, "redb"),
        }
    }
}

// ====================================================================================
// Redb Table Definitions (Tiger Style: explicitly named, typed tables)
// ====================================================================================

/// Raft log entries: key = log index (u64), value = serialized Entry
pub(crate) const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");

/// Raft metadata: key = string identifier, value = serialized data
/// Keys: "vote", "committed", "last_purged_log_id"
pub(crate) const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");

// State machine tables removed (were only used by deprecated RedbStateMachine)

/// Snapshot storage
pub(crate) const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Chain hash table: key = log index (u64), value = ChainHash (32 bytes).
///
/// Stores chain hashes separately from log entries to enable fast chain
/// verification without deserializing entries. Each hash depends on the
/// previous hash, creating an unbreakable integrity chain.
pub(crate) const CHAIN_HASH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("chain_hashes");

/// Integrity metadata table: key = string identifier, value = serialized data.
///
/// Keys:
/// - "integrity_version": Schema version for migration detection
/// - "chain_tip_hash": Hash of the most recent entry
/// - "chain_tip_index": Index of the most recent entry
/// - "snapshot_chain_hash": Chain hash at last snapshot point
pub(crate) const INTEGRITY_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("integrity_meta");

// ====================================================================================
// Redb Storage Errors
// ====================================================================================

/// Errors that can occur during Raft storage operations.
#[derive(Debug, Snafu)]
pub enum StorageError {
    /// Failed to open the redb database file.
    #[snafu(display("failed to open redb database at {}: {source}", path.display()))]
    OpenDatabase {
        /// Path to the database file.
        path: PathBuf,
        /// Underlying redb error.
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    /// Failed to begin a write transaction.
    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite {
        /// Underlying redb transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to begin a read transaction.
    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        /// Underlying redb transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to open a database table.
    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        /// Underlying redb table error.
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    /// Failed to commit a transaction.
    #[snafu(display("failed to commit transaction: {source}"))]
    Commit {
        /// Underlying redb commit error.
        #[snafu(source(from(redb::CommitError, Box::new)))]
        source: Box<redb::CommitError>,
    },

    /// Failed to insert a value into a table.
    #[snafu(display("failed to insert into table: {source}"))]
    Insert {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to retrieve a value from a table.
    #[snafu(display("failed to get from table: {source}"))]
    Get {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to remove a value from a table.
    #[snafu(display("failed to remove from table: {source}"))]
    Remove {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to iterate over a table range.
    #[snafu(display("failed to iterate table range: {source}"))]
    Range {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to serialize data with bincode.
    #[snafu(display("failed to serialize data: {source}"))]
    Serialize {
        /// Underlying bincode error.
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    /// Failed to deserialize data with bincode.
    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize {
        /// Underlying bincode error.
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    /// Failed to create a directory for the database.
    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        /// Path to the directory that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Chain integrity violation detected during verification.
    ///
    /// This indicates that a log entry's chain hash does not match the expected
    /// value based on the previous hash. This could indicate hardware corruption
    /// or tampering.
    #[snafu(display("chain integrity violation at index {index}: expected {expected}, found {found}"))]
    ChainIntegrityViolation {
        /// Log index where the violation was detected.
        index: u64,
        /// Expected hash value (hex-encoded).
        expected: String,
        /// Actual hash value found (hex-encoded).
        found: String,
    },

    /// Snapshot integrity verification failed.
    #[snafu(display("snapshot integrity verification failed: {reason}"))]
    SnapshotIntegrityFailed {
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// Chain hash is missing at the specified log index.
    ///
    /// This should not occur in normal operation and indicates incomplete
    /// migration or database corruption.
    #[snafu(display("chain hash missing at index {index}"))]
    ChainHashMissing {
        /// Log index where the chain hash is missing.
        index: u64,
    },

    /// A storage lock was poisoned due to a panic in another thread.
    #[snafu(display("storage lock poisoned: {context}"))]
    LockPoisoned {
        /// Context describing which lock was poisoned.
        context: String,
    },
}

impl From<StorageError> for io::Error {
    fn from(err: StorageError) -> io::Error {
        io::Error::other(err.to_string())
    }
}

/// Snapshot blob stored in memory for testing.
///
/// Contains both the snapshot metadata (last log ID, membership) and
/// the serialized state machine data, along with optional integrity hash
/// for corruption detection.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredSnapshot {
    /// Snapshot metadata (last log ID, membership, snapshot ID).
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    /// Serialized state machine data (JSON-encoded KV map).
    pub data: Vec<u8>,
    /// Optional integrity hash for corruption detection (Tiger Style).
    #[serde(default)]
    pub integrity: Option<SnapshotIntegrity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // StorageBackend Enum Tests
    // =========================================================================

    #[test]
    fn test_storage_backend_default() {
        let backend = StorageBackend::default();
        assert_eq!(backend, StorageBackend::Redb);
    }

    #[test]
    fn test_storage_backend_from_str_inmemory() {
        assert_eq!("inmemory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
        assert_eq!("in-memory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
        assert_eq!("memory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
    }

    #[test]
    fn test_storage_backend_from_str_redb() {
        assert_eq!("redb".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("single-fsync".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("fast".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("persistent".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("disk".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
    }

    #[test]
    fn test_storage_backend_from_str_case_insensitive() {
        assert_eq!("INMEMORY".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
        assert_eq!("REDB".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("InMemory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
    }

    #[test]
    fn test_storage_backend_from_str_invalid() {
        let result = "invalid".parse::<StorageBackend>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid storage backend"));
    }

    #[test]
    fn test_storage_backend_display_inmemory() {
        assert_eq!(format!("{}", StorageBackend::InMemory), "inmemory");
    }

    #[test]
    fn test_storage_backend_display_redb() {
        assert_eq!(format!("{}", StorageBackend::Redb), "redb");
    }

    #[test]
    fn test_storage_backend_roundtrip() {
        let original = StorageBackend::InMemory;
        let display = format!("{}", original);
        let parsed: StorageBackend = display.parse().unwrap();
        assert_eq!(original, parsed);

        let original = StorageBackend::Redb;
        let display = format!("{}", original);
        let parsed: StorageBackend = display.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_storage_backend_clone() {
        let backend = StorageBackend::Redb;
        let cloned = backend;
        assert_eq!(backend, cloned);
    }

    #[test]
    fn test_storage_backend_debug() {
        let debug_str = format!("{:?}", StorageBackend::InMemory);
        assert!(debug_str.contains("InMemory"));
    }

    #[test]
    fn test_storage_backend_serde_roundtrip() {
        let original = StorageBackend::Redb;
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: StorageBackend = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_storage_backend_serde_inmemory() {
        let original = StorageBackend::InMemory;
        let json = serde_json::to_string(&original).expect("serialize");
        assert_eq!(json, "\"inmemory\"");
    }

    // =========================================================================
    // StorageError Tests
    // =========================================================================

    #[test]
    fn test_storage_error_into_io_error() {
        let err = StorageError::ChainIntegrityViolation {
            index: 42,
            expected: "abc".to_string(),
            found: "def".to_string(),
        };
        let io_err: io::Error = err.into();
        let msg = io_err.to_string();
        assert!(msg.contains("chain integrity violation"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn test_storage_error_snapshot_integrity_failed() {
        let err = StorageError::SnapshotIntegrityFailed {
            reason: "corrupted data".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("snapshot integrity"));
        assert!(msg.contains("corrupted data"));
    }

    #[test]
    fn test_storage_error_chain_hash_missing() {
        let err = StorageError::ChainHashMissing { index: 100 };
        let msg = format!("{}", err);
        assert!(msg.contains("chain hash missing"));
        assert!(msg.contains("100"));
    }

    // =========================================================================
    // StoredSnapshot Tests
    // =========================================================================

    #[test]
    fn test_stored_snapshot_serde() {
        use openraft::Membership;
        use openraft::SnapshotMeta;
        use openraft::StoredMembership;

        let membership = Membership::<AppTypeConfig>::new_with_defaults(vec![], []);
        let meta = SnapshotMeta {
            last_log_id: None,
            last_membership: StoredMembership::new(None, membership),
            snapshot_id: "test-snap".to_string(),
        };
        let snapshot = StoredSnapshot {
            meta,
            data: vec![1, 2, 3, 4],
            integrity: None,
        };

        let serialized = bincode::serialize(&snapshot).expect("serialize");
        let deserialized: StoredSnapshot = bincode::deserialize(&serialized).expect("deserialize");

        assert_eq!(deserialized.data, vec![1, 2, 3, 4]);
        assert_eq!(deserialized.meta.snapshot_id, "test-snap");
    }
}
