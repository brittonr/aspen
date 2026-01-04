//! DataFusion SQL integration for Aspen's Redb storage backend.
//!
//! This crate provides SQL query execution on top of the single-fsync Redb
//! storage engine, using Apache DataFusion as the query engine.
//!
//! # Architecture
//!
//! ```text
//! SQL Query (SELECT * FROM kv WHERE key LIKE 'prefix%')
//!          ↓
//! ┌────────────────────────┐
//! │    RedbSqlExecutor     │  Parse & plan SQL queries
//! └────────────────────────┘
//!          ↓
//! ┌────────────────────────┐
//! │   RedbTableProvider    │  DataFusion TableProvider trait
//! └────────────────────────┘
//!          ↓
//! ┌────────────────────────┐
//! │    RedbExecutionPlan   │  Stream RecordBatches from Redb
//! └────────────────────────┘
//!          ↓
//! ┌────────────────────────┐
//! │   Redb Database        │  Raw KV access via tuple encoding
//! └────────────────────────┘
//! ```
//!
//! # Filter Pushdown
//!
//! The `RedbTableProvider` implements filter pushdown for range predicates:
//! - `key = 'value'` → exact key lookup
//! - `key LIKE 'prefix%'` → prefix scan
//! - `key >= 'start' AND key < 'end'` → range scan
//!
//! These predicates are extracted from WHERE clauses and converted to
//! Redb range queries to minimize data read from storage.
//!
//! # Streaming Results
//!
//! Results are streamed as Arrow `RecordBatch`es with configurable batch size
//! (default 8192 rows) to prevent memory exhaustion on large result sets.
//!
//! # Tiger Style
//!
//! All operations are bounded by constants from `aspen_core::constants`:
//! - Query size: MAX_SQL_QUERY_SIZE (64 KB)
//! - Parameters: MAX_SQL_PARAMS (100)
//! - Results: MAX_SQL_RESULT_ROWS (10,000 rows)
//! - Timeout: MAX_SQL_TIMEOUT_MS (30,000 ms)
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use redb::Database;
//! use aspen_sql::RedbSqlExecutor;
//!
//! // Create executor with database reference
//! let db = Arc::new(Database::create("data.redb")?);
//! let executor = RedbSqlExecutor::new(db);
//!
//! // Execute SQL query
//! let result = executor.execute(
//!     "SELECT * FROM kv WHERE key LIKE 'user:%'",
//!     &[],
//!     Some(100),  // limit
//!     Some(5000), // timeout_ms
//! ).await?;
//! ```

mod error;
mod executor;
mod provider;
mod schema;
mod stream;

// Re-export core SQL types for convenience
pub use aspen_core::sql::{
    SqlColumnInfo, SqlConsistency, SqlQueryError, SqlQueryExecutor, SqlQueryRequest, SqlQueryResult, SqlValue,
    effective_sql_limit, effective_sql_timeout_ms, validate_sql_query, validate_sql_request,
};
pub use error::SqlError;
pub use executor::RedbSqlExecutor;
pub use provider::RedbTableProvider;
pub use schema::KV_SCHEMA;
pub use schema::kv_schema;

/// Get current Unix timestamp in milliseconds.
///
/// Simple utility for TTL expiration checks.
#[inline]
pub fn now_unix_ms() -> u64 {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
