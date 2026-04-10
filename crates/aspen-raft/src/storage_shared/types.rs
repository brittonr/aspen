//! Storage types and table definitions for SharedRedbStorage.
//!
//! This module contains the core types and table definitions used by the
//! single-fsync Redb storage implementation.

use redb::TableDefinition;
use serde::Deserialize;
use serde::Serialize;

use crate::integrity::SnapshotIntegrity;
use crate::types::AppTypeConfig;

// ====================================================================================
// Table Definitions
// ====================================================================================

/// Raft log entries: key = log index (u64), value = serialized Entry
pub const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");

/// Raft metadata: key = string identifier, value = serialized data
/// Keys: "vote", "committed", "last_purged_log_id"
pub const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");

/// Snapshot storage
pub const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Chain hash table: key = log index (u64), value = ChainHash (32 bytes)
pub const CHAIN_HASH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("chain_hashes");

/// Integrity metadata table
pub const INTEGRITY_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("integrity_meta");

// State machine tables - re-export from aspen-core for consistency
pub use aspen_core::storage::KvEntry;
pub use aspen_core::storage::SM_KV_TABLE;

/// Lease data: key = lease_id (u64), value = serialized LeaseEntry
pub const SM_LEASES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("sm_leases");

/// State machine metadata: key = string identifier, value = serialized data
/// Keys: "last_applied_log", "last_membership"
pub const SM_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");

/// Secondary index table: key = index entry key (packed tuple), value = empty
/// Index keys have format: (index_subspace, indexed_value, primary_key) -> ()
pub const SM_INDEX_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_index");

// ====================================================================================
// Trust Tables (feature-gated)
// ====================================================================================

/// Trust shares table: key = epoch (u64), value = serialized Share (33 bytes).
/// Each node stores its own share for the cluster secret at the given epoch.
#[cfg(feature = "trust")]
pub const TRUST_SHARES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("trust_shares");

/// Trust digests table: key = (epoch, node_id) packed as "epoch:node_id",
/// value = SHA3-256 digest (32 bytes). Stores the digest of every node's share.
#[cfg(feature = "trust")]
pub const TRUST_DIGESTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("trust_digests");

/// Trust encrypted-chain table: key = epoch (u64), value = serialized EncryptedSecretChain.
#[cfg(feature = "trust")]
pub const TRUST_CHAINS_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("trust_chains");

/// Trust epoch membership table: key = (epoch, node_id) packed as "epoch:node_id",
/// value = serialized EndpointAddr.
#[cfg(feature = "trust")]
pub const TRUST_MEMBERS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("trust_members");

/// Trust expungement table: singleton (key = 0), value = serialized ExpungedMetadata.
/// Once set, the node is permanently expelled and refuses all trust/Raft operations.
#[cfg(feature = "trust")]
pub const TRUST_EXPUNGED_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("trust_expunged");

/// Trust nonce counter table: key = node_id (u64), value = last persisted counter (u64).
/// Used by `NonceGenerator` for at-rest encryption to prevent nonce reuse after restart.
#[cfg(feature = "trust")]
pub const TRUST_NONCE_COUNTER_TABLE: TableDefinition<u64, u64> = TableDefinition::new("trust_nonce_counter");

/// Re-encryption progress table: key = table name (str), value = last processed key (bytes).
/// Tracks checkpoint state for the background re-encryption task so it can resume after crash.
#[cfg(feature = "trust")]
pub const TRUST_REENCRYPTION_PROGRESS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("trust_reencryption_progress");

// ====================================================================================
// Storage Types
// ====================================================================================

/// Lease entry stored in the state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseEntry {
    /// Time-to-live in seconds.
    pub ttl_seconds: u32,
    /// When the lease expires (Unix milliseconds).
    pub expires_at_ms: u64,
    /// Keys attached to this lease.
    pub keys: Vec<String>,
}

/// Stored snapshot format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSnapshot {
    /// Raft snapshot metadata.
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    /// Serialized snapshot data.
    pub data: Vec<u8>,
    /// Optional integrity hash.
    #[serde(default)]
    pub integrity: Option<SnapshotIntegrity>,
}

// ====================================================================================
// Snapshot Events
// ====================================================================================

/// Events emitted by snapshot operations for hook integration.
#[derive(Debug, Clone)]
pub enum SnapshotEvent {
    /// A snapshot was created (built) by this node.
    Created {
        /// Snapshot ID.
        snapshot_id: String,
        /// Log index included in the snapshot.
        last_log_index: u64,
        /// Term of the last log entry in the snapshot.
        term: u64,
        /// Number of KV entries in the snapshot.
        entry_count: u64,
        /// Size of the snapshot data in bytes.
        size_bytes: u64,
    },
    /// A snapshot was installed (received from another node).
    Installed {
        /// Snapshot ID.
        snapshot_id: String,
        /// Log index included in the snapshot.
        last_log_index: u64,
        /// Term of the last log entry in the snapshot.
        term: u64,
        /// Number of KV entries installed.
        entry_count: u64,
    },
}
