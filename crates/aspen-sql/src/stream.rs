//! RecordBatch streaming from Redb storage.
//!
//! This module provides a streaming iterator over Redb KV entries,
//! converting them to Arrow RecordBatches for DataFusion consumption.

use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use arrow::array::ArrayRef;
use arrow::array::Int64Builder;
use arrow::array::StringBuilder;
use arrow::array::UInt64Builder;
use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use aspen_core::storage::KvEntry;
use aspen_core::storage::SM_KV_TABLE;
use datafusion::error::DataFusionError;
use datafusion::physical_plan::RecordBatchStream;
use futures::Stream;
use redb::Database;

use super::error::SqlError;
use super::schema::KV_SCHEMA;

/// Default batch size for streaming RecordBatches.
pub const DEFAULT_BATCH_SIZE: usize = 8192;

/// A stream that reads from Redb and produces Arrow RecordBatches.
///
/// This stream performs incremental reads from the database, converting
/// entries to Arrow format in batches to avoid memory exhaustion.
///
/// # Empty Batch Handling
///
/// For compatibility with DataFusion aggregation operators, this stream
/// will return an empty batch (0 rows, correct schema) on the first poll
/// if no data is found, rather than immediately returning `None`. This
/// ensures aggregation queries like `COUNT(*)` work correctly even on
/// empty tables or when filters match no rows.
pub struct RedbRecordBatchStream {
    /// Reference to the Redb database.
    db: Arc<Database>,
    /// Schema for the output batches.
    schema: SchemaRef,
    /// Optional projection (column indices to include).
    projection: Option<Vec<usize>>,
    /// Key range start (inclusive). Empty means scan from beginning.
    start_key: Vec<u8>,
    /// Key range end (exclusive). Empty means scan to end.
    end_key: Vec<u8>,
    /// Maximum rows to return (None = unlimited up to Tiger Style bounds).
    limit: Option<usize>,
    /// Batch size for streaming.
    batch_size: usize,
    /// Total rows returned so far.
    rows_returned: usize,
    /// Last key read (for continuation).
    last_key: Option<Vec<u8>>,
    /// Whether we've finished scanning.
    done: bool,
    /// Current timestamp for TTL filtering.
    now_ms: u64,
    /// Whether we've returned at least one batch (even if empty).
    /// Required for DataFusion aggregations to work on empty results.
    returned_first_batch: bool,
}

impl RedbRecordBatchStream {
    /// Create a new stream over the KV table.
    pub fn new(
        db: Arc<Database>,
        projection: Option<Vec<usize>>,
        start_key: Vec<u8>,
        end_key: Vec<u8>,
        limit: Option<usize>,
    ) -> Self {
        // Compute the output schema based on projection.
        // For empty projection (e.g., COUNT(*)), we produce batches with no columns
        // but with a row count that DataFusion can use for aggregation.
        let schema = match &projection {
            Some(indices) if indices.is_empty() => Arc::new(arrow::datatypes::Schema::empty()),
            Some(indices) => Arc::new(KV_SCHEMA.project(indices).expect("valid projection")),
            None => KV_SCHEMA.clone(),
        };

        Self {
            db,
            schema,
            projection,
            start_key,
            end_key,
            limit,
            batch_size: DEFAULT_BATCH_SIZE,
            rows_returned: 0,
            last_key: None,
            done: false,
            now_ms: crate::now_unix_ms(),
            returned_first_batch: false,
        }
    }

    /// Set the batch size for streaming.
    #[allow(dead_code)] // May be used for future tuning
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Read the next batch of records from Redb.
    fn read_next_batch(&mut self) -> Result<Option<RecordBatch>, SqlError> {
        if self.done {
            return Ok(None);
        }

        // Check if we've hit the limit
        if let Some(limit) = self.limit
            && self.rows_returned >= limit
        {
            self.done = true;
            return Ok(None);
        }

        // Calculate how many rows to fetch this batch
        let rows_to_fetch = match self.limit {
            Some(limit) => self.batch_size.min(limit - self.rows_returned),
            None => self.batch_size,
        };

        // Build arrays for each column
        let mut key_builder = StringBuilder::new();
        let mut value_builder = StringBuilder::new();
        let mut version_builder = Int64Builder::new();
        let mut create_rev_builder = Int64Builder::new();
        let mut mod_rev_builder = Int64Builder::new();
        let mut expires_builder = UInt64Builder::new();
        let mut lease_builder = UInt64Builder::new();

        // Read from database
        let read_txn = self.db.begin_read().map_err(|e| SqlError::BeginRead { source: Box::new(e) })?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| SqlError::OpenTable { source: Box::new(e) })?;

