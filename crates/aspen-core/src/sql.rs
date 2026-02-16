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
    QueryTooLarge { size: u32, max: u32 },

    #[error("parameter count {count} exceeds maximum of {max}")]
    TooManyParams { count: u32, max: u32 },

    #[error("SQL queries not supported by {backend} backend")]
    NotSupported { backend: String },
}

/// Validate a SQL query request against Tiger Style bounds.
pub fn validate_sql_request(request: &SqlQueryRequest) -> Result<(), SqlQueryError> {
    if request.query.len() > MAX_SQL_QUERY_SIZE as usize {
        return Err(SqlQueryError::QueryTooLarge {
            size: request.query.len() as u32,
            max: MAX_SQL_QUERY_SIZE,
        });
    }

    if request.params.len() > MAX_SQL_PARAMS as usize {
        return Err(SqlQueryError::TooManyParams {
            count: request.params.len() as u32,
            max: MAX_SQL_PARAMS,
        });
    }

    if let Some(limit) = request.limit
        && limit > MAX_SQL_RESULT_ROWS
    {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: format!("limit {} exceeds maximum of {} rows", limit, MAX_SQL_RESULT_ROWS),
        });
    }

    if let Some(timeout) = request.timeout_ms
        && timeout > MAX_SQL_TIMEOUT_MS
    {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: format!("timeout {}ms exceeds maximum of {}ms", timeout, MAX_SQL_TIMEOUT_MS),
        });
    }

    Ok(())
}

/// Get effective limit, applying defaults and bounds.
pub fn effective_sql_limit(request_limit: Option<u32>) -> u32 {
    request_limit.unwrap_or(DEFAULT_SQL_RESULT_ROWS).min(MAX_SQL_RESULT_ROWS)
}

/// Get effective timeout in milliseconds, applying defaults and bounds.
pub fn effective_sql_timeout_ms(request_timeout: Option<u32>) -> u32 {
    request_timeout.unwrap_or(DEFAULT_SQL_TIMEOUT_MS).min(MAX_SQL_TIMEOUT_MS)
}

// ============================================================================
// SQL Query Validation
// ============================================================================

