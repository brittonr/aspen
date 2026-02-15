//! Secondary index framework for Aspen.
//!
//! Provides FoundationDB-style secondary indexes with transactional guarantees.
//! Index entries are updated atomically with primary KV writes in the same
//! single-fsync transaction.
//!
//! # Index Entry Format
//!
//! Following the FoundationDB pattern, index entries store the indexed value
//! and primary key in the key, with an empty value:
//!
//! ```text
//! Key:   (index_subspace, indexed_value, primary_key) -> ""
//! ```
//!
//! This enables efficient range scans on indexed values while maintaining
//! the ability to look up the primary key.
//!
//! # Built-in Indexes
//!
//! - `idx_mod_revision`: Query keys by modification revision
//! - `idx_create_revision`: Query keys by creation revision
//! - `idx_expires_at`: Query keys by expiration time (for TTL cleanup)
//! - `idx_lease_id`: Query keys attached to a specific lease
//!
//! # Example
//!
//! ```ignore
//! use aspen_layer::{IndexRegistry, Subspace, Tuple};
//!
//! // Create index namespace
//! let idx_subspace = Subspace::new(Tuple::new().push("idx"));
//!
//! // Create registry with built-in indexes
//! let registry = IndexRegistry::with_builtins(idx_subspace);
//!
//! // Generate updates for a write operation
//! let updates = registry.updates_for_set(
//!     b"my-key",
//!     None,        // No previous entry
//!     &new_entry,  // New KvEntry
//! );
//!
//! // Apply updates in same transaction as primary write
//! for key in updates.inserts {
//!     index_table.insert(&key, &[]);
//! }
//! ```

mod errors;
mod field_types;
mod registry;
mod scan;
mod secondary_index;
mod updates;

// =============================================================================
// Constants (Tiger Style)
// =============================================================================

/// Maximum number of indexes per registry.
/// Tiger Style: Bounded to prevent unbounded memory use.
pub const MAX_INDEXES: u32 = 32;

/// Maximum number of index entries returned per scan.
/// Tiger Style: Bounded result set prevents memory exhaustion.
pub const MAX_INDEX_SCAN_RESULTS: u32 = 10_000;

/// System key prefix for index metadata storage.
pub const INDEX_METADATA_PREFIX: &str = "/_sys/index/";

