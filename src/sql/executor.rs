//! SQL query executor for Redb storage.
//!
//! This module provides the `RedbSqlExecutor` which executes SQL queries
//! against the Redb KV storage using DataFusion.
//!
//! ## Performance Optimizations
//!
//! 1. **SessionContext Caching**: The DataFusion `SessionContext` is cached and reused across
//!    queries, avoiding the ~400µs overhead of creating a new context per query. The
//!    `RedbTableProvider` is registered once at construction time.
//!
//! 2. **Simple Query Bypass**: Common query patterns are detected via regex and executed directly
//!    against Redb, bypassing DataFusion entirely:
//!    - Point lookups: `SELECT * FROM kv WHERE key = 'literal'`
//!    - Prefix scans: `SELECT * FROM kv WHERE key LIKE 'prefix%'`
//!    - Count queries: `SELECT COUNT(*) FROM kv`
//!
//!    This optimization reduces point lookup latency from ~300µs to ~10µs.

use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Instant;

use datafusion::prelude::*;
use redb::Database;
use redb::ReadableTable;
use regex::Regex;

use super::provider::RedbTableProvider;
use crate::api::SqlColumnInfo;
use crate::api::SqlQueryError;
use crate::api::SqlQueryResult;
use crate::api::SqlValue;
use crate::api::effective_sql_limit;
use crate::api::effective_sql_timeout_ms;
use crate::raft::storage_shared::SM_KV_TABLE;

/// Regex for point lookup: SELECT * FROM kv WHERE key = 'literal'
/// Captures the key value (group 1 for single quotes, group 2 for double quotes)
static POINT_LOOKUP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^\s*SELECT\s+\*\s+FROM\s+kv\s+WHERE\s+key\s*=\s*(?:'([^']*)'|"([^"]*)")\s*$"#)
        .expect("invalid point lookup regex")
});

/// Regex for prefix scan: SELECT * FROM kv WHERE key LIKE 'prefix%'
/// Captures the prefix (without the trailing %)
static PREFIX_SCAN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^\s*SELECT\s+\*\s+FROM\s+kv\s+WHERE\s+key\s+LIKE\s+(?:'([^']*?)%'|"([^"]*?)%")\s*$"#)
        .expect("invalid prefix scan regex")
});

/// Regex for count all: SELECT COUNT(*) FROM kv
static COUNT_ALL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^\s*SELECT\s+COUNT\s*\(\s*\*\s*\)\s+FROM\s+kv\s*$"#).expect("invalid count regex")
});

/// SQL query executor for Redb storage.
///
/// Wraps a Redb database and provides SQL query execution via DataFusion.
/// Queries are executed against a virtual `kv` table that exposes the
/// key-value data.
///
/// ## Performance
///
/// The executor implements two levels of optimization:
///
/// 1. **Simple Query Bypass**: Point lookups, prefix scans, and COUNT(*) queries are detected via
///    regex and executed directly against Redb, bypassing DataFusion entirely. This reduces latency
///    from ~300µs to ~10µs.
///
/// 2. **SessionContext Caching**: For complex queries that do go through DataFusion, the
///    `SessionContext` is cached and reused, eliminating:
///    - Creating a new SessionContext (~200µs)
///    - Registering the TableProvider (~100µs)
///    - DataFusion optimizer initialization (~100µs)
///
/// Thread Safety: Both `Database` and `SessionContext` are `Send + Sync`,
/// so this executor can be safely shared across async tasks.
///
/// # Example
///
/// ```ignore
/// // Create once, reuse for all queries
/// let executor = RedbSqlExecutor::new(db);
///
/// // Point lookups bypass DataFusion entirely (~10µs)
/// let result = executor.execute(
///     "SELECT * FROM kv WHERE key = 'user:123'",
///     &[],
///     Some(100),
///     Some(5000),
/// ).await?;
///
/// // Complex queries use cached DataFusion context (~300µs)
/// let result = executor.execute(
///     "SELECT key, value FROM kv WHERE key LIKE 'user:%' ORDER BY key",
///     &[],
///     Some(100),
///     Some(5000),
/// ).await?;
/// ```
pub struct RedbSqlExecutor {
    /// Database reference for direct Redb access (bypassing DataFusion).
    db: Arc<Database>,
    /// Cached DataFusion session context (reused across queries).
    ctx: SessionContext,
}

