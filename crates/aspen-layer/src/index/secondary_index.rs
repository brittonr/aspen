//! Secondary index definition and key extraction types.

use std::sync::Arc;

use crate::subspace::Subspace;
use crate::tuple::Tuple;

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
