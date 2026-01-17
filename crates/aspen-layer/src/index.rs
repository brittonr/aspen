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

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use snafu::Snafu;

use crate::subspace::Subspace;
use crate::tuple::Element;
use crate::tuple::Tuple;

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

// =============================================================================
// IndexDefinition (Serializable)
// =============================================================================

/// Serializable index definition for persistence.
///
/// This struct captures the metadata needed to recreate an index, including
/// its name, type, and extraction field. It can be stored in the system
/// namespace and loaded on startup to restore custom indexes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexDefinition {
    /// Unique name for this index (e.g., "idx_mod_revision").
    pub name: String,

    /// Type of field being indexed.
    pub field_type: IndexFieldType,

    /// Name of the built-in field to index (for builtin types).
    /// Options: "mod_revision", "create_revision", "expires_at_ms", "lease_id"
    pub field: Option<String>,

    /// Whether this is a built-in index that's always present.
    pub builtin: bool,

    /// Index options.
    #[serde(default)]
    pub options: IndexOptions,
}

/// Type of the indexed field.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum IndexFieldType {
    /// 64-bit integer field (mod_revision, create_revision, etc.)
    #[default]
    Integer,
    /// String field from value (future: JSON path extraction)
    String,
    /// Unsigned 64-bit integer (expires_at_ms)
    UnsignedInteger,
}

/// Additional index configuration options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IndexOptions {
    /// Whether null values should be indexed.
    #[serde(default)]
    pub index_nulls: bool,

    /// Whether this index enforces uniqueness.
    #[serde(default)]
    pub unique: bool,
}

impl IndexDefinition {
    /// Create a new index definition for a built-in field.
    pub fn builtin(name: &str, field: &str, field_type: IndexFieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            field: Some(field.to_string()),
            builtin: true,
            options: IndexOptions::default(),
        }
    }

    /// Create a new custom index definition.
    pub fn custom(name: &str, field: &str, field_type: IndexFieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            field: Some(field.to_string()),
            builtin: false,
            options: IndexOptions::default(),
        }
    }

    /// Set the unique option.
    pub fn with_unique(mut self, unique: bool) -> Self {
        self.options.unique = unique;
        self
    }

    /// Set the index_nulls option.
    pub fn with_index_nulls(mut self, index_nulls: bool) -> Self {
        self.options.index_nulls = index_nulls;
        self
    }

    /// Get the system key for storing this index definition.
    pub fn system_key(&self) -> String {
        format!("{}{}", INDEX_METADATA_PREFIX, self.name)
    }

    /// Create builtin index definitions.
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::builtin("idx_mod_revision", "mod_revision", IndexFieldType::Integer),
            Self::builtin("idx_create_revision", "create_revision", IndexFieldType::Integer),
            Self::builtin("idx_expires_at", "expires_at_ms", IndexFieldType::UnsignedInteger),
            Self::builtin("idx_lease_id", "lease_id", IndexFieldType::UnsignedInteger),
        ]
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during index operations.
#[derive(Debug, Snafu)]
pub enum IndexError {
    /// Index not found in registry.
    #[snafu(display("index not found: {name}"))]
    NotFound {
        /// Name of the missing index.
        name: String,
    },

    /// Too many indexes registered.
    #[snafu(display("too many indexes: max is {}", MAX_INDEXES))]
    TooManyIndexes,

    /// Failed to extract index key from entry.
    #[snafu(display("failed to extract index key for index {name}: {reason}"))]
    ExtractionFailed {
        /// Name of the index.
        name: String,
        /// Reason for the failure.
        reason: String,
    },

    /// Failed to unpack index key.
    #[snafu(display("failed to unpack index key: {reason}"))]
    UnpackFailed {
        /// Reason for the failure.
        reason: String,
    },
}

/// Result type for index operations.
pub type IndexResult<T> = Result<T, IndexError>;

// =============================================================================
// Key Extractor
// =============================================================================

/// A key extractor function that derives an indexable value from a KvEntry.
///
/// Returns `None` if the entry should not be indexed (e.g., null field).
///
/// The extractor receives:
/// - `value`: The entry's value string
/// - `version`: Per-key version counter
/// - `create_revision`: Raft log index when first created
/// - `mod_revision`: Raft log index of last modification
/// - `expires_at_ms`: Optional expiration timestamp
/// - `lease_id`: Optional lease ID
///
/// This trait object approach avoids coupling to aspen-core's KvEntry directly.
pub type KeyExtractor = Arc<dyn Fn(&IndexableEntry) -> Option<Vec<u8>> + Send + Sync>;

