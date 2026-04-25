//! Storage types and table definitions for RedbKvStorage.

use redb::TableDefinition;
use serde::Deserialize;
use serde::Serialize;

use crate::SnapshotIntegrity;
use aspen_raft_kv_types::RaftKvTypeConfig;

/// Raft log entries: key = log index (u64), value = serialized Entry
pub const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");

/// Raft metadata: key = string identifier, value = serialized data
pub const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");

/// Snapshot storage
pub const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Chain hash table: key = log index (u64), value = ChainHash (32 bytes)
pub const CHAIN_HASH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("chain_hashes");

/// Integrity metadata table
pub const INTEGRITY_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("integrity_meta");

/// KV state machine table: key = key bytes, value = serialized KvEntry
pub const SM_KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_kv");

/// Lease data: key = lease_id (u64), value = serialized LeaseEntry
pub const SM_LEASES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("sm_leases");

/// State machine metadata: key = string identifier, value = serialized data
pub const SM_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");

/// Secondary index table: key = index entry key (packed tuple), value = empty
pub const SM_INDEX_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_index");

/// Lease entry stored in the state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseEntry {
    pub ttl_seconds: u32,
    pub expires_at_ms: u64,
    pub keys: Vec<String>,
}

#[inline]
fn default_snapshot_integrity() -> Option<SnapshotIntegrity> {
    None
}

/// Stored snapshot format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSnapshot {
    pub meta: openraft::SnapshotMeta<RaftKvTypeConfig>,
    pub data: Vec<u8>,
    #[serde(default = "default_snapshot_integrity")]
    pub integrity: Option<SnapshotIntegrity>,
}

/// Events emitted by snapshot operations.
#[derive(Debug, Clone)]
pub enum SnapshotEvent {
    Created {
        snapshot_id: String,
        last_log_index: u64,
        term: u64,
        entry_count: u64,
        size_bytes: u64,
    },
    Installed {
        snapshot_id: String,
        last_log_index: u64,
        term: u64,
        entry_count: u64,
    },
}
