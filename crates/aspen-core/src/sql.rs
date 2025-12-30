//! SQL query types and traits (requires 'sql' feature).
//!
//! Provides types for SQL query execution against the state machine.

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::constants::DEFAULT_SQL_RESULT_ROWS;
use crate::constants::DEFAULT_SQL_TIMEOUT_MS;
use crate::constants::MAX_SQL_PARAMS;
use crate::constants::MAX_SQL_QUERY_SIZE;
use crate::constants::MAX_SQL_RESULT_ROWS;
use crate::constants::MAX_SQL_TIMEOUT_MS;

/// Consistency level for SQL read queries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SqlConsistency {
    /// Linearizable consistency via Raft ReadIndex protocol.
    #[default]
    Linearizable,
    /// Stale read from local state machine.
    Stale,
}

/// A typed SQL value preserving SQLite's type system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// Information about a result column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SqlColumnInfo {
    pub name: String,
}

/// Request to execute a read-only SQL query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SqlQueryRequest {
    pub query: String,
    pub params: Vec<SqlValue>,
    pub consistency: SqlConsistency,
    pub limit: Option<u32>,
    pub timeout_ms: Option<u32>,
}

/// Result of a SQL query execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SqlQueryResult {
    pub columns: Vec<SqlColumnInfo>,
    pub rows: Vec<Vec<SqlValue>>,
    pub row_count: u32,
    pub is_truncated: bool,
    pub execution_time_ms: u64,
}

/// Errors that can occur during SQL query execution.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SqlQueryError {
    #[error("query not allowed: {reason}")]
    QueryNotAllowed { reason: String },

    #[error("SQL syntax error: {message}")]
    SyntaxError { message: String },

    #[error("execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("query timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("not leader; current leader: {leader:?}")]
    NotLeader { leader: Option<u64> },

    #[error("query size {size} exceeds maximum of {max} bytes")]
    QueryTooLarge { size: usize, max: u32 },

    #[error("parameter count {count} exceeds maximum of {max}")]
    TooManyParams { count: usize, max: u32 },

    #[error("SQL queries not supported by {backend} backend")]
    NotSupported { backend: String },
}

/// Validate a SQL query request against Tiger Style bounds.
pub fn validate_sql_request(request: &SqlQueryRequest) -> Result<(), SqlQueryError> {
    if request.query.len() > MAX_SQL_QUERY_SIZE as usize {
        return Err(SqlQueryError::QueryTooLarge {
            size: request.query.len(),
            max: MAX_SQL_QUERY_SIZE,
        });
    }

    if request.params.len() > MAX_SQL_PARAMS as usize {
        return Err(SqlQueryError::TooManyParams {
            count: request.params.len(),
            max: MAX_SQL_PARAMS,
        });
    }

    if let Some(limit) = request.limit {
        if limit > MAX_SQL_RESULT_ROWS {
            return Err(SqlQueryError::QueryNotAllowed {
                reason: format!(
                    "limit {} exceeds maximum of {} rows",
                    limit, MAX_SQL_RESULT_ROWS
                ),
            });
        }
    }

    if let Some(timeout) = request.timeout_ms {
        if timeout > MAX_SQL_TIMEOUT_MS {
            return Err(SqlQueryError::QueryNotAllowed {
                reason: format!(
                    "timeout {}ms exceeds maximum of {}ms",
                    timeout, MAX_SQL_TIMEOUT_MS
                ),
            });
        }
    }

    Ok(())
}

/// Get effective limit, applying defaults and bounds.
pub fn effective_sql_limit(request_limit: Option<u32>) -> u32 {
    request_limit
        .unwrap_or(DEFAULT_SQL_RESULT_ROWS)
        .min(MAX_SQL_RESULT_ROWS)
}

/// Get effective timeout in milliseconds, applying defaults and bounds.
pub fn effective_sql_timeout_ms(request_timeout: Option<u32>) -> u32 {
    request_timeout
        .unwrap_or(DEFAULT_SQL_TIMEOUT_MS)
        .min(MAX_SQL_TIMEOUT_MS)
}

// ============================================================================
// SQL Query Validation
// ============================================================================

/// Keywords that indicate write operations.
const FORBIDDEN_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "REPLACE", "UPSERT", "CREATE", "DROP", "ALTER", "TRUNCATE",
    "BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE", "ATTACH", "DETACH", "VACUUM",
    "REINDEX", "ANALYZE", "PRAGMA",
];

/// Validate that a SQL query is a read-only SELECT statement.
pub fn validate_sql_query(query: &str) -> Result<(), SqlQueryError> {
    let trimmed = query.trim();

    if trimmed.is_empty() {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: "empty query".into(),
        });
    }

    let upper = trimmed.to_uppercase();
    if !upper.starts_with("SELECT") && !upper.starts_with("WITH") {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: format!(
                "query must start with SELECT or WITH, got: {}",
                &trimmed[..trimmed.len().min(20)]
            ),
        });
    }

    for keyword in FORBIDDEN_KEYWORDS {
        if contains_keyword(&upper, keyword) {
            return Err(SqlQueryError::QueryNotAllowed {
                reason: format!("forbidden keyword: {}", keyword),
            });
        }
    }

    Ok(())
}

/// Check if the query contains a keyword as a standalone word.
fn contains_keyword(upper_query: &str, keyword: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = upper_query[start..].find(keyword) {
        let abs_pos = start + pos;
        let before_ok =
            abs_pos == 0 || !is_identifier_char(upper_query.as_bytes()[abs_pos - 1]);
        let after_pos = abs_pos + keyword.len();
        let after_ok =
            after_pos >= upper_query.len() || !is_identifier_char(upper_query.as_bytes()[after_pos]);

        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= upper_query.len() {
            break;
        }
    }
    false
}

fn is_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// SQL query executor interface.
#[async_trait]
pub trait SqlQueryExecutor: Send + Sync {
    /// Execute a read-only SQL query against the state machine.
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError>;
}

// Blanket implementation for Arc<T>
#[async_trait]
impl<T: SqlQueryExecutor + ?Sized> SqlQueryExecutor for std::sync::Arc<T> {
    async fn execute_sql(&self, request: SqlQueryRequest) -> Result<SqlQueryResult, SqlQueryError> {
        (**self).execute_sql(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_select_queries() {
        assert!(validate_sql_query("SELECT * FROM users").is_ok());
        assert!(validate_sql_query("select * from users").is_ok());
        assert!(validate_sql_query("  SELECT * FROM users  ").is_ok());
    }

    #[test]
    fn test_forbidden_write_operations() {
        assert!(validate_sql_query("INSERT INTO users VALUES (1, 'test')").is_err());
        assert!(validate_sql_query("UPDATE users SET name = 'test'").is_err());
        assert!(validate_sql_query("DELETE FROM users WHERE id = 1").is_err());
    }

    #[test]
    fn test_column_names_not_blocked() {
        assert!(validate_sql_query("SELECT updated_at FROM users").is_ok());
        assert!(validate_sql_query("SELECT deleted FROM items").is_ok());
    }

    #[test]
    fn test_empty_query() {
        assert!(validate_sql_query("").is_err());
        assert!(validate_sql_query("   ").is_err());
    }
}