// Re-export all public types
pub use errors::IndexError;
pub use errors::IndexResult;
pub use field_types::IndexDefinition;
pub use field_types::IndexFieldType;
pub use field_types::IndexOptions;
pub use registry::IndexRegistry;
pub use scan::IndexQueryExecutor;
pub use scan::IndexScanResult;
pub use scan::extract_primary_key_from_tuple;
pub use secondary_index::IndexableEntry;
pub use secondary_index::KeyExtractor;
pub use secondary_index::SecondaryIndex;
pub use updates::IndexUpdate;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::subspace::Subspace;
    use crate::tuple::Tuple;

    fn make_test_entry(mod_revision: i64, create_revision: i64) -> IndexableEntry {
        IndexableEntry {
            value: "test".to_string(),
            version: 1,
            create_revision,
            mod_revision,
            expires_at_ms: None,
            lease_id: None,
        }
    }

    #[test]
    fn test_index_creation() {
        let subspace = Subspace::new(Tuple::new().push("idx").push("test"));
        let index = SecondaryIndex::new(
            "test_index",
            subspace,
            Arc::new(|entry| Some(entry.mod_revision.to_be_bytes().to_vec())),
        );

        assert_eq!(index.name(), "test_index");
        assert!(!index.is_numeric());
    }

    #[test]
    fn test_numeric_index() {
        let subspace = Subspace::new(Tuple::new().push("idx").push("numeric"));
        let index = SecondaryIndex::numeric(
            "numeric_index",
            subspace,
            Arc::new(|entry| Some(entry.mod_revision.to_be_bytes().to_vec())),
        );

        assert!(index.is_numeric());
    }

    #[test]
    fn test_build_key() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::new("test", subspace, Arc::new(|_| Some(vec![1, 2, 3])));

        let key = index.build_key(&[1, 2, 3], b"primary-key");
        assert!(!key.is_empty());

        // Key should contain subspace prefix
        assert!(key.starts_with(index.subspace().raw_prefix()));
    }

    #[test]
    fn test_registry_creation() {
        let registry = IndexRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_with_builtins() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        assert_eq!(registry.len(), 4);
        assert!(registry.get("idx_mod_revision").is_some());
        assert!(registry.get("idx_create_revision").is_some());
        assert!(registry.get("idx_expires_at").is_some());
        assert!(registry.get("idx_lease_id").is_some());
    }

    #[test]
    fn test_updates_for_set_new_entry() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        let entry = make_test_entry(100, 50);
        let updates = registry.updates_for_set(b"my-key", None, &entry);

        // Should have inserts for mod_revision and create_revision
        // No expires_at or lease_id since they're None
        assert_eq!(updates.inserts.len(), 2);
        assert!(updates.deletes.is_empty());
    }

    #[test]
    fn test_updates_for_set_update_entry() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        let old_entry = make_test_entry(50, 50);
        let new_entry = make_test_entry(100, 50); // Same create, different mod

        let updates = registry.updates_for_set(b"my-key", Some(&old_entry), &new_entry);

        // Should have deletes for old indexes and inserts for new
        assert!(!updates.inserts.is_empty());
        assert!(!updates.deletes.is_empty());
    }

    #[test]
    fn test_updates_for_delete() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        let entry = make_test_entry(100, 50);
        let updates = registry.updates_for_delete(b"my-key", &entry);

        // Should have deletes for mod_revision and create_revision
        assert_eq!(updates.deletes.len(), 2);
        assert!(updates.inserts.is_empty());
    }

    #[test]
    fn test_optional_fields_not_indexed() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        // Entry without expires_at or lease_id
        let entry = IndexableEntry {
            value: "test".to_string(),
            version: 1,
            create_revision: 10,
            mod_revision: 20,
            expires_at_ms: None,
            lease_id: None,
        };

        let updates = registry.updates_for_set(b"key", None, &entry);

        // Only mod_revision and create_revision should be indexed
        assert_eq!(updates.inserts.len(), 2);
    }

    #[test]
    fn test_optional_fields_are_indexed() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        // Entry with expires_at and lease_id
        let entry = IndexableEntry {
            value: "test".to_string(),
            version: 1,
            create_revision: 10,
            mod_revision: 20,
            expires_at_ms: Some(1234567890),
            lease_id: Some(42),
        };

        let updates = registry.updates_for_set(b"key", None, &entry);

        // All 4 indexes should have entries
        assert_eq!(updates.inserts.len(), 4);
    }

    #[test]
    fn test_index_update_empty() {
        let update = IndexUpdate::empty();
        assert!(update.is_empty());
        assert_eq!(update.operation_count(), 0);
    }

    #[test]
    fn test_max_indexes_limit() {
        let mut registry = IndexRegistry::new();
        let subspace = Subspace::new(Tuple::new().push("idx"));

        // Register MAX_INDEXES indexes
        for i in 0..MAX_INDEXES {
            let index = SecondaryIndex::new(
                format!("index_{i}"),
                subspace.subspace(&Tuple::new().push(i as i64)),
                Arc::new(|_| Some(vec![])),
            );
            registry.register(index).expect("should succeed");
        }

        // Next registration should fail
        let extra =
            SecondaryIndex::new("extra", subspace.subspace(&Tuple::new().push("extra")), Arc::new(|_| Some(vec![])));
        assert!(matches!(registry.register(extra), Err(IndexError::TooManyIndexes)));
    }

    #[test]
    fn test_range_for_value() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::numeric(
            "numeric",
            subspace,
            Arc::new(|entry| Some(entry.mod_revision.to_be_bytes().to_vec())),
        );

        let value = 100i64.to_be_bytes();
        let (start, end) = index.range_for_value(&value);

        // Start should be less than end
        assert!(start < end);
    }

    #[test]
    fn test_range_lt() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::numeric(
            "expires",
            subspace.clone(),
            Arc::new(|entry| entry.expires_at_ms.map(|ms| (ms as i64).to_be_bytes().to_vec())),
        );

        let threshold = 1000i64.to_be_bytes();
        let (start, end) = index.range_lt(&threshold);

        // Start should be subspace start
        let (expected_start, _) = subspace.range();
        assert_eq!(start, expected_start);

        // End should be at threshold
        assert!(end > start);
    }

    #[test]
    fn test_extract_primary_key_from_tuple() {
        let tuple = Tuple::new().push("indexed_value").push("primary_key");

        let pk = extract_primary_key_from_tuple(&tuple);
        assert_eq!(pk, Some(b"primary_key".to_vec()));
    }

    #[test]
    fn test_extract_primary_key_bytes() {
        let tuple = Tuple::new().push(100i64).push(vec![1u8, 2, 3, 4]);

        let pk = extract_primary_key_from_tuple(&tuple);
        assert_eq!(pk, Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_index_scan_result() {
        let mut result = IndexScanResult::empty();
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);

        result.primary_keys.push(b"key1".to_vec());
        result.primary_keys.push(b"key2".to_vec());

        assert!(!result.is_empty());
        assert_eq!(result.len(), 2);
    }

    // =========================================================================
    // IndexDefinition Tests
    // =========================================================================

    #[test]
    fn test_index_definition_builtin() {
        let def = IndexDefinition::builtin("idx_test", "test_field", IndexFieldType::Integer);
        assert_eq!(def.name, "idx_test");
        assert_eq!(def.field, Some("test_field".to_string()));
        assert!(def.builtin);
        assert_eq!(def.field_type, IndexFieldType::Integer);
    }

    #[test]
    fn test_index_definition_custom() {
        let def = IndexDefinition::custom("my_index", "custom_field", IndexFieldType::String);
        assert_eq!(def.name, "my_index");
        assert_eq!(def.field, Some("custom_field".to_string()));
        assert!(!def.builtin);
        assert_eq!(def.field_type, IndexFieldType::String);
    }

    #[test]
    fn test_index_definition_with_options() {
        let def = IndexDefinition::custom("unique_idx", "email", IndexFieldType::String)
            .with_unique(true)
            .with_index_nulls(true);

        assert!(def.options.unique);
        assert!(def.options.index_nulls);
    }

    #[test]
    fn test_index_definition_system_key() {
        let def = IndexDefinition::builtin("idx_test", "field", IndexFieldType::Integer);
        assert_eq!(def.system_key(), "/_sys/index/idx_test");
    }

    #[test]
    fn test_index_definition_builtins() {
        let builtins = IndexDefinition::builtins();
        assert_eq!(builtins.len(), 4);

        let names: Vec<&str> = builtins.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"idx_mod_revision"));
        assert!(names.contains(&"idx_create_revision"));
        assert!(names.contains(&"idx_expires_at"));
        assert!(names.contains(&"idx_lease_id"));
    }

    #[test]
    fn test_index_definition_serialization() {
        let def = IndexDefinition::custom("my_idx", "my_field", IndexFieldType::Integer)
            .with_unique(true)
            .with_index_nulls(false);

        let json = serde_json::to_string(&def).unwrap();
        let roundtrip: IndexDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(def.name, roundtrip.name);
        assert_eq!(def.field, roundtrip.field);
        assert_eq!(def.field_type, roundtrip.field_type);
        assert_eq!(def.builtin, roundtrip.builtin);
        assert_eq!(def.options.unique, roundtrip.options.unique);
        assert_eq!(def.options.index_nulls, roundtrip.options.index_nulls);
    }

    // =========================================================================
    // IndexFieldType Tests
    // =========================================================================

    #[test]
    fn test_index_field_type_default() {
        let default: IndexFieldType = Default::default();
        assert_eq!(default, IndexFieldType::Integer);
    }

    #[test]
    fn test_index_field_type_serialization() {
        assert_eq!(serde_json::to_string(&IndexFieldType::Integer).unwrap(), "\"integer\"");
        assert_eq!(serde_json::to_string(&IndexFieldType::String).unwrap(), "\"string\"");
        assert_eq!(serde_json::to_string(&IndexFieldType::UnsignedInteger).unwrap(), "\"unsignedinteger\"");
    }

    #[test]
    fn test_index_field_type_deserialization() {
        let int: IndexFieldType = serde_json::from_str("\"integer\"").unwrap();
        assert_eq!(int, IndexFieldType::Integer);

        let str: IndexFieldType = serde_json::from_str("\"string\"").unwrap();
        assert_eq!(str, IndexFieldType::String);
    }

    // =========================================================================
    // IndexOptions Tests
    // =========================================================================

    #[test]
    fn test_index_options_default() {
        let opts: IndexOptions = Default::default();
        assert!(!opts.unique);
        assert!(!opts.index_nulls);
    }

    #[test]
    fn test_index_options_serialization() {
        let opts = IndexOptions {
            unique: true,
            index_nulls: true,
        };

        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"unique\":true"));
        assert!(json.contains("\"index_nulls\":true"));
    }

    // =========================================================================
    // IndexError Tests
    // =========================================================================

    #[test]
    fn test_index_error_not_found_display() {
        let err = IndexError::NotFound {
            name: "missing_index".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("missing_index"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_index_error_too_many_indexes_display() {
        let err = IndexError::TooManyIndexes;
        let display = format!("{}", err);
        assert!(display.contains("too many"));
        assert!(display.contains("32"));
    }

    #[test]
    fn test_index_error_extraction_failed_display() {
        let err = IndexError::ExtractionFailed {
            name: "test_idx".to_string(),
            reason: "invalid format".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("test_idx"));
        assert!(display.contains("invalid format"));
    }

    #[test]
    fn test_index_error_unpack_failed_display() {
        let err = IndexError::UnpackFailed {
            reason: "corrupted key".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("unpack"));
        assert!(display.contains("corrupted key"));
    }

    // =========================================================================
    // IndexableEntry Tests
    // =========================================================================

    #[test]
    fn test_indexable_entry_debug() {
        let entry = IndexableEntry {
            value: "test-value".to_string(),
            version: 5,
            create_revision: 10,
            mod_revision: 20,
            expires_at_ms: Some(1234567890),
            lease_id: Some(42),
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("IndexableEntry"));
        assert!(debug.contains("test-value"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn test_indexable_entry_clone() {
        let entry = IndexableEntry {
            value: "original".to_string(),
            version: 1,
            create_revision: 5,
            mod_revision: 10,
            expires_at_ms: None,
            lease_id: None,
        };
        let cloned = entry.clone();
        assert_eq!(entry.value, cloned.value);
        assert_eq!(entry.version, cloned.version);
        assert_eq!(entry.create_revision, cloned.create_revision);
    }

    // =========================================================================
    // IndexRegistry Advanced Tests
    // =========================================================================

    #[test]
    fn test_registry_names() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        let names = registry.names();
        assert!(names.contains(&"idx_mod_revision"));
        assert!(names.contains(&"idx_create_revision"));
        assert!(names.contains(&"idx_expires_at"));
        assert!(names.contains(&"idx_lease_id"));
    }

    #[test]
    fn test_registry_definitions() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        let defs = registry.definitions();
        assert_eq!(defs.len(), 4);
    }

    #[test]
    fn test_registry_unregister_custom() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let mut registry = IndexRegistry::new();

        // Register a custom index
        let custom = SecondaryIndex::new(
            "custom_idx",
            idx_subspace.subspace(&Tuple::new().push("custom")),
            Arc::new(|_| Some(vec![1, 2, 3])),
        );
        registry.register(custom).unwrap();
        assert!(registry.get("custom_idx").is_some());

        // Unregister it
        let removed = registry.unregister("custom_idx");
        assert!(removed);
        assert!(registry.get("custom_idx").is_none());
    }

    #[test]
    fn test_registry_unregister_builtin_prevented() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let mut registry = IndexRegistry::with_builtins(idx_subspace);

        // Try to unregister a builtin - should fail
        let removed = registry.unregister("idx_mod_revision");
        assert!(!removed);
        assert!(registry.get("idx_mod_revision").is_some());
    }

    #[test]
    fn test_registry_iter() {
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let registry = IndexRegistry::with_builtins(idx_subspace);

        let count = registry.iter().count();
        assert_eq!(count, 4);
    }

    // =========================================================================
    // SecondaryIndex Advanced Tests
    // =========================================================================

    #[test]
    fn test_secondary_index_subspace() {
        let subspace = Subspace::new(Tuple::new().push("idx").push("test"));
        let index = SecondaryIndex::new("test", subspace.clone(), Arc::new(|_| Some(vec![])));

        assert_eq!(index.subspace().raw_prefix(), subspace.raw_prefix());
    }

    #[test]
    fn test_secondary_index_extract() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::new("test", subspace, Arc::new(|entry| Some(entry.value.as_bytes().to_vec())));

        let entry = IndexableEntry {
            value: "hello".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: None,
            lease_id: None,
        };

        let extracted = index.extract(&entry);
        assert_eq!(extracted, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_secondary_index_extract_returns_none() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::new(
            "optional",
            subspace,
            Arc::new(|entry| entry.expires_at_ms.map(|ms| ms.to_be_bytes().to_vec())),
        );

        let entry_without = IndexableEntry {
            value: "test".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: None,
            lease_id: None,
        };

        assert_eq!(index.extract(&entry_without), None);

        let entry_with = IndexableEntry {
            value: "test".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
            expires_at_ms: Some(1000),
            lease_id: None,
        };

        assert!(index.extract(&entry_with).is_some());
    }

    // =========================================================================
    // IndexUpdate Advanced Tests
    // =========================================================================

    #[test]
    fn test_index_update_operation_count() {
        let mut update = IndexUpdate::empty();
        assert_eq!(update.operation_count(), 0);

        update.inserts.push(vec![1, 2, 3]);
        assert_eq!(update.operation_count(), 1);

        update.deletes.push(vec![4, 5, 6]);
        assert_eq!(update.operation_count(), 2);

        update.inserts.push(vec![7, 8, 9]);
        assert_eq!(update.operation_count(), 3);
    }

    #[test]
    fn test_index_update_clone() {
        let mut update = IndexUpdate::empty();
        update.inserts.push(vec![1, 2, 3]);
        update.deletes.push(vec![4, 5, 6]);

        let cloned = update.clone();
        assert_eq!(update.inserts, cloned.inserts);
        assert_eq!(update.deletes, cloned.deletes);
    }

    #[test]
    fn test_index_update_debug() {
        let update = IndexUpdate {
            inserts: vec![vec![1, 2]],
            deletes: vec![vec![3, 4]],
        };
        let debug = format!("{:?}", update);
        assert!(debug.contains("IndexUpdate"));
        assert!(debug.contains("inserts"));
        assert!(debug.contains("deletes"));
    }

    // =========================================================================
    // IndexScanResult Advanced Tests
    // =========================================================================

    #[test]
    fn test_index_scan_result_has_more() {
        let result = IndexScanResult {
            primary_keys: vec![b"key1".to_vec()],
            has_more: true,
        };
        assert!(result.has_more);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_index_scan_result_clone() {
        let result = IndexScanResult {
            primary_keys: vec![b"key1".to_vec(), b"key2".to_vec()],
            has_more: false,
        };
        let cloned = result.clone();
        assert_eq!(result.primary_keys, cloned.primary_keys);
        assert_eq!(result.has_more, cloned.has_more);
    }

    // =========================================================================
    // extract_primary_key_from_tuple Edge Cases
    // =========================================================================

    #[test]
    fn test_extract_primary_key_empty_tuple() {
        let tuple = Tuple::new();
        let pk = extract_primary_key_from_tuple(&tuple);
        assert!(pk.is_none());
    }

    #[test]
    fn test_extract_primary_key_single_element() {
        let tuple = Tuple::new().push("only_element");
        let pk = extract_primary_key_from_tuple(&tuple);
        assert_eq!(pk, Some(b"only_element".to_vec()));
    }

    #[test]
    fn test_extract_primary_key_multiple_elements() {
        let tuple = Tuple::new().push("first").push("second").push("last");
        let pk = extract_primary_key_from_tuple(&tuple);
        assert_eq!(pk, Some(b"last".to_vec()));
    }

    #[test]
    fn test_extract_primary_key_integer_last() {
        // Integer as last element should return None (not bytes or string)
        let tuple = Tuple::new().push("indexed_value").push(12345i64);
        let pk = extract_primary_key_from_tuple(&tuple);
        assert!(pk.is_none()); // Integers don't match Bytes or String
    }

    // =========================================================================
    // Range Query Tests
    // =========================================================================

    #[test]
    fn test_range_between_non_numeric() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::new("string_idx", subspace, Arc::new(|_| Some(vec![])));

        let (start, end) = index.range_between(b"aaa", b"zzz");
        assert!(start < end);
    }

    #[test]
    fn test_range_between_numeric() {
        let subspace = Subspace::new(Tuple::new().push("idx"));
        let index = SecondaryIndex::numeric("num_idx", subspace, Arc::new(|_| Some(vec![])));

        let start_val = 100i64.to_be_bytes();
        let end_val = 200i64.to_be_bytes();

        let (start, end) = index.range_between(&start_val, &end_val);
        assert!(start < end);
    }

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn test_max_indexes_constant() {
        assert_eq!(MAX_INDEXES, 32);
    }

    #[test]
    fn test_max_index_scan_results_constant() {
        assert_eq!(MAX_INDEX_SCAN_RESULTS, 10_000);
    }

    #[test]
    fn test_index_metadata_prefix_constant() {
        assert_eq!(INDEX_METADATA_PREFIX, "/_sys/index/");
    }
}