/// Fields from a KvEntry needed for index extraction.
///
/// This avoids a direct dependency on aspen-core::KvEntry, allowing the
/// layer crate to remain independent.
#[derive(Debug, Clone)]
pub struct IndexableEntry {
    /// The value stored for this key.
    pub value: String,
    /// Per-key version counter (1, 2, 3...).
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

// =============================================================================
// SecondaryIndex
// =============================================================================

/// A secondary index definition.
///
/// Indexes store entries in the format:
/// `(index_subspace, indexed_value, primary_key) -> ()`
///
/// This enables efficient range scans on indexed values.
pub struct SecondaryIndex {
    /// Unique name for this index.
    name: String,
    /// Subspace where index entries are stored.
    subspace: Subspace,
    /// Function to extract the indexed value from an entry.
    extractor: KeyExtractor,
    /// Whether this index stores integer values (for proper ordering).
    is_numeric: bool,
}

impl SecondaryIndex {
    /// Create a new secondary index.
    ///
    /// # Arguments
    /// * `name` - Unique identifier for this index
    /// * `subspace` - Subspace for storing index entries
    /// * `extractor` - Function to extract indexed value from entry
    pub fn new(name: impl Into<String>, subspace: Subspace, extractor: KeyExtractor) -> Self {
        Self {
            name: name.into(),
            subspace,
            extractor,
            is_numeric: false,
        }
    }

    /// Create a numeric index (preserves integer ordering).
    ///
    /// Numeric indexes encode integer values using the tuple layer's
    /// order-preserving integer encoding.
    pub fn numeric(name: impl Into<String>, subspace: Subspace, extractor: KeyExtractor) -> Self {
        Self {
            name: name.into(),
            subspace,
            extractor,
            is_numeric: true,
        }
    }

    /// Get the index name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the index subspace.
    pub fn subspace(&self) -> &Subspace {
        &self.subspace
    }

    /// Check if this is a numeric index.
    pub fn is_numeric(&self) -> bool {
        self.is_numeric
    }

    /// Extract the indexed value from an entry.
    ///
    /// Returns `None` if the entry should not be indexed.
    pub fn extract(&self, entry: &IndexableEntry) -> Option<Vec<u8>> {
        (self.extractor)(entry)
    }

    /// Build the index entry key for a given indexed value and primary key.
    ///
    /// Format: (index_subspace, indexed_value, primary_key)
    pub fn build_key(&self, indexed_value: &[u8], primary_key: &[u8]) -> Vec<u8> {
        if self.is_numeric && indexed_value.len() == 8 {
            // Decode as i64 for proper tuple ordering
            let bytes: [u8; 8] = indexed_value.try_into().unwrap_or([0; 8]);
            let value = i64::from_be_bytes(bytes);
            self.subspace.pack(&Tuple::new().push(value).push(primary_key))
        } else {
            self.subspace.pack(&Tuple::new().push(indexed_value).push(primary_key))
        }
    }

    /// Get the range for scanning all entries with a specific indexed value.
    pub fn range_for_value(&self, indexed_value: &[u8]) -> (Vec<u8>, Vec<u8>) {
        if self.is_numeric && indexed_value.len() == 8 {
            let bytes: [u8; 8] = indexed_value.try_into().unwrap_or([0; 8]);
            let value = i64::from_be_bytes(bytes);
            self.subspace.range_of(&Tuple::new().push(value))
        } else {
            self.subspace.range_of(&Tuple::new().push(indexed_value))
        }
    }

    /// Get the range for scanning entries in a value range.
    ///
    /// Returns `(start_key, end_key)` suitable for range scans.
    pub fn range_between(&self, start: &[u8], end: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let start_key = if self.is_numeric && start.len() == 8 {
            let bytes: [u8; 8] = start.try_into().unwrap_or([0; 8]);
            let value = i64::from_be_bytes(bytes);
            self.subspace.pack(&Tuple::new().push(value))
        } else {
            self.subspace.pack(&Tuple::new().push(start))
        };

        let end_key = if self.is_numeric && end.len() == 8 {
            let bytes: [u8; 8] = end.try_into().unwrap_or([0; 8]);
            let value = i64::from_be_bytes(bytes);
            self.subspace.pack(&Tuple::new().push(value))
        } else {
            self.subspace.pack(&Tuple::new().push(end))
        };

        (start_key, end_key)
    }

