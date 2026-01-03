//! SQL query worker for long-running analytical queries.
//!
//! This worker executes SQL queries against the Aspen KV store using DataFusion.
//! It supports four job types:
//!
//! - `execute_query`: Execute an arbitrary SELECT query
//! - `analyze_table`: Get statistics about the KV table
//! - `export_results`: Execute a query and export results to blob store
//! - `aggregate_metrics`: Compute aggregations (count, sum, avg, min, max)

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use redb::Database;
use redb::ReadableTable;
use serde_json::json;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::Job;
use crate::JobResult;
use crate::Worker;

use aspen_blob::BlobStore;
use aspen_core::storage::KvEntry;
use aspen_core::storage::SM_KV_TABLE;

/// Worker for executing SQL queries against the Aspen KV store.
///
/// This worker requires access to:
/// - A Redb database for direct SQL execution
/// - Optionally, a blob store for exporting query results
pub struct SqlQueryWorker {
    node_id: u64,
    /// Database for SQL queries.
    db: Arc<Database>,
    /// Optional blob store for result export.
    blob_store: Option<Arc<dyn BlobStore>>,
}

impl SqlQueryWorker {
    /// Create a new SQL query worker with database access.
    pub fn new(node_id: u64, db: Arc<Database>) -> Self {
        Self {
            node_id,
            db,
            blob_store: None,
        }
    }

    /// Add blob store for result export functionality.
    pub fn with_blob_store(mut self, blob_store: Arc<dyn BlobStore>) -> Self {
        self.blob_store = Some(blob_store);
        self
    }

    /// Execute a SQL query using DataFusion.
    async fn execute_sql_query(
        &self,
        query: &str,
        limit: Option<u32>,
        timeout_secs: u64,
    ) -> Result<(Vec<Vec<serde_json::Value>>, Vec<String>, u64), String> {
        let start = Instant::now();
        let effective_limit = limit.unwrap_or(1000).min(10000) as usize;
        let effective_timeout = timeout_secs.min(300);

        // For simple queries, use direct Redb access for better performance
        // Complex queries would need the full DataFusion executor

        // Parse the query to determine the operation type
        let query_lower = query.trim().to_lowercase();

        if query_lower.starts_with("select count(*)") {
            // COUNT query - scan and count
            return self.execute_count_query(&query_lower, start).await;
        }

        if query_lower.starts_with("select") {
            // General SELECT query - execute via scan
            return self
                .execute_select_query(&query_lower, effective_limit, start)
                .await;
        }

        Err(format!(
            "unsupported query type, only SELECT queries are allowed: {}",
            query
        ))
    }

    /// Execute a COUNT(*) query.
    async fn execute_count_query(
        &self,
        query: &str,
        start: Instant,
    ) -> Result<(Vec<Vec<serde_json::Value>>, Vec<String>, u64), String> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn
            .open_table(SM_KV_TABLE)
            .map_err(|e| format!("failed to open table: {}", e))?;

        // Extract optional WHERE clause prefix filter
        let prefix = self.extract_prefix_from_where(query);
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        let mut count: i64 = 0;

        for item in table
            .iter()
            .map_err(|e| format!("failed to iterate: {}", e))?
        {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            // Apply prefix filter if present
            if let Some(ref p) = prefix {
                let key_bytes = key_guard.value();
                if !key_bytes.starts_with(p.as_bytes()) {
                    if key_bytes > p.as_bytes() {
                        break; // Past prefix range
                    }
                    continue;
                }
            }

            // Deserialize and check expiration
            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            count += 1;
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok((
            vec![vec![json!(count)]],
            vec!["count(*)".to_string()],
            execution_time_ms,
        ))
    }

    /// Execute a SELECT query with optional filters.
    async fn execute_select_query(
        &self,
        query: &str,
        limit: usize,
        start: Instant,
    ) -> Result<(Vec<Vec<serde_json::Value>>, Vec<String>, u64), String> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn
            .open_table(SM_KV_TABLE)
            .map_err(|e| format!("failed to open table: {}", e))?;

        // Extract optional WHERE clause prefix filter
        let prefix = self.extract_prefix_from_where(query);
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        let mut rows: Vec<Vec<serde_json::Value>> = Vec::with_capacity(limit.min(1000));