        // Determine the actual start key for this batch
        let scan_start = match &self.last_key {
            Some(last) => {
                // Start after the last key we read
                let mut next = last.clone();
                next.push(0); // Ensure we skip the last key
                next
            }
            None => self.start_key.clone(),
        };

        let mut rows_in_batch = 0;
        let mut last_key_in_batch: Option<Vec<u8>> = None;

        // Perform the range scan
        let range = if self.end_key.is_empty() {
            table.range(scan_start.as_slice()..)
        } else {
            table.range(scan_start.as_slice()..self.end_key.as_slice())
        };

        let iter = match range {
            Ok(iter) => iter,
            Err(e) => {
                return Err(SqlError::StorageRead { source: Box::new(e) });
            }
        };

        for result in iter {
            if rows_in_batch >= rows_to_fetch {
                break;
            }

            let (key_guard, value_guard) = match result {
                Ok((k, v)) => (k, v),
                Err(e) => {
                    return Err(SqlError::StorageRead { source: Box::new(e) });
                }
            };

            let key_bytes = key_guard.value();
            let value_bytes = value_guard.value();

            // Deserialize the entry
            let entry: KvEntry = match bincode::deserialize(value_bytes) {
                Ok(e) => e,
                Err(_) => continue, // Skip malformed entries
            };

            // Check expiration
            if let Some(expires_at) = entry.expires_at_ms
                && self.now_ms > expires_at
            {
                continue; // Skip expired entries
            }

            // Convert key to string
            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue, // Skip non-UTF8 keys
            };

            // Append to arrays
            key_builder.append_value(key_str);
            value_builder.append_value(&entry.value);
            version_builder.append_value(entry.version);
            create_rev_builder.append_value(entry.create_revision);
            mod_rev_builder.append_value(entry.mod_revision);

            if let Some(expires) = entry.expires_at_ms {
                expires_builder.append_value(expires);
            } else {
                expires_builder.append_null();
            }

            if let Some(lease) = entry.lease_id {
                lease_builder.append_value(lease);
            } else {
                lease_builder.append_null();
            }

            last_key_in_batch = Some(key_bytes.to_vec());
            rows_in_batch += 1;
        }

        // Update state
        if rows_in_batch == 0 {
            self.done = true;
            // For DataFusion aggregation compatibility, return an empty batch
            // with the correct schema on the first poll. This allows COUNT(*)
            // and other aggregations to work correctly on empty tables or
            // when filters match no rows.
            if !self.returned_first_batch {
                self.returned_first_batch = true;
                return self.create_empty_batch();
            }
            return Ok(None);
        }

        self.rows_returned += rows_in_batch;
        self.last_key = last_key_in_batch;

        // If we got fewer rows than requested, we're done
        if rows_in_batch < rows_to_fetch {
            self.done = true;
        }

        // Handle empty projection (COUNT(*) case) - produce batch with row count only
        let is_empty_projection = matches!(&self.projection, Some(indices) if indices.is_empty());

        if is_empty_projection {
            // For COUNT(*), we don't need columns, just the row count.
            // Create a batch with no columns but with the correct row count.
            self.returned_first_batch = true;
            let batch = RecordBatch::try_new_with_options(
                self.schema.clone(),
                vec![],
                &arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(rows_in_batch)),
            )?;
            return Ok(Some(batch));
        }

        // Build the arrays for normal projections
        let all_arrays: Vec<ArrayRef> = vec![
            Arc::new(key_builder.finish()),
            Arc::new(value_builder.finish()),
            Arc::new(version_builder.finish()),
            Arc::new(create_rev_builder.finish()),
            Arc::new(mod_rev_builder.finish()),
            Arc::new(expires_builder.finish()),
            Arc::new(lease_builder.finish()),
        ];

        // Apply projection if specified
        let arrays: Vec<ArrayRef> = match &self.projection {
            Some(indices) => indices.iter().map(|&i| all_arrays[i].clone()).collect(),
            None => all_arrays,
        };

        self.returned_first_batch = true;
        let batch = RecordBatch::try_new(self.schema.clone(), arrays)?;
        Ok(Some(batch))
    }

    /// Create an empty RecordBatch with the correct schema.
    ///
    /// This is used for DataFusion aggregation compatibility - aggregations
    /// like COUNT(*) need at least one batch (even if empty) to process.
    fn create_empty_batch(&self) -> Result<Option<RecordBatch>, SqlError> {
        // Handle empty projection (COUNT(*) case)
        let is_empty_projection = matches!(&self.projection, Some(indices) if indices.is_empty());

        if is_empty_projection {
            // For COUNT(*), produce a batch with 0 columns and 0 rows
            let batch = RecordBatch::try_new_with_options(
                self.schema.clone(),
                vec![],
                &arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(0)),
            )?;
            return Ok(Some(batch));
        }

        // Create empty arrays for each column in the projected schema
        let arrays: Vec<ArrayRef> =
            self.schema.fields().iter().map(|field| arrow::array::new_empty_array(field.data_type())).collect();

        let batch = RecordBatch::try_new(self.schema.clone(), arrays)?;
        Ok(Some(batch))
    }
}