impl RedbSqlExecutor {
    /// Create a new SQL executor for the given database.
    ///
    /// This creates and caches the DataFusion SessionContext with optimal
    /// settings for single-partition Redb queries. The `kv` table is
    /// registered once at construction time. The database reference is
    /// also stored for direct access in simple query bypass paths.
    pub fn new(db: Arc<Database>) -> Self {
        // Configure session for optimal Redb performance:
        // - Single partition (Redb is single-threaded for reads)
        // - 8192 batch size (good balance for streaming)
        // - Statistics collection disabled (we don't need it for simple KV queries)
        let config =
            SessionConfig::new().with_target_partitions(1).with_batch_size(8192).with_information_schema(false);

        let ctx = SessionContext::new_with_config(config);

        // Register the KV table ONCE at construction time
        let provider = RedbTableProvider::new(db.clone());
        ctx.register_table("kv", Arc::new(provider)).expect("failed to register kv table");

        Self { db, ctx }
    }

    /// Execute a SQL query and return the result.
    ///
    /// # Arguments
    ///
    /// * `query` - SQL query string (SELECT only)
    /// * `params` - Query parameters for binding (currently unused, DataFusion doesn't support
    ///   parameterized queries directly)
    /// * `limit` - Maximum rows to return (bounded by MAX_SQL_RESULT_ROWS)
    /// * `timeout_ms` - Query timeout in milliseconds
    ///
    /// # Returns
    ///
    /// `SqlQueryResult` containing the query results, or an error.
    ///
    /// # Performance
    ///
    /// This method first tries to match the query against fast-path patterns
    /// (point lookups, prefix scans, COUNT(*)) and executes them directly
    /// against Redb. If no fast path matches, it falls back to DataFusion.
    pub async fn execute(
        &self,
        query: &str,
        _params: &[SqlValue],
        limit: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<SqlQueryResult, SqlQueryError> {
        let start = Instant::now();
        let effective_limit = effective_sql_limit(limit) as usize;
        let effective_timeout = effective_sql_timeout_ms(timeout_ms);

        // Try fast paths first (bypassing DataFusion entirely)
        if let Some(result) = self.try_fast_path(query, effective_limit, start)? {
            return Ok(result);
        }

        // Fall back to DataFusion for complex queries
        self.execute_with_datafusion(query, effective_limit, effective_timeout, start).await
    }

    /// Try to execute a query via fast path (direct Redb access).
    ///
    /// Returns `Some(result)` if a fast path was used, `None` if the query
    /// should fall back to DataFusion.
    fn try_fast_path(
        &self,
        query: &str,
        limit: usize,
        start: Instant,
    ) -> Result<Option<SqlQueryResult>, SqlQueryError> {
        // Try point lookup: SELECT * FROM kv WHERE key = 'literal'
        if let Some(captures) = POINT_LOOKUP_REGEX.captures(query) {
            // Group 1 is single quotes, group 2 is double quotes
            let key = captures.get(1).or_else(|| captures.get(2)).map(|m| m.as_str()).unwrap_or("");
            return Ok(Some(self.execute_point_lookup(key, start)?));
        }

        // Try prefix scan: SELECT * FROM kv WHERE key LIKE 'prefix%'
        if let Some(captures) = PREFIX_SCAN_REGEX.captures(query) {
            let prefix = captures.get(1).or_else(|| captures.get(2)).map(|m| m.as_str()).unwrap_or("");
            return Ok(Some(self.execute_prefix_scan(prefix, limit, start)?));
        }

        // Try count all: SELECT COUNT(*) FROM kv
        if COUNT_ALL_REGEX.is_match(query) {
            return Ok(Some(self.execute_count_all(start)?));
        }

        // No fast path matched
        Ok(None)
    }

    /// Execute a point lookup directly against Redb.
    fn execute_point_lookup(&self, key: &str, start: Instant) -> Result<SqlQueryResult, SqlQueryError> {
        let read_txn = self.db.begin_read().map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to begin read transaction: {}", e),
        })?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to open table: {}", e),
        })?;

        let entry = table.get(key.as_bytes()).map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to get key: {}", e),
        })?;

        let rows = match entry {
            Some(value) => {
                let kv: crate::raft::storage_shared::KvEntry =
                    bincode::deserialize(value.value()).map_err(|e| SqlQueryError::ExecutionFailed {
                        reason: format!("failed to deserialize entry: {}", e),
                    })?;

                // Check expiration
                if let Some(expires_at) = kv.expires_at_ms {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    if now > expires_at {
                        vec![]
                    } else {
                        vec![self.kv_entry_to_row(key, &kv)]
                    }
                } else {
                    vec![self.kv_entry_to_row(key, &kv)]
                }
            }
            None => vec![],
        };

        Ok(SqlQueryResult {
            columns: self.kv_columns(),
            row_count: rows.len() as u32,
            rows,
            is_truncated: false,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Execute a prefix scan directly against Redb.
    fn execute_prefix_scan(&self, prefix: &str, limit: usize, start: Instant) -> Result<SqlQueryResult, SqlQueryError> {
        let read_txn = self.db.begin_read().map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to begin read transaction: {}", e),
        })?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to open table: {}", e),
        })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let prefix_bytes = prefix.as_bytes();
        let mut rows = Vec::with_capacity(limit.min(128));
        let mut is_truncated = false;

        for item in table.iter().map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to iterate table: {}", e),
        })? {
            let (key_guard, value_guard) = item.map_err(|e| SqlQueryError::ExecutionFailed {
                reason: format!("failed to read entry: {}", e),
            })?;

            let key_bytes = key_guard.value();

            // Check prefix match
            if !key_bytes.starts_with(prefix_bytes) {
                if key_bytes > prefix_bytes {
                    // Past the prefix range (assuming sorted keys)
                    break;
                }
                continue;
            }

            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let kv: crate::raft::storage_shared::KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = kv.expires_at_ms
                && now > expires_at
            {
                continue;
            }

            if rows.len() >= limit {
                is_truncated = true;
                break;
            }

            rows.push(self.kv_entry_to_row(key_str, &kv));
        }

        Ok(SqlQueryResult {
            columns: self.kv_columns(),
            row_count: rows.len() as u32,
            rows,
            is_truncated,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Execute COUNT(*) directly against Redb.
    fn execute_count_all(&self, start: Instant) -> Result<SqlQueryResult, SqlQueryError> {
        let read_txn = self.db.begin_read().map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to begin read transaction: {}", e),
        })?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to open table: {}", e),
        })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut count: i64 = 0;

        for item in table.iter().map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to iterate table: {}", e),
        })? {
            let (_, value_guard) = item.map_err(|e| SqlQueryError::ExecutionFailed {
                reason: format!("failed to read entry: {}", e),
            })?;

            let kv: crate::raft::storage_shared::KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = kv.expires_at_ms
                && now > expires_at
            {
                continue;
            }

            count += 1;
        }

        Ok(SqlQueryResult {
            columns: vec![SqlColumnInfo {
                name: "count(*)".to_string(),
            }],
            row_count: 1,
            rows: vec![vec![SqlValue::Integer(count)]],
            is_truncated: false,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Convert a KvEntry to a SQL row (all 7 columns).
    fn kv_entry_to_row(&self, key: &str, kv: &crate::raft::storage_shared::KvEntry) -> Vec<SqlValue> {
        vec![
            SqlValue::Text(key.to_string()),
            SqlValue::Text(kv.value.clone()),
            SqlValue::Integer(kv.version),
            SqlValue::Integer(kv.create_revision),
            SqlValue::Integer(kv.mod_revision),
            kv.expires_at_ms.map(|v| SqlValue::Integer(v as i64)).unwrap_or(SqlValue::Null),
            kv.lease_id.map(|v| SqlValue::Integer(v as i64)).unwrap_or(SqlValue::Null),
        ]
    }

    /// Return column info for the KV table.
    fn kv_columns(&self) -> Vec<SqlColumnInfo> {
        vec![
            SqlColumnInfo {
                name: "key".to_string(),
            },
            SqlColumnInfo {
                name: "value".to_string(),
            },
            SqlColumnInfo {
                name: "version".to_string(),
            },
            SqlColumnInfo {
                name: "create_revision".to_string(),
            },
            SqlColumnInfo {
                name: "mod_revision".to_string(),
            },
            SqlColumnInfo {
                name: "expires_at_ms".to_string(),
            },
            SqlColumnInfo {
                name: "lease_id".to_string(),
            },
        ]
    }

    /// Execute a query using DataFusion (fallback for complex queries).
    async fn execute_with_datafusion(
        &self,
        query: &str,
        effective_limit: usize,
        effective_timeout: u32,
        start: Instant,
    ) -> Result<SqlQueryResult, SqlQueryError> {
        // Execute the query with timeout
        let df = tokio::time::timeout(std::time::Duration::from_millis(effective_timeout as u64), self.ctx.sql(query))
            .await
            .map_err(|_| SqlQueryError::Timeout {
                duration_ms: effective_timeout as u64,
            })?
            .map_err(|e| SqlQueryError::SyntaxError { message: e.to_string() })?;

        // Apply limit and collect results
        let df = df.limit(0, Some(effective_limit + 1)).map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("failed to apply limit: {}", e),
        })?;

        // Get schema before collecting
        let schema = df.schema().clone();

        // Collect results with timeout
        let batches = tokio::time::timeout(std::time::Duration::from_millis(effective_timeout as u64), df.collect())
            .await
            .map_err(|_| SqlQueryError::Timeout {
                duration_ms: effective_timeout as u64,
            })?
            .map_err(|e| SqlQueryError::ExecutionFailed {
                reason: format!("query execution failed: {}", e),
            })?;

        // Convert Arrow schema to column info
        let columns: Vec<SqlColumnInfo> =
            schema.fields().iter().map(|f| SqlColumnInfo { name: f.name().clone() }).collect();

        // Convert RecordBatches to rows
        let mut rows: Vec<Vec<SqlValue>> = Vec::new();
        for batch in &batches {
            for row_idx in 0..batch.num_rows() {
                if rows.len() > effective_limit {
                    break;
                }

                let mut row: Vec<SqlValue> = Vec::with_capacity(batch.num_columns());
                for col_idx in 0..batch.num_columns() {
                    let array = batch.column(col_idx);
                    let value = arrow_value_to_sql_value(array, row_idx)?;
                    row.push(value);
                }
                rows.push(row);
            }
        }

        // Determine if truncated
        let is_truncated = rows.len() > effective_limit;
        if is_truncated {
            rows.truncate(effective_limit);
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(SqlQueryResult {
            columns,
            row_count: rows.len() as u32,
            rows,
            is_truncated,
            execution_time_ms,
        })
    }
}