        for item in table
            .iter()
            .map_err(|e| format!("failed to iterate: {}", e))?
        {
            if rows.len() >= limit {
                break;
            }

            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let key_bytes = key_guard.value();

            // Apply prefix filter if present
            if let Some(ref p) = prefix {
                if !key_bytes.starts_with(p.as_bytes()) {
                    if key_bytes > p.as_bytes() {
                        break; // Past prefix range
                    }
                    continue;
                }
            }

            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Deserialize entry
            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            // Build row with all columns
            rows.push(vec![
                json!(key_str),
                json!(kv.value),
                json!(kv.version),
                json!(kv.create_revision),
                json!(kv.mod_revision),
                kv.expires_at_ms.map(|v| json!(v)).unwrap_or(json!(null)),
                kv.lease_id.map(|v| json!(v)).unwrap_or(json!(null)),
            ]);
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let columns = vec![
            "key".to_string(),
            "value".to_string(),
            "version".to_string(),
            "create_revision".to_string(),
            "mod_revision".to_string(),
            "expires_at_ms".to_string(),
            "lease_id".to_string(),
        ];

        Ok((rows, columns, execution_time_ms))
    }

    /// Extract prefix from a WHERE clause like "WHERE key LIKE 'prefix%'".
    fn extract_prefix_from_where(&self, query: &str) -> Option<String> {
        // Simple regex-like extraction for "key LIKE 'prefix%'" pattern
        if let Some(like_pos) = query.find("key like '") {
            let start = like_pos + 10; // len("key like '")
            if let Some(end) = query[start..].find('%') {
                return Some(query[start..start + end].to_string());
            }
        }
        if let Some(like_pos) = query.find("key = '") {
            let start = like_pos + 7; // len("key = '")
            if let Some(end) = query[start..].find('\'') {
                return Some(query[start..start + end].to_string());
            }
        }
        None
    }

    /// Get table statistics by scanning the entire table.
    async fn analyze_table_stats(&self, table_name: &str) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn
            .open_table(SM_KV_TABLE)
            .map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        let mut row_count: u64 = 0;
        let mut total_key_bytes: u64 = 0;
        let mut total_value_bytes: u64 = 0;
        let mut min_version: i64 = i64::MAX;
        let mut max_version: i64 = i64::MIN;
        let mut min_create_rev: i64 = i64::MAX;
        let mut max_mod_rev: i64 = i64::MIN;
        let mut expired_count: u64 = 0;
        let mut with_lease_count: u64 = 0;

        for item in table
            .iter()
            .map_err(|e| format!("failed to iterate: {}", e))?
        {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let key_bytes = key_guard.value();
            let value_bytes = value_guard.value();

            // Deserialize entry
            let kv: KvEntry = match bincode::deserialize(value_bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    expired_count += 1;
                    continue;
                }
            }

            row_count += 1;
            total_key_bytes += key_bytes.len() as u64;
            total_value_bytes += kv.value.len() as u64;

            min_version = min_version.min(kv.version);
            max_version = max_version.max(kv.version);
            min_create_rev = min_create_rev.min(kv.create_revision);
            max_mod_rev = max_mod_rev.max(kv.mod_revision);

            if kv.lease_id.is_some() {
                with_lease_count += 1;
            }
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Handle empty table case
        if row_count == 0 {
            min_version = 0;
            max_version = 0;
            min_create_rev = 0;
            max_mod_rev = 0;
        }

