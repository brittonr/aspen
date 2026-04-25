//! `IndexQueryExecutor` implementation for `RedbKvStorage`.

use aspen_layer::IndexError;
use aspen_layer::IndexQueryExecutor;
use aspen_layer::IndexScanResult;
use aspen_layer::IndexResult;
use aspen_layer::Tuple;
use aspen_layer::extract_primary_key_from_tuple;
use aspen_layer::MAX_INDEX_SCAN_RESULTS;

use super::RedbKvStorage;
use super::SM_INDEX_TABLE;

#[inline]
fn index_scan_limit_usize(limit_results: u32) -> usize {
    usize::try_from(limit_results).unwrap_or(usize::MAX)
}

impl IndexQueryExecutor for RedbKvStorage {
    fn scan_by_index(&self, index_name: &str, value: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (start, end) = index.range_for_value(value);
        self.scan_index_range(&start, &end, limit)
    }

    fn range_by_index(&self, index_name: &str, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (range_start, range_end) = index.range_between(start, end);
        self.scan_index_range(&range_start, &range_end, limit)
    }

    fn scan_index_lt(&self, index_name: &str, threshold: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (range_start, range_end) = index.range_lt(threshold);
        self.scan_index_range(&range_start, &range_end, limit)
    }
}

impl RedbKvStorage {
    fn scan_index_range(&self, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let max_results = limit.min(MAX_INDEX_SCAN_RESULTS);
        let max_result_slots = index_scan_limit_usize(max_results);

        let read_txn = self.db.begin_read().map_err(|e| IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        let index_table = read_txn.open_table(SM_INDEX_TABLE).map_err(|e| IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        let mut primary_keys = Vec::with_capacity(max_result_slots);
        let mut has_more = false;

        let range = index_table.range(start..end).map_err(|e| IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        for item in range {
            let (key_guard, _) = item.map_err(|e| IndexError::ExtractionFailed {
                name: "scan".to_string(),
                reason: e.to_string(),
            })?;

            if primary_keys.len() >= max_result_slots {
                has_more = true;
                break;
            }

            let index_key = key_guard.value();
            if let Ok(tuple) = Tuple::unpack(index_key)
                && let Some(pk) = extract_primary_key_from_tuple(&tuple)
            {
                primary_keys.push(pk);
            }
        }

        Ok(IndexScanResult { primary_keys, has_more })
    }
}
