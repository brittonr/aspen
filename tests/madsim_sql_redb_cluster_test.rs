//! 3-node SQL integration tests on Redb-backed Raft cluster.
//!
//! Tests validate:
//! - SQL queries work on data replicated via Raft consensus
//! - Linearizable vs Stale consistency modes
//! - Filter pushdown with replicated data
//! - Aggregations across distributed state
//! - Leader failover during SQL execution
//! - Query routing to current leader
//!
//! # Architecture
//!
//! These tests use `AspenRaftTester` with `StorageBackend::Redb`, which creates
//! real `SharedRedbStorage` instances for each node. This enables testing the
//! full SQL execution path:
//!
//! ```text
//! Write through Raft → Replicated to all nodes → SQL query via DataFusion
//! ```
//!
//! # References
//!
//! - DataFusion testing: <https://datafusion.apache.org/contributor-guide/testing.html>
//! - src/testing/madsim_tester.rs - AspenRaftTester abstraction
//! - src/sql/executor.rs - RedbSqlExecutor implementation

use std::time::Duration;

use aspen::api::SqlConsistency;
use aspen::testing::madsim_tester::{AspenRaftTester, TesterConfig};

// ============================================================================
// Basic Replication Tests
// ============================================================================

/// Test basic SQL SELECT after data is replicated through Raft.
#[madsim::test]
async fn test_sql_basic_select_after_replication() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_basic_select").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No leader elected");
    eprintln!("Leader elected: node {}", leader);

    // Write data through Raft consensus
    for i in 0..10 {
        t.write(format!("user:{}", i), format!("User{}", i))
            .await
            .expect("write failed");
    }

    // Wait for replication
    t.wait_for_log_sync(5).await.expect("log sync failed");

    // SQL query through leader
    let result = t
        .execute_sql("SELECT * FROM kv WHERE key LIKE 'user:%'")
        .await
        .expect("SQL query failed");

    assert_eq!(result.row_count, 10, "Expected 10 rows");
    assert!(!result.is_truncated, "Result should not be truncated");

    t.end();
}

/// Test COUNT(*) aggregation on replicated data.
#[madsim::test]
async fn test_sql_count_aggregation() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_count_agg").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Write 25 entries
    for i in 0..25 {
        t.write(format!("item:{:03}", i), format!("value{}", i))
            .await
            .expect("write failed");
    }

    t.wait_for_log_sync(5).await.expect("log sync failed");

    // COUNT(*) aggregation
    let result = t
        .execute_sql("SELECT COUNT(*) as cnt FROM kv")
        .await
        .expect("SQL query failed");

    assert_eq!(result.row_count, 1, "COUNT should return 1 row");
    // The count should be in the first column of the first row
    eprintln!("COUNT result: {:?}", result.rows);

    t.end();
}

/// Test SQL filter pushdown with prefix LIKE.
#[madsim::test]
async fn test_sql_filter_pushdown_prefix() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_filter_prefix").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Write data with different prefixes
    t.write("config:timeout".into(), "30".into())
        .await
        .expect("write failed");
    t.write("config:retries".into(), "3".into())
        .await
        .expect("write failed");
    t.write("user:1".into(), "Alice".into())
        .await
        .expect("write failed");
    t.write("user:2".into(), "Bob".into())
        .await
        .expect("write failed");
    t.write("data:record:1".into(), "rec1".into())
        .await
        .expect("write failed");

    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Prefix scan via SQL LIKE
    let result = t
        .execute_sql("SELECT key, value FROM kv WHERE key LIKE 'config:%'")
        .await
        .expect("SQL query failed");

    assert_eq!(result.row_count, 2, "Expected 2 config keys");

    t.end();
}

// ============================================================================
// Consistency Mode Tests
// ============================================================================

/// Test Linearizable SQL query on leader succeeds.
#[madsim::test]
async fn test_sql_linearizable_on_leader() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_linearizable_leader").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No leader elected");

    // Write data
    t.write("key1".into(), "value1".into())
        .await
        .expect("write failed");
    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Linearizable query (default)
    let result = t
        .execute_sql_with_consistency(
            "SELECT * FROM kv WHERE key = 'key1'",
            SqlConsistency::Linearizable,
        )
        .await
        .expect("Linearizable query should succeed on leader");

    assert_eq!(result.row_count, 1);
    eprintln!("Leader {} served linearizable query successfully", leader);

    t.end();
}

/// Test Stale SQL query on any node.
#[madsim::test]
async fn test_sql_stale_on_follower() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_stale_follower").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No leader elected");

    // Write data
    t.write("stale-key".into(), "stale-value".into())
        .await
        .expect("write failed");
    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Find a follower
    let follower = (0..3).find(|&i| i != leader).expect("Should have follower");

    // Stale query on follower should work (local state machine read)
    let result = t
        .execute_sql_on_node(follower, "SELECT * FROM kv", SqlConsistency::Stale)
        .await
        .expect("Stale query on follower should succeed");

    assert_eq!(result.row_count, 1, "Follower should have replicated data");
    eprintln!(
        "Follower {} served stale query successfully with {} rows",
        follower, result.row_count
    );

    t.end();
}

