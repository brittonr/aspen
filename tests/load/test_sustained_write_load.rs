//! Test: Sustained Write Load
//!
//! Validates that the cluster can handle sustained write operations and
//! measures baseline throughput performance.
//!
//! # Test Strategy
//!
//! 1. Start 3-node cluster
//! 2. Write 1000 key-value pairs sequentially
//! 3. Measure total time and throughput (ops/sec)
//! 4. Verify all writes succeed
//! 5. Log baseline metrics for performance tracking
//!
//! # Tiger Style Compliance
//!
//! - Fixed operation count: 1000 writes
//! - Fixed cluster size: 3 nodes
//! - Explicit measurement: total time, throughput
//! - Bounded test duration: expected < 60 seconds

use std::time::{Duration, Instant};

use aspen::api::{
    ClusterController, ClusterNode, InitRequest, KeyValueStore, WriteCommand, WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::node::NodeClient;
use aspen::raft::RaftControlClient;
use aspen::testing::create_test_aspen_node;

/// Test sustained write load with 1000 sequential operations.
///
/// Measures baseline write throughput for performance tracking.
#[tokio::test]
#[ignore] // Resource-intensive test, run explicitly with --ignored
async fn test_sustained_write_1000_ops() -> anyhow::Result<()> {
    const TOTAL_WRITES: u64 = 1000;

    // Start 3-node cluster
    let temp_dir1 = tempfile::tempdir()?;
    let temp_dir2 = tempfile::tempdir()?;
    let temp_dir3 = tempfile::tempdir()?;

    let mut handles = Vec::new();

    for (node_id, data_dir) in [
        (1, temp_dir1.path()),
        (2, temp_dir2.path()),
        (3, temp_dir3.path()),
    ] {
        let config = ClusterBootstrapConfig {
            node_id,
            control_backend: ControlBackend::RaftActor,
            host: "127.0.0.1".to_string(),
            http_addr: format!("127.0.0.1:{}", 36000 + node_id as u16).parse()?,
            ractor_port: 0, // OS-assigned
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "sustained-write-load".to_string(),
            heartbeat_interval_ms: 500,
            election_timeout_min_ms: 1500,
            election_timeout_max_ms: 3000,
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: aspen::raft::storage::StorageBackend::default(),
            redb_log_path: None,
            redb_sm_path: None,
            sqlite_log_path: None,
            sqlite_sm_path: None,
            supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
            raft_mailbox_capacity: 1000,
        };

        let handle = bootstrap_node(config).await?;
        handles.push(handle);
    }

    // Initialize cluster
    let cluster = RaftControlClient::new(handles[0].raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![
            ClusterNode::with_iroh_addr(1, create_test_aspen_node(1).iroh_addr),
            ClusterNode::with_iroh_addr(2, create_test_aspen_node(2).iroh_addr),
            ClusterNode::with_iroh_addr(3, create_test_aspen_node(3).iroh_addr),
        ],
    };
    cluster.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create KV client
    let kv = NodeClient::new(handles[0].raft_actor.clone());

    // Perform sustained writes
    let start = Instant::now();
    let mut successful_writes = 0u64;
    let mut failed_writes = 0u64;

    for i in 0..TOTAL_WRITES {
        let key = format!("load-test-key-{}", i);
        let value = format!("value-{}", i);

        let write_req = WriteRequest {
            command: WriteCommand::Set { key, value },
        };

        match kv.write(write_req).await {
            Ok(_) => successful_writes += 1,
            Err(e) => {
                eprintln!("Write {} failed: {}", i, e);
                failed_writes += 1;
            }
        }

        // Progress indicator every 100 writes
        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{} writes completed", i + 1, TOTAL_WRITES);
        }
    }

    let duration = start.elapsed();

    // Calculate metrics
    let duration_secs = duration.as_secs_f64();
    let throughput = successful_writes as f64 / duration_secs;

    // Log results
    println!("\n=== Sustained Write Load Test Results ===");
    println!("Total writes:       {}", TOTAL_WRITES);
    println!("Successful writes:  {}", successful_writes);
    println!("Failed writes:      {}", failed_writes);
    println!("Duration:           {:.2} seconds", duration_secs);
    println!("Throughput:         {:.2} ops/sec", throughput);
    println!(
        "Average latency:    {:.2} ms/op",
        duration.as_millis() as f64 / TOTAL_WRITES as f64
    );

    // Assertions
    assert_eq!(successful_writes, TOTAL_WRITES, "all writes should succeed");
    assert_eq!(failed_writes, 0, "no writes should fail");

    // Sanity check: expect at least 10 ops/sec (conservative baseline)
    assert!(
        throughput >= 10.0,
        "throughput should be >= 10 ops/sec, got {:.2}",
        throughput
    );

    // Cleanup
    for handle in handles {
        handle.shutdown().await?;
    }

    Ok(())
}

/// Smaller sustained write test for faster CI execution (100 writes).
#[tokio::test]
async fn test_sustained_write_100_ops() -> anyhow::Result<()> {
    const TOTAL_WRITES: u64 = 100;

    // Single-node cluster for faster test
    let temp_dir = tempfile::tempdir()?;

    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:46000".parse()?,
        ractor_port: 0, // OS-assigned
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "sustained-write-small".to_string(),
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config).await?;

    // Initialize single-node cluster
    let cluster = RaftControlClient::new(handle.raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(
            1,
            create_test_aspen_node(1).iroh_addr,
        )],
    };
    cluster.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Create KV client
    let kv = NodeClient::new(handle.raft_actor.clone());

    // Perform writes
    let start = Instant::now();
    let mut successful_writes = 0u64;

    for i in 0..TOTAL_WRITES {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("key-{}", i),
                value: format!("value-{}", i),
            },
        };

        kv.write(write_req).await?;
        successful_writes += 1;
    }

    let duration = start.elapsed();
    let throughput = successful_writes as f64 / duration.as_secs_f64();

    println!(
        "Sustained write (100 ops): {:.2} ops/sec, {:.2} ms avg latency",
        throughput,
        duration.as_millis() as f64 / TOTAL_WRITES as f64
    );

    assert_eq!(successful_writes, TOTAL_WRITES);

    // Cleanup
    handle.shutdown().await?;

    Ok(())
}

/// Batch write test to measure batching improvements (future optimization).
#[tokio::test]
#[ignore] // Future optimization - batching not yet implemented
async fn test_batch_write_optimization() -> anyhow::Result<()> {
    // Placeholder for future batch write optimization
    // Will compare sequential writes vs batched writes throughput
    //
    // Expected: Batching should provide 2-5x throughput improvement
    // by reducing network round-trips and log append operations.

    Ok(())
}
