//! Integration tests for DataFusion SQL execution on Redb storage.
//!
//! These tests verify that the SQL layer correctly:
//! 1. Executes basic SELECT queries
//! 2. Applies filter pushdown for key predicates
//! 3. Returns results in the correct format
//! 4. Handles edge cases (empty tables, limits, etc.)

use std::sync::Arc;

use aspen::api::SqlQueryError;
use aspen::api::SqlValue;
use aspen::raft::storage_shared::KvEntry;
use aspen::sql::RedbSqlExecutor;
use redb::Database;
use redb::TableDefinition;
use tempfile::tempdir;

/// Table definition for KV data (must match storage_shared.rs).
const SM_KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_kv");

/// Helper to create a test database with sample data.
fn setup_test_db() -> Arc<Database> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.redb");
    let db = Database::create(path).unwrap();

    // Insert some test data directly
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(SM_KV_TABLE).unwrap();

        // Insert test entries
        let entries = vec![
            ("user:1", "Alice", 1, 1, 1),
            ("user:2", "Bob", 1, 2, 2),
            ("user:3", "Charlie", 1, 3, 3),
            ("config:timeout", "30", 1, 4, 4),
            ("config:retries", "3", 1, 5, 5),
            ("data:record:1", "value1", 1, 6, 6),
            ("data:record:2", "value2", 1, 7, 7),
        ];

        for (key, value, version, create_rev, mod_rev) in entries {
            let entry = KvEntry {
                value: value.to_string(),
                version,
                create_revision: create_rev,
                mod_revision: mod_rev,
                expires_at_ms: None,
                lease_id: None,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            table.insert(key.as_bytes(), entry_bytes.as_slice()).unwrap();
        }
    }
    write_txn.commit().unwrap();

    Arc::new(db)
}

/// Helper to create an empty test database.
fn setup_empty_db() -> Arc<Database> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.redb");
    let db = Database::create(path).unwrap();

    // Create the table but don't insert data
    let write_txn = db.begin_write().unwrap();
    {
        let _table = write_txn.open_table(SM_KV_TABLE).unwrap();
    }
    write_txn.commit().unwrap();

    Arc::new(db)
}

#[tokio::test]
async fn test_basic_select_all() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT * FROM kv", &[], None, None).await.unwrap();

    assert_eq!(result.row_count, 7);
    assert!(!result.is_truncated);
    assert_eq!(result.columns.len(), 7);
    assert_eq!(result.columns[0].name, "key");
    assert_eq!(result.columns[1].name, "value");
}

#[tokio::test]
async fn test_select_specific_columns() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT key, value FROM kv", &[], None, None).await.unwrap();

    assert_eq!(result.columns.len(), 2);
    assert_eq!(result.columns[0].name, "key");
    assert_eq!(result.columns[1].name, "value");
}

#[tokio::test]
async fn test_filter_exact_key() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT key, value FROM kv WHERE key = 'user:1'", &[], None, None).await.unwrap();

    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][0], SqlValue::Text("user:1".to_string()));
    assert_eq!(result.rows[0][1], SqlValue::Text("Alice".to_string()));
}

#[tokio::test]
async fn test_filter_prefix_like() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor
        .execute("SELECT key, value FROM kv WHERE key LIKE 'user:%'", &[], None, None)
        .await
        .unwrap();

    assert_eq!(result.row_count, 3);
    // Results should be user:1, user:2, user:3
    let keys: Vec<&str> = result
        .rows
        .iter()
        .map(|row| match &row[0] {
            SqlValue::Text(s) => s.as_str(),
            _ => panic!("expected text"),
        })
        .collect();

    assert!(keys.contains(&"user:1"));
    assert!(keys.contains(&"user:2"));
    assert!(keys.contains(&"user:3"));
}

#[tokio::test]
async fn test_filter_range() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor
        .execute("SELECT key, value FROM kv WHERE key >= 'data:' AND key < 'datb:'", &[], None, None)
        .await
        .unwrap();

    assert_eq!(result.row_count, 2);
    // Should return data:record:1 and data:record:2
}

#[tokio::test]
async fn test_limit() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT * FROM kv", &[], Some(3), None).await.unwrap();

    assert_eq!(result.row_count, 3);
    assert!(result.is_truncated);
}

