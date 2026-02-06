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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use redb::TableHandle;

    use super::*;

    // ============================================================================
    // KvEntry construction and field access
    // ============================================================================

    #[test]
    fn kv_entry_construction_basic() {
        let entry = KvEntry {
            value: "test_value".to_string(),
            version: 1,
            create_revision: 100,
            mod_revision: 100,
            expires_at_ms: None,
            lease_id: None,
        };

        assert_eq!(entry.value, "test_value");
        assert_eq!(entry.version, 1);
        assert_eq!(entry.create_revision, 100);
        assert_eq!(entry.mod_revision, 100);
        assert!(entry.expires_at_ms.is_none());
        assert!(entry.lease_id.is_none());
    }

    #[test]
    fn kv_entry_with_ttl() {
        let entry = KvEntry {
            value: "expiring".to_string(),
            version: 1,
            create_revision: 50,
            mod_revision: 50,
            expires_at_ms: Some(1_700_000_000_000),
            lease_id: None,
        };

        assert_eq!(entry.expires_at_ms, Some(1_700_000_000_000));
    }

    #[test]
    fn kv_entry_with_lease() {
        let entry = KvEntry {
            value: "leased".to_string(),
            version: 1,
            create_revision: 75,
            mod_revision: 75,
            expires_at_ms: None,
            lease_id: Some(12345),
        };

        assert_eq!(entry.lease_id, Some(12345));
    }

    #[test]
    fn kv_entry_with_ttl_and_lease() {
        let entry = KvEntry {
            value: "both".to_string(),
            version: 3,
            create_revision: 10,
            mod_revision: 30,
            expires_at_ms: Some(1_800_000_000_000),
            lease_id: Some(99999),
        };

        assert!(entry.expires_at_ms.is_some());
        assert!(entry.lease_id.is_some());
        assert_eq!(entry.version, 3);
    }

    // ============================================================================
    // KvEntry serialization round-trip tests
    // ============================================================================

    #[test]
    fn kv_entry_bincode_roundtrip() {
        let original = KvEntry {
            value: "bincode_test".to_string(),
            version: 42,
            create_revision: 1000,
            mod_revision: 1005,
            expires_at_ms: Some(9999999999),
            lease_id: Some(777),
        };

        let encoded = bincode::serialize(&original).expect("bincode serialize failed");
        let decoded: KvEntry = bincode::deserialize(&encoded).expect("bincode deserialize failed");

        assert_eq!(original, decoded);
    }

    #[test]
    fn kv_entry_bincode_roundtrip_empty_value() {
        let original = KvEntry {
            value: "".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: None,
            lease_id: None,
        };

        let encoded = bincode::serialize(&original).expect("bincode serialize failed");
        let decoded: KvEntry = bincode::deserialize(&encoded).expect("bincode deserialize failed");

        assert_eq!(original, decoded);
    }

    #[test]
    fn kv_entry_bincode_roundtrip_large_value() {
        let large_value = "x".repeat(100_000);
        let original = KvEntry {
            value: large_value,
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: None,
            lease_id: None,
        };

        let encoded = bincode::serialize(&original).expect("bincode serialize failed");
        let decoded: KvEntry = bincode::deserialize(&encoded).expect("bincode deserialize failed");

        assert_eq!(original, decoded);
    }

    #[test]
    fn kv_entry_bincode_roundtrip_unicode() {
        let original = KvEntry {
            value: "Hello, \u{1F600} World! \u{4E2D}\u{6587}".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: None,
            lease_id: None,
        };

        let encoded = bincode::serialize(&original).expect("bincode serialize failed");
        let decoded: KvEntry = bincode::deserialize(&encoded).expect("bincode deserialize failed");

        assert_eq!(original, decoded);
    }

    #[test]
    fn kv_entry_postcard_roundtrip() {
        let original = KvEntry {
            value: "postcard_test".to_string(),
            version: 100,
            create_revision: 500,
            mod_revision: 600,
            expires_at_ms: Some(123456789),
            lease_id: Some(42),
        };

        let encoded = postcard::to_allocvec(&original).expect("postcard serialize failed");
        let decoded: KvEntry = postcard::from_bytes(&encoded).expect("postcard deserialize failed");

        assert_eq!(original, decoded);
    }

    // ============================================================================
    // KvEntry edge cases for revisions
    // ============================================================================

    #[test]
    fn kv_entry_max_revision_values() {
        let entry = KvEntry {
            value: "max".to_string(),
            version: i64::MAX,
            create_revision: i64::MAX,
            mod_revision: i64::MAX,
            expires_at_ms: Some(u64::MAX),
            lease_id: Some(u64::MAX),
        };

        let encoded = bincode::serialize(&entry).expect("bincode serialize failed");
        let decoded: KvEntry = bincode::deserialize(&encoded).expect("bincode deserialize failed");

        assert_eq!(entry, decoded);
    }

    #[test]
    fn kv_entry_zero_revision_values() {
        let entry = KvEntry {
            value: "zero".to_string(),
            version: 0,
            create_revision: 0,
            mod_revision: 0,
            expires_at_ms: Some(0),
            lease_id: Some(0),
        };

        assert_eq!(entry.version, 0);
        assert_eq!(entry.create_revision, 0);
    }

    #[test]
    fn kv_entry_negative_version() {
        // While negative versions shouldn't occur in practice, the type allows it
        let entry = KvEntry {
            value: "negative".to_string(),
            version: -1,
            create_revision: -100,
            mod_revision: -50,
            expires_at_ms: None,
            lease_id: None,
        };

        let encoded = bincode::serialize(&entry).expect("bincode serialize failed");
        let decoded: KvEntry = bincode::deserialize(&encoded).expect("bincode deserialize failed");

        assert_eq!(entry, decoded);
    }

    // ============================================================================
    // KvEntry Clone and Debug
    // ============================================================================

    #[test]
    fn kv_entry_clone() {
        let original = KvEntry {
            value: "clone_test".to_string(),
            version: 5,
            create_revision: 10,
            mod_revision: 15,
            expires_at_ms: Some(1000),
            lease_id: Some(2000),
        };

        let cloned = original.clone();

        assert_eq!(original, cloned);
        // Verify they are independent
        assert_eq!(cloned.value, "clone_test");
    }

    #[test]
    fn kv_entry_debug() {
        let entry = KvEntry {
            value: "debug".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: None,
            lease_id: None,
        };

        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("KvEntry"));
        assert!(debug_str.contains("debug"));
        assert!(debug_str.contains("version"));
    }

    // ============================================================================
    // SM_KV_TABLE constant test
    // ============================================================================

    #[test]
    fn sm_kv_table_name_is_correct() {
        // Verify the table name is what we expect
        assert_eq!(SM_KV_TABLE.name(), "sm_kv");
    }
}
