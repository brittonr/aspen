//! SQL query validation for read-only query safety.
//!
//! This module provides multi-layer defense against SQL injection and write operations:
//!
//! 1. **Statement validation**: Only SELECT and WITH...SELECT (CTEs) are allowed
//! 2. **Keyword blocking**: Forbidden keywords like INSERT, UPDATE, DELETE, DROP, etc.
//! 3. **Parameterized queries**: User values passed as parameters, not in query string
//! 4. **Read-only connection**: `PRAGMA query_only = ON` as defense-in-depth (in executor)
//!
//! # Tiger Style
//!
//! - Query size limits enforced by `validate_sql_request()` in api/mod.rs
//! - This module focuses on semantic validation (is the query read-only?)

use crate::api::SqlQueryError;

/// Keywords that indicate write operations.
/// These are blocked even in subqueries or CTEs.
const FORBIDDEN_KEYWORDS: &[&str] = &[
    // Write operations
    "INSERT",
    "UPDATE",
    "DELETE",
    "REPLACE",
    "UPSERT",
    // Schema modifications
    "CREATE",
    "DROP",
    "ALTER",
    "TRUNCATE",
    // Transaction control (not needed for single queries)
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
    "SAVEPOINT",
    "RELEASE",
    // Database operations
    "ATTACH",
    "DETACH",
    "VACUUM",
    "REINDEX",
    "ANALYZE",
    // Pragma that could modify state
    "PRAGMA",
];

/// Validate that a SQL query is a read-only SELECT statement.
///
/// Performs multi-layer validation:
/// 1. Checks query starts with SELECT or WITH (for CTEs)
/// 2. Scans for forbidden keywords that indicate write operations
///
/// This is defense-in-depth; the executor also uses `PRAGMA query_only = ON`.
///
/// # Arguments
///
/// * `query` - The SQL query string to validate
///
/// # Returns
///
/// * `Ok(())` if the query appears to be read-only
/// * `Err(SqlQueryError::QueryNotAllowed)` if the query contains write operations
///
/// # Examples
///
/// ```ignore
/// use aspen::api::sql_validation::validate_sql_query;
///
/// // Valid SELECT queries
/// assert!(validate_sql_query("SELECT * FROM users").is_ok());
/// assert!(validate_sql_query("SELECT COUNT(*) FROM orders WHERE status = ?1").is_ok());
/// assert!(validate_sql_query("WITH cte AS (SELECT id FROM items) SELECT * FROM cte").is_ok());
///
/// // Invalid: write operations
/// assert!(validate_sql_query("INSERT INTO users VALUES (1, 'test')").is_err());
/// assert!(validate_sql_query("DELETE FROM users WHERE id = 1").is_err());
/// assert!(validate_sql_query("DROP TABLE users").is_err());
/// ```
pub fn validate_sql_query(query: &str) -> Result<(), SqlQueryError> {
    let trimmed = query.trim();

    // Empty query check
    if trimmed.is_empty() {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: "empty query".into(),
        });
    }

    // Check query starts with SELECT or WITH (case-insensitive)
    let upper = trimmed.to_uppercase();
    if !upper.starts_with("SELECT") && !upper.starts_with("WITH") {
        return Err(SqlQueryError::QueryNotAllowed {
            reason: format!("query must start with SELECT or WITH, got: {}", &trimmed[..trimmed.len().min(20)]),
        });
    }

    // Scan for forbidden keywords
    // We check for word boundaries to avoid false positives like "UPDATES" column name
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
///
/// This avoids false positives like:
/// - Column named "UPDATES" matching "UPDATE"
/// - Table named "DELETED_ITEMS" matching "DELETE"
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

