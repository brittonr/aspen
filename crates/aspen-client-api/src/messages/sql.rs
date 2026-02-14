//! SQL query response types.
//!
//! Response types for SQL query operations against the state machine.

use serde::Deserialize;
use serde::Serialize;

/// SQL cell value for RPC transport.
///
/// PostCard-compatible representation of SQL values. Unlike `serde_json::Value`,
/// this enum uses explicit variants that PostCard can serialize without
/// self-describing serialization (`serialize_any()`).
///
/// Maps directly to SQLite's type affinity system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlCellValue {
    /// SQL NULL value.
    Null,
    /// 64-bit signed integer (SQLite INTEGER).
    Integer(i64),
    /// 64-bit floating point (SQLite REAL).
    Real(f64),
    /// UTF-8 text string (SQLite TEXT).
    Text(String),
    /// Binary data as base64-encoded string (SQLite BLOB).
    /// Base64 encoding ensures safe text transport.
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
///
/// Contains the result of a read-only SQL query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Column names.
    pub columns: Option<Vec<String>>,
    /// Result rows. Each inner vec contains values for one row in column order.
    /// Uses `SqlCellValue` instead of `serde_json::Value` for PostCard compatibility.
    pub rows: Option<Vec<Vec<SqlCellValue>>>,
    /// Number of rows returned.
    pub row_count: Option<u32>,
    /// True if more rows exist but were not returned due to limit.
    pub is_truncated: Option<bool>,
    /// Query execution time in milliseconds.
    pub execution_time_ms: Option<u64>,
    /// Error message if query failed.
    pub error: Option<String>,
}