impl RecordBatchStream for RedbRecordBatchStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

impl Stream for RedbRecordBatchStream {
    type Item = Result<RecordBatch, DataFusionError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.read_next_batch() {
            Ok(Some(batch)) => Poll::Ready(Some(Ok(batch))),
            Ok(None) => Poll::Ready(None),
            Err(e) => Poll::Ready(Some(Err(DataFusionError::External(Box::new(e))))),
        }
    }
}

/// Create a full table scan stream.
pub fn full_scan_stream(
    db: Arc<Database>,
    projection: Option<Vec<usize>>,
    limit: Option<usize>,
) -> RedbRecordBatchStream {
    RedbRecordBatchStream::new(db, projection, Vec::new(), Vec::new(), limit)
}

/// Create a prefix scan stream.
///
/// Scans all keys that start with the given prefix.
pub fn prefix_scan_stream(
    db: Arc<Database>,
    prefix: &[u8],
    projection: Option<Vec<usize>>,
    limit: Option<usize>,
) -> RedbRecordBatchStream {
    // Calculate end key using strinc (FoundationDB pattern)
    let end_key = strinc(prefix);
    RedbRecordBatchStream::new(db, projection, prefix.to_vec(), end_key.unwrap_or_default(), limit)
}

/// Create a range scan stream.
///
/// Scans all keys in the range [start, end).
pub fn range_scan_stream(
    db: Arc<Database>,
    start: &[u8],
    end: &[u8],
    projection: Option<Vec<usize>>,
    limit: Option<usize>,
) -> RedbRecordBatchStream {
    RedbRecordBatchStream::new(db, projection, start.to_vec(), end.to_vec(), limit)
}

/// Compute the strict upper bound for a byte string (FoundationDB strinc).
///
/// Returns the lexicographically smallest byte string that is greater than
/// any string with the given prefix. Returns None if all bytes are 0xFF.
fn strinc(key: &[u8]) -> Option<Vec<u8>> {
    // Find the last byte that isn't 0xFF
    for i in (0..key.len()).rev() {
        if key[i] != 0xFF {
            let mut result = key[..=i].to_vec();
            result[i] += 1;
            return Some(result);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strinc_basic() {
        assert_eq!(strinc(b"abc"), Some(b"abd".to_vec()));
        assert_eq!(strinc(b"ab\xff"), Some(b"ac".to_vec()));
        assert_eq!(strinc(b"\xff\xff\xff"), None);
        assert_eq!(strinc(b""), None);
    }

    #[test]
    fn strinc_prefix_bound() {
        // prefix "test" should have upper bound "tesu"
        let prefix = b"test";
        let upper = strinc(prefix).unwrap();
        assert_eq!(upper, b"tesu");

        // "test\x00" < "tesu" (any key with prefix "test" is less than upper bound)
        assert!(prefix.as_slice() < upper.as_slice());
        assert!(b"test\x00".as_slice() < upper.as_slice());
        assert!(b"test\xff\xff".as_slice() < upper.as_slice());
    }
}