/// Check if a byte is a valid SQL identifier character.
fn is_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_select_queries() {
        // Basic SELECT
        assert!(validate_sql_query("SELECT * FROM users").is_ok());
        assert!(validate_sql_query("select * from users").is_ok()); // lowercase
        assert!(validate_sql_query("  SELECT * FROM users  ").is_ok()); // whitespace

        // With WHERE clause
        assert!(validate_sql_query("SELECT id, name FROM users WHERE id = ?1").is_ok());

        // With JOINs
        assert!(validate_sql_query("SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id").is_ok());

        // With aggregations
        assert!(validate_sql_query("SELECT COUNT(*), AVG(price) FROM products").is_ok());

        // With subquery
        assert!(validate_sql_query("SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)").is_ok());

        // CTE (Common Table Expression)
        assert!(
            validate_sql_query(
                "WITH active_users AS (SELECT id FROM users WHERE active = 1) SELECT * FROM active_users"
            )
            .is_ok()
        );
    }

    #[test]
    fn test_forbidden_write_operations() {
        // INSERT
        let err = validate_sql_query("INSERT INTO users VALUES (1, 'test')").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // UPDATE
        let err = validate_sql_query("UPDATE users SET name = 'test' WHERE id = 1").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // DELETE
        let err = validate_sql_query("DELETE FROM users WHERE id = 1").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // REPLACE
        let err = validate_sql_query("REPLACE INTO users VALUES (1, 'test')").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));
    }

    #[test]
    fn test_forbidden_schema_operations() {
        // CREATE TABLE
        let err = validate_sql_query("CREATE TABLE test (id INTEGER)").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // DROP TABLE
        let err = validate_sql_query("DROP TABLE users").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // ALTER TABLE
        let err = validate_sql_query("ALTER TABLE users ADD COLUMN email TEXT").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));
    }

    #[test]
    fn test_forbidden_database_operations() {
        // PRAGMA
        let err = validate_sql_query("PRAGMA table_info(users)").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // ATTACH
        let err = validate_sql_query("ATTACH DATABASE 'other.db' AS other").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // VACUUM
        let err = validate_sql_query("VACUUM").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));
    }

    #[test]
    fn test_forbidden_in_subquery() {
        // INSERT hidden in what looks like a SELECT
        // This should fail because query doesn't start with SELECT
        let err = validate_sql_query("INSERT INTO log SELECT * FROM users").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        // DELETE in subquery - this still starts with SELECT but has DELETE keyword
        let err =
            validate_sql_query("SELECT * FROM users WHERE id NOT IN (DELETE FROM users RETURNING id)").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));
    }

    #[test]
    fn test_column_names_not_blocked() {
        // Column named UPDATED_AT should not trigger UPDATE block
        assert!(validate_sql_query("SELECT updated_at FROM users").is_ok());
        assert!(validate_sql_query("SELECT UPDATED_AT FROM users").is_ok());

        // Column named DELETED should not trigger DELETE block
        assert!(validate_sql_query("SELECT deleted FROM items").is_ok());

        // Table named CREATED_ITEMS should not trigger CREATE block
        assert!(validate_sql_query("SELECT * FROM created_items").is_ok());
    }

    #[test]
    fn test_empty_query() {
        let err = validate_sql_query("").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));

        let err = validate_sql_query("   ").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));
    }

    #[test]
    fn test_non_select_query() {
        let err = validate_sql_query("EXPLAIN SELECT * FROM users").unwrap_err();
        assert!(matches!(err, SqlQueryError::QueryNotAllowed { .. }));
    }

    #[test]
    fn test_keyword_detection() {
        // Keyword at start
        assert!(contains_keyword("UPDATE users SET x = 1", "UPDATE"));

        // Keyword in middle
        assert!(contains_keyword("SELECT * FROM t; DELETE FROM t", "DELETE"));

        // Not a keyword (part of identifier)
        assert!(!contains_keyword("SELECT updated_at FROM t", "UPDATE"));
        assert!(!contains_keyword("SELECT * FROM UPDATES", "UPDATE"));

        // Case sensitivity (function expects uppercase)
        assert!(contains_keyword("UPDATE", "UPDATE"));
        assert!(!contains_keyword("update", "UPDATE")); // lowercase won't match
    }
}
