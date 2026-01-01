//! Integration tests for write batching functionality.
//!
//! These tests verify that:
//! 1. Write batching is correctly enabled by default
//! 2. Batched writes are correctly persisted and readable
//! 3. SQL queries correctly return batched data
//! 4. Different batch configurations work correctly
//! 5. Concurrent writes are properly batched and complete
//!
//! Tiger Style: Fixed timeouts, bounded operations, explicit error handling.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::KeyValueStoreError;
use aspen::api::ReadRequest;
#[cfg(feature = "sql")]
use aspen_api::SqlConsistency;
#[cfg(feature = "sql")]
use aspen_api::SqlQueryExecutor;
#[cfg(feature = "sql")]
use aspen_api::SqlQueryRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::BatchConfig;
use aspen::raft::storage::StorageBackend;
use openraft::ServerState;
use tempfile::TempDir;

/// Helper to create a single-node cluster with batching enabled.
async fn setup_node_with_batching(temp_dir: &TempDir, batch_config: BatchConfig) -> anyhow::Result<Node> {
    let data_dir = temp_dir.path().join("node-1");

    let mut node = NodeBuilder::new(NodeId(1), &data_dir)
        .with_storage(StorageBackend::Redb)
        .with_cookie("batch-test")
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .with_write_batching(batch_config)
        .start()
        .await?;

    node.spawn_router();

    let raft_node = node.raft_node();
    let endpoint_addr = node.endpoint_addr();
    raft_node
        .init(InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, endpoint_addr)],
        })
        .await?;

    raft_node
        .raft()
        .wait(Some(Duration::from_secs(5)))
        .state(ServerState::Leader, "node becomes leader")
        .await?;

    Ok(node)
}

/// Helper to create a single-node cluster without batching.
async fn setup_node_without_batching(temp_dir: &TempDir) -> anyhow::Result<Node> {
    let data_dir = temp_dir.path().join("node-1");

    let mut node = NodeBuilder::new(NodeId(1), &data_dir)
        .with_storage(StorageBackend::Redb)
        .with_cookie("batch-test")
        .with_gossip(false)
        .with_mdns(false)
        .with_heartbeat_interval_ms(100)
        .with_election_timeout_ms(300, 600)
        .without_write_batching()
        .start()
        .await?;

    node.spawn_router();

    let raft_node = node.raft_node();
    let endpoint_addr = node.endpoint_addr();
    raft_node
        .init(InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, endpoint_addr)],
        })
        .await?;

    raft_node
        .raft()
        .wait(Some(Duration::from_secs(5)))
        .state(ServerState::Leader, "node becomes leader")
        .await?;

    Ok(node)
}

// =============================================================================
// Basic Batching Tests
// =============================================================================

/// Test that single writes work with batching enabled.
#[tokio::test]
async fn test_single_write_with_batching() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write a single key
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "test-key".to_string(),
                value: "test-value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    // Read it back
    let result = raft_node.read(ReadRequest::new("test-key")).await.expect("read should succeed");

    assert_eq!(result.kv.map(|kv| kv.value), Some("test-value".to_string()));

    node.shutdown().await.expect("shutdown");
}

/// Test that multiple sequential writes work with batching.
#[tokio::test]
async fn test_multiple_sequential_writes_with_batching() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write multiple keys
    for i in 0..10 {
        raft_node
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key-{}", i),
                    value: format!("value-{}", i),
                },
            })
            .await
            .expect("write should succeed");
    }

    // Verify all keys
    for i in 0..10 {
        let result = raft_node.read(ReadRequest::new(format!("key-{}", i))).await.expect("read should succeed");

        assert_eq!(result.kv.map(|kv| kv.value), Some(format!("value-{}", i)));
    }

    node.shutdown().await.expect("shutdown");
}

// =============================================================================
// Concurrent Write Tests
// =============================================================================