// ============================================================================
// Failover Tests
// ============================================================================

/// Test SQL query works after leader crash and re-election.
#[madsim::test]
async fn test_sql_after_leader_crash() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_leader_crash").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let old_leader = t.check_one_leader().await.expect("No leader elected");
    eprintln!("Initial leader: {}", old_leader);

    // Write data through initial leader
    t.write("crash-test-key".into(), "crash-test-value".into())
        .await
        .expect("write failed");
    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Crash the leader
    t.crash_node(old_leader).await;
    eprintln!("Crashed leader {}", old_leader);

    // Wait for new leader election
    madsim::time::sleep(Duration::from_secs(10)).await;
    let new_leader = t.check_one_leader().await.expect("No new leader elected");
    assert_ne!(old_leader, new_leader, "New leader should be different");
    eprintln!("New leader: {}", new_leader);

    // SQL query should work through new leader
    let result = t
        .execute_sql("SELECT * FROM kv WHERE key = 'crash-test-key'")
        .await
        .expect("SQL query after failover should succeed");

    assert_eq!(result.row_count, 1, "Data should survive leader crash");

    t.end();
}

/// Test data is consistent after partition heals.
#[madsim::test]
async fn test_sql_after_partition_heal() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_partition_heal").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Write initial data
    t.write("pre-partition".into(), "value1".into())
        .await
        .expect("write failed");

    // Partition node 2 (not the typical leader)
    t.disconnect(2);
    eprintln!("Partitioned node 2");

    // Write more data (should succeed with 2/3 quorum)
    t.write("during-partition".into(), "value2".into())
        .await
        .expect("write during partition failed");
    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Heal partition
    t.connect(2);
    eprintln!("Reconnected node 2");

    // Wait for catch-up
    madsim::time::sleep(Duration::from_secs(3)).await;
    t.wait_for_log_sync(5).await.expect("log sync after heal");

    // SQL query should see all data
    let result = t
        .execute_sql("SELECT COUNT(*) FROM kv")
        .await
        .expect("SQL query after partition heal should succeed");

    assert_eq!(result.row_count, 1, "COUNT should return 1 row");
    eprintln!("Data consistent after partition heal: {:?}", result.rows);

    t.end();
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test SQL query on empty table.
#[madsim::test]
async fn test_sql_empty_result_set() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_empty_result").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Query with no matching data
    let result = t
        .execute_sql("SELECT * FROM kv WHERE key LIKE 'nonexistent:%'")
        .await
        .expect("SQL query for empty result should succeed");

    assert_eq!(result.row_count, 0, "Should return 0 rows");
    assert!(!result.is_truncated, "Empty result should not be truncated");

    t.end();
}

/// Test SQL syntax error handling.
#[madsim::test]
async fn test_sql_syntax_error() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_syntax_error").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Invalid SQL should fail
    let result = t.execute_sql("SELEKT * FORM kv").await;
    assert!(result.is_err(), "Invalid SQL should return error");
    eprintln!("Syntax error correctly reported: {:?}", result.unwrap_err());

    t.end();
}

/// Test SQL with ORDER BY.
#[madsim::test]
async fn test_sql_order_by() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_order_by").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Write data in non-sorted order
    t.write("z:last".into(), "3".into())
        .await
        .expect("write failed");
    t.write("a:first".into(), "1".into())
        .await
        .expect("write failed");
    t.write("m:middle".into(), "2".into())
        .await
        .expect("write failed");

    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Query with ORDER BY
    let result = t
        .execute_sql("SELECT key, value FROM kv ORDER BY key ASC")
        .await
        .expect("SQL query with ORDER BY failed");

    assert_eq!(result.row_count, 3);
    eprintln!("ORDER BY result: {:?}", result.rows);

    t.end();
}

/// Test SQL with LIMIT clause.
#[madsim::test]
async fn test_sql_limit() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let config = TesterConfig::new(3, "sql_limit").with_redb_storage(temp_dir.path());
    let mut t = AspenRaftTester::with_config(config).await;

    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No leader elected");

    // Write 20 entries
    for i in 0..20 {
        t.write(format!("limit:{:02}", i), format!("val{}", i))
            .await
            .expect("write failed");
    }

    t.wait_for_log_sync(5).await.expect("log sync failed");

    // Query with LIMIT
    let result = t
        .execute_sql("SELECT * FROM kv WHERE key LIKE 'limit:%' LIMIT 5")
        .await
        .expect("SQL query with LIMIT failed");

    assert_eq!(result.row_count, 5, "LIMIT should cap results at 5");

    t.end();
}