/// Keywords that indicate write operations.
const FORBIDDEN_KEYWORDS: &[&str] = &[
    "INSERT",
    "UPDATE",
    "DELETE",
    "REPLACE",
    "UPSERT",
    "CREATE",
    "DROP",
    "ALTER",
    "TRUNCATE",
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
    "SAVEPOINT",
    "RELEASE",
    "ATTACH",
    "DETACH",
    "VACUUM",
    "REINDEX",
    "ANALYZE",
    "PRAGMA",
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
            reason: format!("query must start with SELECT or WITH, got: {}", &trimmed[..trimmed.len().min(20)]),
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
        let before_ok = abs_pos == 0 || !is_identifier_char(upper_query.as_bytes()[abs_pos - 1]);
        let after_pos = abs_pos + keyword.len();
        let after_ok = after_pos >= upper_query.len() || !is_identifier_char(upper_query.as_bytes()[after_pos]);

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

    // =========================================================================
    // SqlConsistency Tests
    // =========================================================================

    #[test]
    fn test_sql_consistency_default() {
        let consistency: SqlConsistency = Default::default();
        assert_eq!(consistency, SqlConsistency::Linearizable);
    }

    #[test]
    fn test_sql_consistency_debug() {
        assert_eq!(format!("{:?}", SqlConsistency::Linearizable), "Linearizable");
        assert_eq!(format!("{:?}", SqlConsistency::Stale), "Stale");
    }

    #[test]
    fn test_sql_consistency_clone() {
        let c1 = SqlConsistency::Stale;
        let c2 = c1;
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_sql_consistency_serialization() {
        let consistency = SqlConsistency::Linearizable;
        let json = serde_json::to_string(&consistency).expect("serialize");
        let recovered: SqlConsistency = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(consistency, recovered);
    }

    // =========================================================================
    // SqlValue Tests
    // =========================================================================

    #[test]
    fn test_sql_value_null() {
        let val = SqlValue::Null;
        assert!(matches!(val, SqlValue::Null));
    }

    #[test]
    fn test_sql_value_integer() {
        let val = SqlValue::Integer(42);
        assert!(matches!(val, SqlValue::Integer(42)));
    }

    #[test]
    fn test_sql_value_integer_negative() {
        let val = SqlValue::Integer(-100);
        assert!(matches!(val, SqlValue::Integer(-100)));
    }

    #[test]
    fn test_sql_value_real() {
        let val = SqlValue::Real(3.14159);
        if let SqlValue::Real(v) = val {
            assert!((v - 3.14159).abs() < f64::EPSILON);
        } else {
            panic!("Expected Real");
        }
    }

    #[test]
    fn test_sql_value_text() {
        let val = SqlValue::Text("hello".to_string());
        assert!(matches!(val, SqlValue::Text(s) if s == "hello"));
    }

    #[test]
    fn test_sql_value_blob() {
        let val = SqlValue::Blob(vec![1, 2, 3, 4]);
        assert!(matches!(val, SqlValue::Blob(b) if b == vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_sql_value_serialization_roundtrip() {
        let values = vec![
            SqlValue::Null,
            SqlValue::Integer(i64::MAX),
            SqlValue::Integer(i64::MIN),
            SqlValue::Real(f64::MAX),
            SqlValue::Text("test string".to_string()),
            SqlValue::Blob(vec![0xFF, 0x00, 0xAB]),
        ];

        for val in values {
            let json = serde_json::to_string(&val).expect("serialize");
            let recovered: SqlValue = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(val, recovered);
        }
    }

    #[test]
    fn test_sql_value_clone() {
        let val = SqlValue::Text("cloneable".to_string());
        let cloned = val.clone();
        assert_eq!(val, cloned);
    }

    // =========================================================================
    // SqlColumnInfo Tests
    // =========================================================================

    #[test]
    fn test_sql_column_info_construction() {
        let col = SqlColumnInfo {
            name: "user_id".to_string(),
        };
        assert_eq!(col.name, "user_id");
    }

    #[test]
    fn test_sql_column_info_debug() {
        let col = SqlColumnInfo {
            name: "test".to_string(),
        };
        let debug = format!("{:?}", col);
        assert!(debug.contains("SqlColumnInfo"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_sql_column_info_serialization() {
        let col = SqlColumnInfo {
            name: "column_name".to_string(),
        };
        let json = serde_json::to_string(&col).expect("serialize");
        let recovered: SqlColumnInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(col, recovered);
    }

    // =========================================================================
    // SqlQueryRequest Tests
    // =========================================================================

    #[test]
    fn test_sql_query_request_default() {
        let req: SqlQueryRequest = Default::default();
        assert_eq!(req.query, "");
        assert!(req.params.is_empty());
        assert_eq!(req.consistency, SqlConsistency::Linearizable);
        assert!(req.limit.is_none());
        assert!(req.timeout_ms.is_none());
    }

    #[test]
    fn test_sql_query_request_construction() {
        let req = SqlQueryRequest {
            query: "SELECT * FROM users".to_string(),
            params: vec![SqlValue::Integer(1)],
            consistency: SqlConsistency::Stale,
            limit: Some(100),
            timeout_ms: Some(5000),
        };

        assert_eq!(req.query, "SELECT * FROM users");
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.consistency, SqlConsistency::Stale);
        assert_eq!(req.limit, Some(100));
        assert_eq!(req.timeout_ms, Some(5000));
    }

    #[test]
    fn test_sql_query_request_serialization() {
        let req = SqlQueryRequest {
            query: "SELECT id FROM items".to_string(),
            params: vec![SqlValue::Text("test".to_string())],
            consistency: SqlConsistency::Linearizable,
            limit: Some(50),
            timeout_ms: None,
        };

        let json = serde_json::to_string(&req).expect("serialize");
        let recovered: SqlQueryRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(req, recovered);
    }

    // =========================================================================
    // SqlQueryResult Tests
    // =========================================================================

    #[test]
    fn test_sql_query_result_construction() {
        let result = SqlQueryResult {
            columns: vec![SqlColumnInfo { name: "id".to_string() }],
            rows: vec![vec![SqlValue::Integer(1)]],
            row_count: 1,
            is_truncated: false,
            execution_time_ms: 5,
        };

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.row_count, 1);
        assert!(!result.is_truncated);
        assert_eq!(result.execution_time_ms, 5);
    }

    #[test]
    fn test_sql_query_result_empty() {
        let result = SqlQueryResult {
            columns: vec![],
            rows: vec![],
            row_count: 0,
            is_truncated: false,
            execution_time_ms: 1,
        };

        assert!(result.columns.is_empty());
        assert!(result.rows.is_empty());
    }

    #[test]
    fn test_sql_query_result_truncated() {
        let result = SqlQueryResult {
            columns: vec![],
            rows: vec![],
            row_count: 1000,
            is_truncated: true,
            execution_time_ms: 100,
        };

        assert!(result.is_truncated);
    }

    // =========================================================================
    // SqlQueryError Tests
    // =========================================================================

    #[test]
    fn test_sql_query_error_query_not_allowed() {
        let err = SqlQueryError::QueryNotAllowed {
            reason: "write operation".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("not allowed"));
        assert!(display.contains("write operation"));
    }

    #[test]
    fn test_sql_query_error_syntax_error() {
        let err = SqlQueryError::SyntaxError {
            message: "near 'SELEC'".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("syntax error"));
    }

    #[test]
    fn test_sql_query_error_execution_failed() {
        let err = SqlQueryError::ExecutionFailed {
            reason: "table not found".to_string(),
        };
        assert!(format!("{}", err).contains("execution failed"));
    }

    #[test]
    fn test_sql_query_error_timeout() {
        let err = SqlQueryError::Timeout { duration_ms: 5000 };
        let display = format!("{}", err);
        assert!(display.contains("timed out"));
        assert!(display.contains("5000"));
    }

    #[test]
    fn test_sql_query_error_not_leader() {
        let err = SqlQueryError::NotLeader { leader: Some(3) };
        let display = format!("{}", err);
        assert!(display.contains("not leader"));
        assert!(display.contains("3"));
    }

    #[test]
    fn test_sql_query_error_query_too_large() {
        let err = SqlQueryError::QueryTooLarge {
            size: 2000000,
            max: 1000000,
        };
        let display = format!("{}", err);
        assert!(display.contains("2000000"));
        assert!(display.contains("1000000"));
    }

    #[test]
    fn test_sql_query_error_too_many_params() {
        let err = SqlQueryError::TooManyParams { count: 200, max: 100 };
        let display = format!("{}", err);
        assert!(display.contains("200"));
        assert!(display.contains("100"));
    }

    #[test]
    fn test_sql_query_error_not_supported() {
        let err = SqlQueryError::NotSupported {
            backend: "redb".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("not supported"));
        assert!(display.contains("redb"));
    }

    #[test]
    fn test_sql_query_error_clone() {
        let err = SqlQueryError::Timeout { duration_ms: 1000 };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // =========================================================================
    // validate_sql_request Tests
    // =========================================================================

    #[test]
    fn test_validate_sql_request_valid() {
        let req = SqlQueryRequest {
            query: "SELECT * FROM users".to_string(),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: Some(100),
            timeout_ms: Some(5000),
        };
        assert!(validate_sql_request(&req).is_ok());
    }

    #[test]
    fn test_validate_sql_request_query_too_large() {
        let req = SqlQueryRequest {
            query: "x".repeat(MAX_SQL_QUERY_SIZE as usize + 1),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: None,
            timeout_ms: None,
        };
        let result = validate_sql_request(&req);
        assert!(matches!(result, Err(SqlQueryError::QueryTooLarge { .. })));
    }

    #[test]
    fn test_validate_sql_request_too_many_params() {
        let req = SqlQueryRequest {
            query: "SELECT * FROM users".to_string(),
            params: (0..MAX_SQL_PARAMS as i64 + 1).map(SqlValue::Integer).collect(),
            consistency: SqlConsistency::Linearizable,
            limit: None,
            timeout_ms: None,
        };
        let result = validate_sql_request(&req);
        assert!(matches!(result, Err(SqlQueryError::TooManyParams { .. })));
    }

    #[test]
    fn test_validate_sql_request_limit_too_high() {
        let req = SqlQueryRequest {
            query: "SELECT * FROM users".to_string(),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: Some(MAX_SQL_RESULT_ROWS + 1),
            timeout_ms: None,
        };
        let result = validate_sql_request(&req);
        assert!(matches!(result, Err(SqlQueryError::QueryNotAllowed { .. })));
    }

    #[test]
    fn test_validate_sql_request_timeout_too_high() {
        let req = SqlQueryRequest {
            query: "SELECT * FROM users".to_string(),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: None,
            timeout_ms: Some(MAX_SQL_TIMEOUT_MS + 1),
        };
        let result = validate_sql_request(&req);
        assert!(matches!(result, Err(SqlQueryError::QueryNotAllowed { .. })));
    }

    #[test]
    fn test_validate_sql_request_at_limits() {
        let req = SqlQueryRequest {
            query: "x".repeat(MAX_SQL_QUERY_SIZE as usize),
            params: (0..MAX_SQL_PARAMS as i64).map(SqlValue::Integer).collect(),
            consistency: SqlConsistency::Linearizable,
            limit: Some(MAX_SQL_RESULT_ROWS),
            timeout_ms: Some(MAX_SQL_TIMEOUT_MS),
        };
        assert!(validate_sql_request(&req).is_ok());
    }

    // =========================================================================
    // effective_sql_limit Tests
    // =========================================================================

    #[test]
    fn test_effective_sql_limit_none() {
        assert_eq!(effective_sql_limit(None), DEFAULT_SQL_RESULT_ROWS);
    }

    #[test]
    fn test_effective_sql_limit_under_max() {
        assert_eq!(effective_sql_limit(Some(500)), 500);
    }

    #[test]
    fn test_effective_sql_limit_over_max() {
        assert_eq!(effective_sql_limit(Some(MAX_SQL_RESULT_ROWS + 1000)), MAX_SQL_RESULT_ROWS);
    }

    #[test]
    fn test_effective_sql_limit_at_max() {
        assert_eq!(effective_sql_limit(Some(MAX_SQL_RESULT_ROWS)), MAX_SQL_RESULT_ROWS);
    }

    #[test]
    fn test_effective_sql_limit_zero() {
        assert_eq!(effective_sql_limit(Some(0)), 0);
    }

    // =========================================================================
    // effective_sql_timeout_ms Tests
    // =========================================================================

    #[test]
    fn test_effective_sql_timeout_none() {
        assert_eq!(effective_sql_timeout_ms(None), DEFAULT_SQL_TIMEOUT_MS);
    }

    #[test]
    fn test_effective_sql_timeout_under_max() {
        assert_eq!(effective_sql_timeout_ms(Some(5000)), 5000);
    }

    #[test]
    fn test_effective_sql_timeout_over_max() {
        assert_eq!(effective_sql_timeout_ms(Some(MAX_SQL_TIMEOUT_MS + 1000)), MAX_SQL_TIMEOUT_MS);
    }

    #[test]
    fn test_effective_sql_timeout_at_max() {
        assert_eq!(effective_sql_timeout_ms(Some(MAX_SQL_TIMEOUT_MS)), MAX_SQL_TIMEOUT_MS);
    }

    // =========================================================================
    // validate_sql_query Extended Tests
    // =========================================================================

    #[test]
    fn test_with_cte_queries() {
        assert!(validate_sql_query("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
        assert!(validate_sql_query("with recursive tree AS (SELECT 1) SELECT * FROM tree").is_ok());
    }

    #[test]
    fn test_subqueries() {
        assert!(validate_sql_query("SELECT * FROM (SELECT id FROM users)").is_ok());
        assert!(validate_sql_query("SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)").is_ok());
    }

    #[test]
    fn test_forbidden_in_cte() {
        // DELETE inside CTE should be caught
        assert!(validate_sql_query("WITH deleted AS (DELETE FROM users RETURNING *) SELECT * FROM deleted").is_err());
    }

    #[test]
    fn test_all_forbidden_keywords() {
        let forbidden = vec![
            "INSERT INTO users VALUES (1)",
            "UPDATE users SET name = 'test'",
            "DELETE FROM users",
            "REPLACE INTO users VALUES (1)",
            "CREATE TABLE test (id INT)",
            "DROP TABLE users",
            "ALTER TABLE users ADD COLUMN x INT",
            "TRUNCATE users",
            "BEGIN TRANSACTION",
            "COMMIT",
            "ROLLBACK",
            "SAVEPOINT sp1",
            "RELEASE sp1",
            "ATTACH DATABASE 'x' AS x",
            "DETACH DATABASE x",
            "VACUUM",
            "REINDEX",
            "ANALYZE users",
            "PRAGMA table_info(users)",
        ];

        for query in forbidden {
            assert!(validate_sql_query(query).is_err(), "Query should be rejected: {}", query);
        }
    }

    #[test]
    fn test_keyword_in_string_literal() {
        // Keywords in string literals should be allowed (column values containing keywords)
        // Note: This depends on the implementation - current impl may not handle this
        assert!(validate_sql_query("SELECT * FROM items WHERE status = 'DELETED'").is_ok());
    }

    #[test]
    fn test_keyword_as_table_prefix() {
        // Table/column names containing keywords should be allowed
        assert!(validate_sql_query("SELECT deleted_at FROM items").is_ok());
        assert!(validate_sql_query("SELECT * FROM update_log").is_ok());
    }

    #[test]
    fn test_complex_valid_queries() {
        let valid = vec![
            "SELECT u.*, o.total FROM users u JOIN orders o ON u.id = o.user_id",
            "SELECT COUNT(*), AVG(price) FROM products GROUP BY category HAVING COUNT(*) > 5",
            "SELECT * FROM items ORDER BY created_at DESC LIMIT 10 OFFSET 20",
            "SELECT DISTINCT category FROM products",
            "SELECT * FROM users WHERE name LIKE '%test%'",
        ];

        for query in valid {
            assert!(validate_sql_query(query).is_ok(), "Query should be valid: {}", query);
        }
    }

    #[test]
    fn test_invalid_start_keywords() {
        let invalid_starts = vec![
            "EXPLAIN SELECT * FROM users",
            "SHOW TABLES",
            "DESCRIBE users",
            "USE database",
        ];

        for query in invalid_starts {
            assert!(validate_sql_query(query).is_err(), "Query should be rejected: {}", query);
        }
    }
}
