//! Arrow schema definitions for Redb KV storage.
//!
//! Defines the schema for the virtual `kv` table that exposes Redb
//! key-value data to DataFusion SQL queries.

use std::sync::{Arc, LazyLock};

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// Arrow schema for the `kv` table.
///
/// The KV table exposes the following columns:
/// - `key`: UTF-8 string (the key)
/// - `value`: UTF-8 string (the value)
/// - `version`: Int64 (per-key version counter, starts at 1)
/// - `create_revision`: Int64 (Raft log index when key was created)
/// - `mod_revision`: Int64 (Raft log index of last modification)
/// - `expires_at_ms`: UInt64 (optional TTL expiration timestamp, nullable)
/// - `lease_id`: UInt64 (optional attached lease ID, nullable)
pub static KV_SCHEMA: LazyLock<SchemaRef> = LazyLock::new(kv_schema);

/// Create the Arrow schema for the KV table.
///
/// This is exposed as a function for cases where the static reference
/// isn't suitable (e.g., testing).
pub fn kv_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("key", DataType::Utf8, false),
        Field::new("value", DataType::Utf8, false),
        Field::new("version", DataType::Int64, false),
        Field::new("create_revision", DataType::Int64, false),
        Field::new("mod_revision", DataType::Int64, false),
        Field::new("expires_at_ms", DataType::UInt64, true),
        Field::new("lease_id", DataType::UInt64, true),
    ]))
}

/// Column indices for efficient access.
#[allow(dead_code)] // Used for future column-specific operations
pub mod columns {
    /// Index of the `key` column.
    pub const KEY: usize = 0;
    /// Index of the `value` column.
    pub const VALUE: usize = 1;
    /// Index of the `version` column.
    pub const VERSION: usize = 2;
    /// Index of the `create_revision` column.
    pub const CREATE_REVISION: usize = 3;
    /// Index of the `mod_revision` column.
    pub const MOD_REVISION: usize = 4;
    /// Index of the `expires_at_ms` column.
    pub const EXPIRES_AT_MS: usize = 5;
    /// Index of the `lease_id` column.
    pub const LEASE_ID: usize = 6;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_correct_columns() {
        let schema = kv_schema();
        assert_eq!(schema.fields().len(), 7);

        // Check column names and types
        assert_eq!(schema.field(0).name(), "key");
        assert_eq!(schema.field(0).data_type(), &DataType::Utf8);
        assert!(!schema.field(0).is_nullable());

        assert_eq!(schema.field(1).name(), "value");
        assert_eq!(schema.field(1).data_type(), &DataType::Utf8);
        assert!(!schema.field(1).is_nullable());

        assert_eq!(schema.field(2).name(), "version");
        assert_eq!(schema.field(2).data_type(), &DataType::Int64);

        assert_eq!(schema.field(3).name(), "create_revision");
        assert_eq!(schema.field(3).data_type(), &DataType::Int64);

        assert_eq!(schema.field(4).name(), "mod_revision");
        assert_eq!(schema.field(4).data_type(), &DataType::Int64);

        assert_eq!(schema.field(5).name(), "expires_at_ms");
        assert_eq!(schema.field(5).data_type(), &DataType::UInt64);
        assert!(schema.field(5).is_nullable());

        assert_eq!(schema.field(6).name(), "lease_id");
        assert_eq!(schema.field(6).data_type(), &DataType::UInt64);
        assert!(schema.field(6).is_nullable());
    }

    #[test]
    fn column_indices_match_schema() {
        let schema = kv_schema();
        assert_eq!(schema.field(columns::KEY).name(), "key");
        assert_eq!(schema.field(columns::VALUE).name(), "value");
        assert_eq!(schema.field(columns::VERSION).name(), "version");
        assert_eq!(
            schema.field(columns::CREATE_REVISION).name(),
            "create_revision"
        );
        assert_eq!(schema.field(columns::MOD_REVISION).name(), "mod_revision");
        assert_eq!(schema.field(columns::EXPIRES_AT_MS).name(), "expires_at_ms");
        assert_eq!(schema.field(columns::LEASE_ID).name(), "lease_id");
    }
}
