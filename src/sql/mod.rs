//! DataFusion SQL integration for Redb storage backend.
//!
//! This module provides SQL query execution on top of the single-fsync Redb
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
//! │   SharedRedbStorage    │  Raw KV access via tuple encoding
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
//! All operations are bounded by constants from `crate::raft::constants`:
//! - Query size: MAX_SQL_QUERY_SIZE (64 KB)
//! - Parameters: MAX_SQL_PARAMS (100)
//! - Results: MAX_SQL_RESULT_ROWS (10,000 rows)
//! - Timeout: MAX_SQL_TIMEOUT_MS (30,000 ms)

mod error;
mod executor;
mod provider;
mod schema;
mod stream;

pub use error::SqlError;
pub use executor::RedbSqlExecutor;
pub use provider::RedbTableProvider;
pub use schema::{KV_SCHEMA, kv_schema};
