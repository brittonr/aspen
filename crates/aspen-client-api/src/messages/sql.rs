//! SQL query types.
//!
//! Request/response types for SQL query operations against the state machine.

use serde::Deserialize;
use serde::Serialize;

/// SQL domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlRequest {
    /// Execute a read-only SQL query against the state machine.
    ExecuteSql {
        /// SQL query string (must be SELECT or WITH...SELECT).
        query: String,
        /// Query parameters (JSON-serialized SqlValue array).
        params: String,
        /// Consistency level: "linearizable" (default) or "stale".
        consistency: String,
        /// Maximum rows to return.
        limit: Option<u32>,
        /// Query timeout in milliseconds.
        timeout_ms: Option<u32>,
    },
}

#[cfg(feature = "auth")]
impl SqlRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        Some(aspen_auth::Operation::Read {
            key: "_sql:".to_string(),
        })
    }
}

/// SQL cell value for RPC transport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlCellValue {
    /// SQL NULL value.
    Null,
    /// 64-bit signed integer.
    Integer(i64),
    /// 64-bit floating point.
    Real(f64),
    /// UTF-8 text string.
    Text(String),
    /// Binary data as base64-encoded string.
    Blob(String),
}

impl SqlCellValue {
    /// Convert to display string for TUI rendering.
    pub fn to_display_string(&self) -> String {
        match self {
            SqlCellValue::Null => "(null)".to_string(),
            SqlCellValue::Integer(i) => i.to_string(),
            SqlCellValue::Real(f) => f.to_string(),
            SqlCellValue::Text(s) => s.clone(),
            SqlCellValue::Blob(b64) => format!("[blob: {}]", b64),
        }
    }
}

/// SQL query result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    /// Whether the query succeeded.
    pub is_success: bool,
    /// Column names.
    pub columns: Option<Vec<String>>,
    /// Result rows.
    pub rows: Option<Vec<Vec<SqlCellValue>>>,
    /// Number of rows returned.
    pub row_count: Option<u32>,
    /// True if more rows exist but were not returned.
    pub is_truncated: Option<bool>,
    /// Query execution time in milliseconds.
    pub execution_time_ms: Option<u64>,
    /// Error message if query failed.
    pub error: Option<String>,
}
