//! `IndexQueryExecutor` implementation for `SharedRedbStorage`.

use std::sync::Arc;

use super::*;

// ====================================================================================
// IndexQueryExecutor Implementation
// ====================================================================================

impl IndexQueryExecutor for SharedRedbStorage {
    /// Scan an index for a specific value.
    ///
    /// Returns primary keys of entries with the given indexed value.
    fn scan_by_index(&self, index_name: &str, value: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| aspen_core::layer::IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (start, end) = index.range_for_value(value);
        self.scan_index_range(&start, &end, limit)
    }

    /// Scan an index for a range of values.
    ///
    /// Returns primary keys of entries with indexed values in [start, end).
    fn range_by_index(&self, index_name: &str, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| aspen_core::layer::IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (range_start, range_end) = index.range_between(start, end);
        self.scan_index_range(&range_start, &range_end, limit)
    }

    /// Scan an index for values less than a threshold.
    ///
    /// Useful for TTL cleanup: find all keys expiring before a timestamp.
    fn scan_index_lt(&self, index_name: &str, threshold: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| aspen_core::layer::IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (range_start, range_end) = index.range_lt(threshold);
        self.scan_index_range(&range_start, &range_end, limit)
    }
}

impl SharedRedbStorage {
    /// Internal helper to scan an index range and extract primary keys.
    fn scan_index_range(&self, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        // Cap the limit to prevent resource exhaustion
        let effective_limit = limit.min(aspen_core::layer::MAX_INDEX_SCAN_RESULTS);

        let read_txn = self.db.begin_read().map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        let index_table =
            read_txn.open_table(SM_INDEX_TABLE).map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
                name: "scan".to_string(),
                reason: e.to_string(),
            })?;

        let mut primary_keys = Vec::new();
        let mut has_more = false;

        // Scan the range
        let range = index_table.range(start..end).map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        for item in range {
            let (key_guard, _) = item.map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
                name: "scan".to_string(),
                reason: e.to_string(),
            })?;

            if primary_keys.len() >= effective_limit as usize {
                has_more = true;
                break;
            }

            // Unpack the index key to extract the primary key
            let index_key = key_guard.value();
            if let Ok(tuple) = Tuple::unpack(index_key)
                && let Some(pk) = extract_primary_key_from_tuple(&tuple)
            {
                primary_keys.push(pk);
            }
        }

        Ok(IndexScanResult { primary_keys, has_more })
    }

    /// Get a reference to the index registry.
    ///
    /// This allows external code to access index metadata and create custom queries.
    pub fn index_registry(&self) -> &Arc<IndexRegistry> {
        &self.index_registry
    }
}