/// Convert an Arrow array value at a given index to SqlValue.
fn arrow_value_to_sql_value(array: &arrow::array::ArrayRef, index: usize) -> Result<SqlValue, SqlQueryError> {
    use arrow::array::*;
    use arrow::datatypes::DataType;

    if array.is_null(index) {
        return Ok(SqlValue::Null);
    }

    let data_type = array.data_type();
    match data_type {
        DataType::Null => Ok(SqlValue::Null),

        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast boolean array".into(),
            })?;
            Ok(SqlValue::Integer(if arr.value(index) { 1 } else { 0 }))
        }

        DataType::Int8 => {
            let arr = array.as_any().downcast_ref::<Int8Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast int8 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast int16 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast int32 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast int64 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index)))
        }

        DataType::UInt8 => {
            let arr = array.as_any().downcast_ref::<UInt8Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast uint8 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::UInt16 => {
            let arr = array.as_any().downcast_ref::<UInt16Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast uint16 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::UInt32 => {
            let arr = array.as_any().downcast_ref::<UInt32Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast uint32 array".into(),
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::UInt64 => {
            let arr = array.as_any().downcast_ref::<UInt64Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast uint64 array".into(),
            })?;
            // Note: This may lose precision for very large values
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast float32 array".into(),
            })?;
            Ok(SqlValue::Real(arr.value(index) as f64))
        }

        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast float64 array".into(),
            })?;
            Ok(SqlValue::Real(arr.value(index)))
        }

        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast string array".into(),
            })?;
            Ok(SqlValue::Text(arr.value(index).to_string()))
        }

        DataType::LargeUtf8 => {
            let arr =
                array.as_any().downcast_ref::<LargeStringArray>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast large string array".into(),
                })?;
            Ok(SqlValue::Text(arr.value(index).to_string()))
        }

        DataType::Binary => {
            let arr = array.as_any().downcast_ref::<BinaryArray>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                reason: "failed to downcast binary array".into(),
            })?;
            Ok(SqlValue::Blob(arr.value(index).to_vec()))
        }

        DataType::LargeBinary => {
            let arr =
                array.as_any().downcast_ref::<LargeBinaryArray>().ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast large binary array".into(),
                })?;
            Ok(SqlValue::Blob(arr.value(index).to_vec()))
        }

        // For other types, convert to string representation
        _ => {
            let formatter = arrow::util::display::ArrayFormatter::try_new(
                array.as_ref(),
                &arrow::util::display::FormatOptions::default(),
            )
            .map_err(|e| SqlQueryError::ExecutionFailed {
                reason: format!("failed to create array formatter: {}", e),
            })?;

            Ok(SqlValue::Text(formatter.value(index).to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use arrow::array::Int64Array;
    use arrow::array::StringArray;

    use super::*;

    #[test]
    fn convert_string_value() {
        let array: arrow::array::ArrayRef = Arc::new(StringArray::from(vec!["hello", "world"]));
        let value = arrow_value_to_sql_value(&array, 0).unwrap();
        assert_eq!(value, SqlValue::Text("hello".to_string()));
    }

    #[test]
    fn convert_int64_value() {
        let array: arrow::array::ArrayRef = Arc::new(Int64Array::from(vec![42, 100]));
        let value = arrow_value_to_sql_value(&array, 0).unwrap();
        assert_eq!(value, SqlValue::Integer(42));
    }

    #[test]
    fn convert_null_value() {
        let array: arrow::array::ArrayRef = Arc::new(StringArray::from(vec![Some("a"), None]));
        let value = arrow_value_to_sql_value(&array, 1).unwrap();
        assert_eq!(value, SqlValue::Null);
    }
}