/// Test that concurrent writes are properly batched.
#[tokio::test]
async fn test_concurrent_writes_batched() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node().clone();
    let counter = Arc::new(AtomicU64::new(0));

    // Spawn 50 concurrent writes
    let mut handles = Vec::with_capacity(50);
    for _ in 0..50 {
        let rn = raft_node.clone();
        let idx = counter.fetch_add(1, Ordering::Relaxed);
        handles.push(tokio::spawn(async move {
            rn.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("concurrent-key-{:04}", idx),
                    value: format!("concurrent-value-{:04}", idx),
                },
            })
            .await
        }));
    }

    // Wait for all writes to complete
    for h in handles {
        h.await.expect("task panic").expect("write failed");
    }

    // Verify all 50 keys exist
    for i in 0..50 {
        let result = raft_node
            .read(ReadRequest::new(format!("concurrent-key-{:04}", i)))
            .await
            .expect("read should succeed");

        assert_eq!(
            result.kv.map(|kv| kv.value),
            Some(format!("concurrent-value-{:04}", i)),
            "Key concurrent-key-{:04} should exist",
            i
        );
    }

    node.shutdown().await.expect("shutdown");
}

/// Test that large concurrent write volume works correctly.
///
/// Note: Due to storage limits (MAX_SETMULTI_KEYS=100), writes are batched
/// into groups of 100 max. For 200 writes, we'll have at least 2 batches.
#[tokio::test]
async fn test_large_concurrent_write_volume() {
    let temp_dir = TempDir::new().unwrap();
    // Use high throughput config for larger volume
    let node = setup_node_with_batching(&temp_dir, BatchConfig::high_throughput()).await.expect("setup node");

    let raft_node = node.raft_node().clone();
    let counter = Arc::new(AtomicU64::new(0));

    // Spawn many concurrent writes in two waves of 100 to respect batch limits
    for _ in 0..2 {
        let mut handles = Vec::with_capacity(100);
        for _ in 0..100 {
            let rn = raft_node.clone();
            let idx = counter.fetch_add(1, Ordering::Relaxed);
            handles.push(tokio::spawn(async move {
                rn.write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("volume-key-{:06}", idx),
                        value: format!("volume-value-{:06}", idx),
                    },
                })
                .await
            }));
        }

        // Wait for this wave to complete before starting next
        for h in handles {
            h.await.expect("task panic").expect("write failed");
        }
    }

    // Verify sample of keys from both waves
    for i in [0, 50, 100, 150, 199] {
        let result =
            raft_node.read(ReadRequest::new(format!("volume-key-{:06}", i))).await.expect("read should succeed");

        assert!(result.kv.is_some(), "Key volume-key-{:06} should exist", i);
    }

    node.shutdown().await.expect("shutdown");
}

// =============================================================================
// SQL Verification Tests
// =============================================================================

/// Test that batched writes are visible via SQL queries.
#[cfg(feature = "sql")]
#[tokio::test]
async fn test_batched_writes_visible_via_sql() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node().clone();

    // Write 100 keys concurrently
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::with_capacity(100);
    for _ in 0..100 {
        let rn = raft_node.clone();
        let idx = counter.fetch_add(1, Ordering::Relaxed);
        handles.push(tokio::spawn(async move {
            rn.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("sql-test-key-{:04}", idx),
                    value: format!("sql-test-value-{:04}", idx),
                },
            })
            .await
        }));
    }

    for h in handles {
        h.await.expect("task panic").expect("write failed");
    }

    // Verify via SQL COUNT(*)
    let count_result = raft_node
        .execute_sql(SqlQueryRequest {
            query: "SELECT COUNT(*) FROM kv WHERE key LIKE 'sql-test-key-%'".to_string(),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: Some(1),
            timeout_ms: Some(30000),
        })
        .await
        .expect("sql query failed");

    assert!(!count_result.rows.is_empty(), "COUNT(*) should return a row");

    // Verify via SQL SELECT *
    let select_result = raft_node
        .execute_sql(SqlQueryRequest {
            query: "SELECT * FROM kv WHERE key LIKE 'sql-test-key-%'".to_string(),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: Some(1000),
            timeout_ms: Some(30000),
        })
        .await
        .expect("sql query failed");

    assert_eq!(select_result.rows.len(), 100, "Should return exactly 100 rows");

    node.shutdown().await.expect("shutdown");
}