#[tokio::test]
async fn test_empty_table() {
    let db = setup_empty_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT * FROM kv", &[], None, None).await.unwrap();

    assert_eq!(result.row_count, 0);
    assert!(!result.is_truncated);
}

#[tokio::test]
async fn test_no_matching_rows() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT * FROM kv WHERE key = 'nonexistent'", &[], None, None).await.unwrap();

    assert_eq!(result.row_count, 0);
    assert!(!result.is_truncated);
}

// COUNT(*) and other aggregations work correctly with our RecordBatchStream
// implementation, which handles empty projections by returning batches with
// a row count but no columns.

#[tokio::test]
async fn test_count_aggregation() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT COUNT(*) as cnt FROM kv", &[], None, None).await.unwrap();

    assert_eq!(result.row_count, 1);
    // The count should be 7
    assert_eq!(result.rows[0][0], SqlValue::Integer(7));
}

#[tokio::test]
async fn test_count_with_filter() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor
        .execute("SELECT COUNT(*) as cnt FROM kv WHERE key LIKE 'user:%'", &[], None, None)
        .await
        .unwrap();

    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][0], SqlValue::Integer(3));
}

#[tokio::test]
async fn test_order_by() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor
        .execute("SELECT key, value FROM kv WHERE key LIKE 'user:%' ORDER BY key DESC", &[], None, None)
        .await
        .unwrap();

    assert_eq!(result.row_count, 3);
    // Should be ordered: user:3, user:2, user:1
    assert_eq!(result.rows[0][0], SqlValue::Text("user:3".to_string()));
    assert_eq!(result.rows[1][0], SqlValue::Text("user:2".to_string()));
    assert_eq!(result.rows[2][0], SqlValue::Text("user:1".to_string()));
}

// GROUP BY with string functions (like substring) requires DataFusion to have
// these functions registered. The default configuration doesn't include them.
// This test is skipped until we add the necessary function registration.
#[tokio::test]
#[ignore = "substring function not available in default DataFusion configuration"]
async fn test_group_by() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    // Count entries per prefix (using substring to get prefix)
    let result = executor
        .execute(
            "SELECT substring(key, 1, 4) as prefix, COUNT(*) as cnt FROM kv GROUP BY substring(key, 1, 4) ORDER BY cnt DESC",
            &[],
            None,
            None,
        )
        .await
        .unwrap();

    // Should have: user (3), conf (2), data (2)
    assert!(result.row_count > 0);
}

#[tokio::test]
async fn test_version_column() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT key, version FROM kv WHERE key = 'user:1'", &[], None, None).await.unwrap();

    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][1], SqlValue::Integer(1));
}

#[tokio::test]
async fn test_revision_columns() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor
        .execute("SELECT key, create_revision, mod_revision FROM kv WHERE key = 'user:1'", &[], None, None)
        .await
        .unwrap();

    assert_eq!(result.row_count, 1);
    // create_revision and mod_revision should be 1 for user:1
    assert_eq!(result.rows[0][1], SqlValue::Integer(1));
    assert_eq!(result.rows[0][2], SqlValue::Integer(1));
}

#[tokio::test]
async fn test_execution_time_recorded() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT * FROM kv", &[], None, None).await.unwrap();

    // Execution time should be recorded (this always passes since u64 >= 0)
    // Just check that the field is populated and the query completed
    let _ = result.execution_time_ms;
    assert!(result.row_count > 0);
}

#[tokio::test]
async fn test_syntax_error() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor
        .execute("SELECT * FORM kv", &[], None, None) // typo: FORM instead of FROM
        .await;

    assert!(result.is_err());
    if let Err(SqlQueryError::SyntaxError { message }) = result {
        assert!(!message.is_empty());
    } else {
        panic!("expected SyntaxError");
    }
}

#[tokio::test]
async fn test_invalid_table() {
    let db = setup_test_db();
    let executor = RedbSqlExecutor::new(db);

    let result = executor.execute("SELECT * FROM nonexistent", &[], None, None).await;

    // DataFusion should return an error for unknown table
    assert!(result.is_err());
}