    /// Get the range for scanning entries less than a threshold.
    ///
    /// Useful for TTL cleanup: find all keys expiring before a timestamp.
    pub fn range_lt(&self, threshold: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let (start, _) = self.subspace.range();
        let end = if self.is_numeric && threshold.len() == 8 {
            let bytes: [u8; 8] = threshold.try_into().unwrap_or([0; 8]);
            let value = i64::from_be_bytes(bytes);
            self.subspace.pack(&Tuple::new().push(value))
        } else {
            self.subspace.pack(&Tuple::new().push(threshold))
        };
        (start, end)
    }
}

// =============================================================================
// IndexRegistry
// =============================================================================

/// Registry for managing multiple secondary indexes.
///
/// The registry provides a centralized place to define and access indexes,
/// and generates the operations needed to update all indexes during writes.
pub struct IndexRegistry {
    /// Registered indexes by name.
    indexes: BTreeMap<String, SecondaryIndex>,
}

impl IndexRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            indexes: BTreeMap::new(),
        }
    }

    /// Create a registry with the built-in indexes.
    ///
    /// Built-in indexes:
    /// - `idx_mod_revision`: Query by modification revision
    /// - `idx_create_revision`: Query by creation revision
    /// - `idx_expires_at`: Query by expiration time (for TTL cleanup)
    /// - `idx_lease_id`: Query keys by lease
    ///
    /// Note: Registration of builtin indexes cannot fail because MAX_INDEXES > 4
    /// and the registry starts empty. Errors are silently ignored for Tiger Style
    /// compliance (no panics in initialization code).
    pub fn with_builtins(index_subspace: Subspace) -> Self {
        let mut registry = Self::new();

        // idx_mod_revision: Query by modification revision
        // Note: Registration cannot fail as we're adding to an empty registry with capacity > 4
        let mod_rev_space = index_subspace.subspace(&Tuple::new().push("mod_revision"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_mod_revision",
            mod_rev_space,
            Arc::new(|entry| Some(entry.mod_revision.to_be_bytes().to_vec())),
        ));

        // idx_create_revision: Query by creation revision
        let create_rev_space = index_subspace.subspace(&Tuple::new().push("create_revision"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_create_revision",
            create_rev_space,
            Arc::new(|entry| Some(entry.create_revision.to_be_bytes().to_vec())),
        ));

        // idx_expires_at: Query by expiration time (for TTL cleanup)
        let expires_space = index_subspace.subspace(&Tuple::new().push("expires_at"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_expires_at",
            expires_space,
            Arc::new(|entry| entry.expires_at_ms.map(|ms| (ms as i64).to_be_bytes().to_vec())),
        ));

        // idx_lease_id: Query keys by lease
        let lease_space = index_subspace.subspace(&Tuple::new().push("lease_id"));
        let _ = registry.register(SecondaryIndex::numeric(
            "idx_lease_id",
            lease_space,
            Arc::new(|entry| entry.lease_id.map(|id| (id as i64).to_be_bytes().to_vec())),
        ));

        registry
    }

    /// Register a new index.
    ///
    /// # Errors
    ///
    /// Returns `IndexError::TooManyIndexes` if the registry is at capacity.
    pub fn register(&mut self, index: SecondaryIndex) -> IndexResult<()> {
        if self.indexes.len() >= MAX_INDEXES as usize {
            return Err(IndexError::TooManyIndexes);
        }
        self.indexes.insert(index.name.clone(), index);
        Ok(())
    }

    /// Get an index by name.
    pub fn get(&self, name: &str) -> Option<&SecondaryIndex> {
        self.indexes.get(name)
    }

    /// Iterate over all indexes.
    pub fn iter(&self) -> impl Iterator<Item = &SecondaryIndex> {
        self.indexes.values()
    }

    /// Get the number of registered indexes.
    pub fn len(&self) -> usize {
        self.indexes.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.indexes.is_empty()
    }

    /// Get all index names.
    pub fn names(&self) -> Vec<&str> {
        self.indexes.keys().map(|s| s.as_str()).collect()
    }

    /// Get definitions for all registered indexes.
    ///
    /// This can be used to persist index metadata and recreate
    /// indexes on startup.
    pub fn definitions(&self) -> Vec<IndexDefinition> {
        // Return definitions for the builtin indexes
        // Custom indexes would need to be tracked separately
        IndexDefinition::builtins()
    }

    /// Remove an index by name.
    ///
    /// Note: Built-in indexes cannot be removed; this is a no-op for them.
    pub fn unregister(&mut self, name: &str) -> bool {
        // Check if it's a builtin (don't allow removal)
        let is_builtin = matches!(name, "idx_mod_revision" | "idx_create_revision" | "idx_expires_at" | "idx_lease_id");

        if is_builtin {
            return false;
        }

        self.indexes.remove(name).is_some()
    }
}