/// Test that SQL point lookup works on batched data.
#[cfg(feature = "sql")]
#[tokio::test]
async fn test_sql_point_lookup_on_batched_data() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write a specific key
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "unique-lookup-key".to_string(),
                value: "unique-lookup-value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    // Query via SQL point lookup
    let result = raft_node
        .execute_sql(SqlQueryRequest {
            query: "SELECT key, value FROM kv WHERE key = 'unique-lookup-key'".to_string(),
            params: vec![],
            consistency: SqlConsistency::Linearizable,
            limit: Some(1),
            timeout_ms: Some(5000),
        })
        .await
        .expect("sql query failed");

    assert_eq!(result.rows.len(), 1, "Should find exactly one row");

    node.shutdown().await.expect("shutdown");
}

// =============================================================================
// Configuration Variant Tests
// =============================================================================

/// Test low latency batch config.
#[tokio::test]
async fn test_low_latency_batch_config() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::low_latency()).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write and read a key
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "low-latency-key".to_string(),
                value: "low-latency-value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    let result = raft_node.read(ReadRequest::new("low-latency-key")).await.expect("read should succeed");

    assert_eq!(result.kv.map(|kv| kv.value), Some("low-latency-value".to_string()));

    node.shutdown().await.expect("shutdown");
}

/// Test high throughput batch config.
#[tokio::test]
async fn test_high_throughput_batch_config() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::high_throughput()).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write and read a key
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "high-throughput-key".to_string(),
                value: "high-throughput-value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    let result = raft_node.read(ReadRequest::new("high-throughput-key")).await.expect("read should succeed");

    assert_eq!(result.kv.map(|kv| kv.value), Some("high-throughput-value".to_string()));

    node.shutdown().await.expect("shutdown");
}

/// Test writes work without batching.
#[tokio::test]
async fn test_writes_without_batching() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_without_batching(&temp_dir).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write and read a key
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "no-batch-key".to_string(),
                value: "no-batch-value".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    let result = raft_node.read(ReadRequest::new("no-batch-key")).await.expect("read should succeed");

    assert_eq!(result.kv.map(|kv| kv.value), Some("no-batch-value".to_string()));

    node.shutdown().await.expect("shutdown");
}

// =============================================================================
// Delete Operation Tests
// =============================================================================

/// Test that delete operations work through batching.
#[tokio::test]
async fn test_delete_through_batching() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node();

    // Write a key
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "delete-me".to_string(),
                value: "will-be-deleted".to_string(),
            },
        })
        .await
        .expect("write should succeed");

    // Verify it exists
    let result = raft_node.read(ReadRequest::new("delete-me")).await.expect("read should succeed");
    assert!(result.kv.is_some());

    // Delete through batching
    raft_node
        .write(WriteRequest {
            command: WriteCommand::Delete {
                key: "delete-me".to_string(),
            },
        })
        .await
        .expect("delete should succeed");

    // Verify it's gone - deleted keys return NotFound error
    let result = raft_node.read(ReadRequest::new("delete-me")).await;
    assert!(
        matches!(result, Err(KeyValueStoreError::NotFound { .. })),
        "Deleted key should return NotFound, got {:?}",
        result
    );

    node.shutdown().await.expect("shutdown");
}

/// Test concurrent deletes through batching.
#[tokio::test]
async fn test_concurrent_deletes_through_batching() {
    let temp_dir = TempDir::new().unwrap();
    let node = setup_node_with_batching(&temp_dir, BatchConfig::default()).await.expect("setup node");

    let raft_node = node.raft_node().clone();

    // First, write 20 keys
    for i in 0..20 {
        raft_node
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("delete-batch-{:02}", i),
                    value: format!("value-{:02}", i),
                },
            })
            .await
            .expect("write should succeed");
    }

    // Now delete all 20 concurrently
    let mut handles = Vec::with_capacity(20);
    for i in 0..20 {
        let rn = raft_node.clone();
        handles.push(tokio::spawn(async move {
            rn.write(WriteRequest {
                command: WriteCommand::Delete {
                    key: format!("delete-batch-{:02}", i),
                },
            })
            .await
        }));
    }

    for h in handles {
        h.await.expect("task panic").expect("delete failed");
    }

    // Verify all are gone - deleted keys return NotFound error
    for i in 0..20 {
        let result = raft_node.read(ReadRequest::new(format!("delete-batch-{:02}", i))).await;
        assert!(
            matches!(result, Err(KeyValueStoreError::NotFound { .. })),
            "Key delete-batch-{:02} should return NotFound, got {:?}",
            i,
            result
        );
    }

    node.shutdown().await.expect("shutdown");
}
