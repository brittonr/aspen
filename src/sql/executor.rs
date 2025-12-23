//! SQL query executor for Redb storage.
//!
//! This module provides the `RedbSqlExecutor` which executes SQL queries
//! against the Redb KV storage using DataFusion.

use std::sync::Arc;
use std::time::Instant;

use datafusion::prelude::*;
use redb::Database;

use super::provider::RedbTableProvider;
use crate::api::{
    SqlColumnInfo, SqlQueryError, SqlQueryResult, SqlValue, effective_sql_limit,
    effective_sql_timeout_ms,
};

/// SQL query executor for Redb storage.
///
/// Wraps a Redb database and provides SQL query execution via DataFusion.
/// Queries are executed against a virtual `kv` table that exposes the
/// key-value data.
///
/// # Example
///
/// ```ignore
/// let executor = RedbSqlExecutor::new(db);
/// let result = executor.execute(
///     "SELECT key, value FROM kv WHERE key LIKE 'user:%'",
///     &[],
///     Some(100),
///     Some(5000),
/// ).await?;
/// ```
pub struct RedbSqlExecutor {
    /// Reference to the Redb database.
    db: Arc<Database>,
}

impl RedbSqlExecutor {
    /// Create a new SQL executor for the given database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Execute a SQL query and return the result.
    ///
    /// # Arguments
    ///
    /// * `query` - SQL query string (SELECT only)
    /// * `params` - Query parameters for binding (currently unused, DataFusion
    ///   doesn't support parameterized queries directly)
    /// * `limit` - Maximum rows to return (bounded by MAX_SQL_RESULT_ROWS)
    /// * `timeout_ms` - Query timeout in milliseconds
    ///
    /// # Returns
    ///
    /// `SqlQueryResult` containing the query results, or an error.
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

        // Create a DataFusion session
        let config = SessionConfig::new()
            .with_target_partitions(1) // Single partition for Redb
            .with_batch_size(8192);

        let ctx = SessionContext::new_with_config(config);

        // Register the KV table
        let provider = RedbTableProvider::new(self.db.clone());
        ctx.register_table("kv", Arc::new(provider)).map_err(|e| {
            SqlQueryError::ExecutionFailed {
                reason: format!("failed to register table: {}", e),
            }
        })?;

        // Execute the query with timeout
        let df = tokio::time::timeout(
            std::time::Duration::from_millis(effective_timeout as u64),
            ctx.sql(query),
        )
        .await
        .map_err(|_| SqlQueryError::Timeout {
            duration_ms: effective_timeout as u64,
        })?
        .map_err(|e| SqlQueryError::SyntaxError {
            message: e.to_string(),
        })?;

        // Apply limit and collect results
        let df =
            df.limit(0, Some(effective_limit + 1))
                .map_err(|e| SqlQueryError::ExecutionFailed {
                    reason: format!("failed to apply limit: {}", e),
                })?;

        // Get schema before collecting
        let schema = df.schema().clone();

        // Collect results with timeout
        let batches = tokio::time::timeout(
            std::time::Duration::from_millis(effective_timeout as u64),
            df.collect(),
        )
        .await
        .map_err(|_| SqlQueryError::Timeout {
            duration_ms: effective_timeout as u64,
        })?
        .map_err(|e| SqlQueryError::ExecutionFailed {
            reason: format!("query execution failed: {}", e),
        })?;

        // Convert Arrow schema to column info
        let columns: Vec<SqlColumnInfo> = schema
            .fields()
            .iter()
            .map(|f| SqlColumnInfo {
                name: f.name().clone(),
            })
            .collect();

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
fn arrow_value_to_sql_value(
    array: &arrow::array::ArrayRef,
    index: usize,
) -> Result<SqlValue, SqlQueryError> {
    use arrow::array::*;
    use arrow::datatypes::DataType;

    if array.is_null(index) {
        return Ok(SqlValue::Null);
    }

    let data_type = array.data_type();
    match data_type {
        DataType::Null => Ok(SqlValue::Null),

        DataType::Boolean => {
            let arr = array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast boolean array".into(),
                })?;
            Ok(SqlValue::Integer(if arr.value(index) { 1 } else { 0 }))
        }

        DataType::Int8 => {
            let arr = array.as_any().downcast_ref::<Int8Array>().ok_or_else(|| {
                SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast int8 array".into(),
                }
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().ok_or_else(|| {
                SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast int16 array".into(),
                }
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().ok_or_else(|| {
                SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast int32 array".into(),
                }
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().ok_or_else(|| {
                SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast int64 array".into(),
                }
            })?;
            Ok(SqlValue::Integer(arr.value(index)))
        }

        DataType::UInt8 => {
            let arr = array.as_any().downcast_ref::<UInt8Array>().ok_or_else(|| {
                SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast uint8 array".into(),
                }
            })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::UInt16 => {
            let arr = array
                .as_any()
                .downcast_ref::<UInt16Array>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast uint16 array".into(),
                })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::UInt32 => {
            let arr = array
                .as_any()
                .downcast_ref::<UInt32Array>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast uint32 array".into(),
                })?;
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::UInt64 => {
            let arr = array
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast uint64 array".into(),
                })?;
            // Note: This may lose precision for very large values
            Ok(SqlValue::Integer(arr.value(index) as i64))
        }

        DataType::Float32 => {
            let arr = array
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast float32 array".into(),
                })?;
            Ok(SqlValue::Real(arr.value(index) as f64))
        }

        DataType::Float64 => {
            let arr = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast float64 array".into(),
                })?;
            Ok(SqlValue::Real(arr.value(index)))
        }

        DataType::Utf8 => {
            let arr = array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast string array".into(),
                })?;
            Ok(SqlValue::Text(arr.value(index).to_string()))
        }

        DataType::LargeUtf8 => {
            let arr = array
                .as_any()
                .downcast_ref::<LargeStringArray>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast large string array".into(),
                })?;
            Ok(SqlValue::Text(arr.value(index).to_string()))
        }

        DataType::Binary => {
            let arr = array
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
                    reason: "failed to downcast binary array".into(),
                })?;
            Ok(SqlValue::Blob(arr.value(index).to_vec()))
        }

        DataType::LargeBinary => {
            let arr = array
                .as_any()
                .downcast_ref::<LargeBinaryArray>()
                .ok_or_else(|| SqlQueryError::ExecutionFailed {
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
    use super::*;
    use arrow::array::{Int64Array, StringArray};

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
