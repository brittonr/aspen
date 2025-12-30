//! Centralized storage constants and types.
//!
//! This module provides storage-related constants that are shared between
//! the Raft state machine and other modules (like SQL executor).
//!
//! # Motivation
//!
//! Previously, `SM_KV_TABLE` and `KvEntry` were defined in `raft/storage_shared.rs`
//! with duplicates in `sql/stream.rs`, tests, and benchmarks. Centralizing them
//! here eliminates duplication and ensures consistency.

use redb::TableDefinition;
use serde::Deserialize;
use serde::Serialize;

/// State machine KV table definition.
///
/// This is the Redb table where all key-value data is stored.
/// Public for direct access by SQL executor and other modules.
pub const SM_KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_kv");

/// Key-value entry stored in the state machine.
///
/// This struct is shared between:
/// - `raft/storage_shared.rs` for state machine writes
/// - `sql/` module for SQL query execution
/// - Tests and benchmarks
///
/// # Serialization
///
/// Serialized using bincode for efficient storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvEntry {
    /// The value stored for this key.
    pub value: String,
    /// Per-key version counter (1, 2, 3...). Reset to 1 on delete+recreate.
    pub version: i64,
    /// Raft log index when key was first created.
    pub create_revision: i64,
    /// Raft log index of last modification.
    pub mod_revision: i64,
    /// Optional expiration timestamp (Unix milliseconds).
    pub expires_at_ms: Option<u64>,
    /// Optional lease ID this key is attached to.
    pub lease_id: Option<u64>,
}
