//! Index scan result types and query execution trait.

use super::errors::IndexResult;
use crate::tuple::Element;
use crate::tuple::Tuple;

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