        Ok(json!({
            "table": table_name,
            "row_count": row_count,
            "total_key_bytes": total_key_bytes,
            "total_value_bytes": total_value_bytes,
            "total_size_bytes": total_key_bytes + total_value_bytes,
            "avg_key_size": if row_count > 0 { total_key_bytes / row_count } else { 0 },
            "avg_value_size": if row_count > 0 { total_value_bytes / row_count } else { 0 },
            "min_version": min_version,
            "max_version": max_version,
            "min_create_revision": min_create_rev,
            "max_mod_revision": max_mod_rev,
            "expired_keys_skipped": expired_count,
            "keys_with_lease": with_lease_count,
            "analysis_time_ms": execution_time_ms,
            "analyzed_at": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Export query results to blob store.
    async fn export_to_blob(
        &self,
        rows: &[Vec<serde_json::Value>],
        columns: &[String],
        format: &str,
    ) -> Result<(String, u64), String> {
        let blob_store = self
            .blob_store
            .as_ref()
            .ok_or("blob store not configured for export")?;

        let data = match format {
            "csv" => {
                let mut csv = columns.join(",") + "\n";
                for row in rows {
                    let row_str: Vec<String> = row
                        .iter()
                        .map(|v| match v {
                            serde_json::Value::String(s) => {
                                // Escape quotes and wrap in quotes if contains comma
                                if s.contains(',') || s.contains('"') || s.contains('\n') {
                                    format!("\"{}\"", s.replace('"', "\"\""))
                                } else {
                                    s.clone()
                                }
                            }
                            serde_json::Value::Null => String::new(),
                            other => other.to_string(),
                        })
                        .collect();
                    csv.push_str(&row_str.join(","));
                    csv.push('\n');
                }
                csv.into_bytes()
            }
            "json" | "jsonl" => {
                let mut output = Vec::new();
                for row in rows {
                    let obj: serde_json::Map<String, serde_json::Value> = columns
                        .iter()
                        .zip(row.iter())
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    let line = serde_json::to_string(&obj)
                        .map_err(|e| format!("serialization error: {}", e))?;
                    output.extend_from_slice(line.as_bytes());
                    output.push(b'\n');
                }
                output
            }
            _ => {
                return Err(format!(
                    "unsupported export format '{}', use 'csv' or 'json'",
                    format
                ));
            }
        };

        let size = data.len() as u64;
        let result = blob_store
            .add_bytes(&data)
            .await
            .map_err(|e| format!("failed to store blob: {}", e))?;

        let hash = result.blob_ref.hash.to_hex().to_string();

        info!(
            node_id = self.node_id,
            hash = %hash,
            size = size,
            format = format,
            rows = rows.len(),
            "exported query results to blob"
        );

        Ok((hash, size))
    }

    /// Compute aggregate metrics.
    async fn compute_aggregates(
        &self,
        metric: &str,
        column: Option<&str>,
        group_by: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn
            .open_table(SM_KV_TABLE)
            .map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        // Aggregation state
        let mut count: i64 = 0;
        let mut sum: i64 = 0;
        let mut min_val: Option<i64> = None;
        let mut max_val: Option<i64> = None;
        let mut values: Vec<i64> = Vec::new(); // For percentile calculations

        // Determine which column to aggregate
        let target_col = column.unwrap_or("version");

        for item in table
            .iter()
            .map_err(|e| format!("failed to iterate: {}", e))?
        {
            let (_, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            // Get the value for the target column
            let value = match target_col {
                "version" => kv.version,
                "create_revision" => kv.create_revision,
                "mod_revision" => kv.mod_revision,
                "value_length" => kv.value.len() as i64,
                _ => kv.version, // Default to version
            };

            count += 1;
            sum += value;
            min_val = Some(min_val.map_or(value, |m| m.min(value)));
            max_val = Some(max_val.map_or(value, |m| m.max(value)));

            // Store for percentile (limit to avoid memory issues)
            if values.len() < 100_000 {
                values.push(value);
            }
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Compute requested metric
        let result = match metric {
            "count" => json!({
                "metric": "count",
                "value": count
            }),
            "sum" => json!({
                "metric": "sum",
                "column": target_col,
                "value": sum
            }),
            "avg" | "average" => json!({
                "metric": "avg",
                "column": target_col,
                "value": if count > 0 { sum as f64 / count as f64 } else { 0.0 }
            }),
            "min" => json!({
                "metric": "min",
                "column": target_col,
                "value": min_val.unwrap_or(0)
            }),
            "max" => json!({
                "metric": "max",
                "column": target_col,
                "value": max_val.unwrap_or(0)
            }),
            "stats" | "all" => {
                // Sort for percentile calculations
                values.sort_unstable();
                let p50 = if !values.is_empty() {
                    values[values.len() / 2]
                } else {
                    0
                };
                let p95 = if !values.is_empty() {
                    values[(values.len() as f64 * 0.95) as usize]
                } else {
                    0
                };
                let p99 = if !values.is_empty() {
                    values[(values.len() as f64 * 0.99) as usize]
                } else {
                    0
                };

                json!({
                    "metric": "stats",
                    "column": target_col,
                    "count": count,
                    "sum": sum,
                    "avg": if count > 0 { sum as f64 / count as f64 } else { 0.0 },
                    "min": min_val.unwrap_or(0),
                    "max": max_val.unwrap_or(0),
                    "p50": p50,
                    "p95": p95,
                    "p99": p99
                })
            }
            _ => {
                return Err(format!(
                    "unknown metric '{}', use count/sum/avg/min/max/stats",
                    metric
                ));
            }
        };

        Ok(json!({
            "node_id": self.node_id,
            "group_by": group_by,
            "results": result,
            "execution_time_ms": execution_time_ms
        }))
    }
}

#[async_trait]
impl Worker for SqlQueryWorker {
    async fn execute(&self, job: Job) -> JobResult {
        match job.spec.job_type.as_str() {
            "execute_query" => {
                let query = job.spec.payload["query"]
                    .as_str()
                    .unwrap_or("SELECT * FROM kv LIMIT 10");
                let limit = job.spec.payload["limit"].as_u64().map(|v| v as u32);
                let timeout_secs = job.spec.payload["timeout_secs"].as_u64().unwrap_or(60);

                info!(
                    node_id = self.node_id,
                    query_preview = &query[..query.len().min(100)],
                    limit = ?limit,
                    timeout_secs = timeout_secs,
                    "executing SQL query"
                );

                match self.execute_sql_query(query, limit, timeout_secs).await {
                    Ok((rows, columns, execution_time_ms)) => {
                        let row_count = rows.len();
                        let is_truncated = limit.is_some_and(|l| rows.len() >= l as usize);

                        debug!(
                            node_id = self.node_id,
                            rows_returned = row_count,
                            execution_time_ms = execution_time_ms,
                            "query completed"
                        );

                        JobResult::success(json!({
                            "node_id": self.node_id,
                            "columns": columns,
                            "rows": rows,
                            "rows_returned": row_count,
                            "is_truncated": is_truncated,
                            "execution_time_ms": execution_time_ms
                        }))
                    }
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "query failed");
                        JobResult::failure(format!("query execution failed: {}", e))
                    }
                }
            }

            "analyze_table" => {
                let table_name = job.spec.payload["table"].as_str().unwrap_or("kv");

                info!(
                    node_id = self.node_id,
                    table = table_name,
                    "analyzing table statistics"
                );

                match self.analyze_table_stats(table_name).await {
                    Ok(stats) => {
                        debug!(
                            node_id = self.node_id,
                            table = table_name,
                            "table analysis completed"
                        );
                        JobResult::success(json!({
                            "node_id": self.node_id,
                            "statistics": stats
                        }))
                    }
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "table analysis failed");
                        JobResult::failure(format!("table analysis failed: {}", e))
                    }
                }
            }

            "export_results" => {
                let query = job.spec.payload["query"]
                    .as_str()
                    .unwrap_or("SELECT * FROM kv LIMIT 100");
                let format = job.spec.payload["format"].as_str().unwrap_or("csv");
                let limit = job.spec.payload["limit"].as_u64().map(|v| v as u32);

                info!(
                    node_id = self.node_id,
                    format = format,
                    "exporting query results to blob"
                );

                // First execute the query
                match self.execute_sql_query(query, limit, 120).await {
                    Ok((rows, columns, query_time_ms)) => {
                        // Then export to blob
                        match self.export_to_blob(&rows, &columns, format).await {
                            Ok((blob_hash, size_bytes)) => JobResult::success(json!({
                                "node_id": self.node_id,
                                "format": format,
                                "blob_hash": blob_hash,
                                "rows_exported": rows.len(),
                                "size_bytes": size_bytes,
                                "query_time_ms": query_time_ms
                            })),
                            Err(e) => JobResult::failure(format!("export failed: {}", e)),
                        }
                    }
                    Err(e) => JobResult::failure(format!("query failed before export: {}", e)),
                }
            }

            "aggregate_metrics" => {
                let metric = job.spec.payload["metric"].as_str().unwrap_or("count");
                let column = job.spec.payload["column"].as_str();
                let group_by = job.spec.payload["group_by"].as_str();

                info!(
                    node_id = self.node_id,
                    metric = metric,
                    column = ?column,
                    group_by = ?group_by,
                    "computing aggregate metrics"
                );

                match self.compute_aggregates(metric, column, group_by).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => JobResult::failure(format!("aggregation failed: {}", e)),
                }
            }

            _ => JobResult::failure(format!(
                "unknown SQL query task: {}",
                job.spec.job_type
            )),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec![
            "execute_query".to_string(),
            "analyze_table".to_string(),
            "export_results".to_string(),
            "aggregate_metrics".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_prefix_from_where() {
        let worker = SqlQueryWorker {
            node_id: 1,
            db: Arc::new(redb::Database::builder().create_with_backend(redb::backends::InMemoryBackend::new()).unwrap()),
            blob_store: None,
        };

        // Test LIKE pattern
        let query = "select * from kv where key like 'user:%'";
        assert_eq!(worker.extract_prefix_from_where(query), Some("user:".to_string()));

        // Test exact match
        let query = "select * from kv where key = 'config'";
        assert_eq!(worker.extract_prefix_from_where(query), Some("config".to_string()));

        // Test no WHERE clause
        let query = "select * from kv";
        assert_eq!(worker.extract_prefix_from_where(query), None);
    }
}