impl Default for IndexRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// IndexUpdate
// =============================================================================

/// Operations to update indexes during writes.
///
/// These are generated by the IndexRegistry and applied within the same
/// transaction as the primary KV write.
#[derive(Debug, Clone, Default)]
pub struct IndexUpdate {
    /// Index entry keys to insert (key -> empty value).
    pub inserts: Vec<Vec<u8>>,
    /// Index entry keys to delete.
    pub deletes: Vec<Vec<u8>>,
}

impl IndexUpdate {
    /// Create an empty update.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if the update has any operations.
    pub fn is_empty(&self) -> bool {
        self.inserts.is_empty() && self.deletes.is_empty()
    }

    /// Get total number of operations.
    pub fn operation_count(&self) -> usize {
        self.inserts.len() + self.deletes.len()
    }
}

impl IndexRegistry {
    /// Generate index updates for a Set operation.
    ///
    /// # Arguments
    /// * `primary_key` - The primary key being set
    /// * `old_entry` - Previous entry (if any) - needed to remove old index entries
    /// * `new_entry` - New entry being written
    pub fn updates_for_set(
        &self,
        primary_key: &[u8],
        old_entry: Option<&IndexableEntry>,
        new_entry: &IndexableEntry,
    ) -> IndexUpdate {
        let mut update = IndexUpdate::empty();

        for index in self.indexes.values() {
            // Delete old index entry if it existed and had a value
            if let Some(old) = old_entry
                && let Some(old_value) = index.extract(old)
            {
                update.deletes.push(index.build_key(&old_value, primary_key));
            }

            // Insert new index entry if the new entry has a value
            if let Some(new_value) = index.extract(new_entry) {
                update.inserts.push(index.build_key(&new_value, primary_key));
            }
        }

        update
    }

    /// Generate index updates for a Delete operation.
    ///
    /// # Arguments
    /// * `primary_key` - The primary key being deleted
    /// * `old_entry` - The entry being deleted
    pub fn updates_for_delete(&self, primary_key: &[u8], old_entry: &IndexableEntry) -> IndexUpdate {
        let mut update = IndexUpdate::empty();

        for index in self.indexes.values() {
            if let Some(old_value) = index.extract(old_entry) {
                update.deletes.push(index.build_key(&old_value, primary_key));
            }
        }

        update
    }
}

// =============================================================================
// IndexScanResult
// =============================================================================

/// Result of an index scan.
#[derive(Debug, Clone, Default)]
pub struct IndexScanResult {
    /// Primary keys matching the index query.
    pub primary_keys: Vec<Vec<u8>>,
    /// Whether more results are available (for pagination).
    pub has_more: bool,
}

impl IndexScanResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.primary_keys.len()
    }

    /// Check if the result is empty.
    pub fn is_empty(&self) -> bool {
        self.primary_keys.is_empty()
    }
}

// =============================================================================
// IndexQueryExecutor Trait
// =============================================================================

/// Trait for index query execution.
///
/// Implemented by storage backends that support secondary indexes.
pub trait IndexQueryExecutor {
    /// Scan an index for a specific value.
    ///
    /// Returns primary keys of entries with the given indexed value.
    fn scan_by_index(&self, index_name: &str, value: &[u8], limit: u32) -> IndexResult<IndexScanResult>;

    /// Scan an index for a range of values.
    ///
    /// Returns primary keys of entries with indexed values in [start, end).
    fn range_by_index(&self, index_name: &str, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult>;

    /// Scan an index for values less than a threshold.
    ///
    /// Useful for TTL cleanup: find all keys expiring before a timestamp.
    fn scan_index_lt(&self, index_name: &str, threshold: &[u8], limit: u32) -> IndexResult<IndexScanResult>;
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract the primary key from an index entry key.
///
/// Index keys have format: (index_subspace, indexed_value, primary_key)
/// This function extracts the primary key from the unpacked tuple.
pub fn extract_primary_key_from_tuple(tuple: &Tuple) -> Option<Vec<u8>> {
    // Primary key is the last element
    let len = tuple.len();
    if len < 1 {
        return None;
    }

    match tuple.get(len - 1) {
        Some(Element::Bytes(pk)) => Some(pk.clone()),
        Some(Element::String(pk)) => Some(pk.as_bytes().to_vec()),
        _ => None,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
