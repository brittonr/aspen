//! Shell-facing storage helpers.
//!
//! Alloc-only consumers keep portable record types in `aspen-core::storage`,
//! while shell consumers opt into Redb table definitions here.

pub use aspen_core::storage::KvEntry;
use redb::TableDefinition;

/// State machine KV table definition for Redb-backed shells.
pub const SM_KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_kv");
