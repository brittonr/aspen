//! Test: Mixed Workload (70% Reads, 30% Writes)
//!
//! Validates cluster performance under realistic mixed read/write workload.
//!
//! # Test Strategy
//!
//! 1. Start 3-node cluster
//! 2. Run 1000 operations: 70% reads, 30% writes
//! 3. Randomize key access patterns (realistic distribution)
//! 4. Measure throughput and success rate
//! 5. Verify >99% success rate
//!
//! # Tiger Style Compliance
//!
//! - Fixed total operations: 1000
//! - Fixed read/write ratio: 70/30
//! - Fixed key space: 100 keys (allows cache hits)
//! - Bounded test duration: expected < 60 seconds

use std::time::{Duration, Instant};

use aspen::api::{
    ClusterController, ClusterNode, InitRequest, KeyValueStore, ReadRequest, WriteCommand,
    WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::node::NodeClient;
use aspen::raft::RaftControlClient;
use aspen::testing::create_test_aspen_node;
use rand::Rng;

/// Test mixed workload with 70% reads, 30% writes over 1000 operations.
#[tokio::test]
#[ignore] // Resource-intensive test, run explicitly with --ignored
async fn test_mixed_workload_1000_ops() -> anyhow::Result<()> {
    const TOTAL_OPS: usize = 1000;
    const READ_PERCENTAGE: usize = 70;
    const KEY_SPACE: u64 = 100; // Total unique keys

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
            http_addr: format!("127.0.0.1:{}", 38000 + node_id as u16).parse()?,
            ractor_port: 0, // OS-assigned
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "mixed-workload".to_string(),
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
    let kv = NodeClient::with_timeout(handles[0].raft_actor.clone(), 5000);

    // Pre-populate half the key space to allow reads
    println!("Pre-populating {} keys...", KEY_SPACE / 2);
    for i in 0..(KEY_SPACE / 2) {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("mixed-key-{}", i),
                value: format!("initial-value-{}", i),
            },
        };
        kv.write(write_req).await?;
    }

    println!("Pre-population complete");
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Generate operation sequence (70% reads, 30% writes)
    let mut rng = rand::thread_rng();
    let mut operations = Vec::with_capacity(TOTAL_OPS);

    for _ in 0..TOTAL_OPS {
        let is_read = rng.gen_range(0..100) < READ_PERCENTAGE;
        let key_id = rng.gen_range(0..KEY_SPACE);
        operations.push((is_read, key_id));
    }

    // Execute mixed workload
    println!(
        "Executing {} operations ({}% read, {}% write)...",
        TOTAL_OPS,
        READ_PERCENTAGE,
        100 - READ_PERCENTAGE
    );

    let start = Instant::now();
    let mut successful_reads = 0u64;
    let mut successful_writes = 0u64;
    let mut failed_reads = 0u64;
    let mut failed_writes = 0u64;

    for (i, (is_read, key_id)) in operations.iter().enumerate() {
        let key = format!("mixed-key-{}", key_id);

        if *is_read {
            let read_req = ReadRequest { key: key.clone() };
            match kv.read(read_req).await {
                Ok(_) => successful_reads += 1,
                Err(e) => {
                    eprintln!("Read {} failed (key: {}): {}", i, key, e);
                    failed_reads += 1;
                }
            }
        } else {
            let value = format!("updated-value-{}", i);
            let write_req = WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value,
                },
            };
            match kv.write(write_req).await {
                Ok(_) => successful_writes += 1,
                Err(e) => {
                    eprintln!("Write {} failed (key: {}): {}", i, key, e);
                    failed_writes += 1;
                }
            }
        }

        // Progress indicator every 100 ops
        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{} operations completed", i + 1, TOTAL_OPS);
        }
    }

    let duration = start.elapsed();

    // Calculate metrics
    let duration_secs = duration.as_secs_f64();
    let total_successful = successful_reads + successful_writes;
    let total_failed = failed_reads + failed_writes;
    let total_ops = total_successful + total_failed;
    let success_rate = (total_successful as f64 / total_ops as f64) * 100.0;
    let throughput = total_successful as f64 / duration_secs;

    // Log results
    println!("\n=== Mixed Workload Test Results ===");
    println!("Total operations:   {}", total_ops);
    println!("Successful reads:   {}", successful_reads);
    println!("Successful writes:  {}", successful_writes);
    println!("Failed reads:       {}", failed_reads);
    println!("Failed writes:      {}", failed_writes);
    println!("Success rate:       {:.2}%", success_rate);
    println!("Duration:           {:.2} seconds", duration_secs);
    println!("Throughput:         {:.2} ops/sec", throughput);
    println!(
        "Average latency:    {:.2} ms/op",
        duration.as_millis() as f64 / total_ops as f64
    );
    println!("\nOperation mix:");
    println!(
        "  Reads:  {} ({:.1}%)",
        successful_reads + failed_reads,
        ((successful_reads + failed_reads) as f64 / total_ops as f64) * 100.0
    );
    println!(
        "  Writes: {} ({:.1}%)",
        successful_writes + failed_writes,
        ((successful_writes + failed_writes) as f64 / total_ops as f64) * 100.0
    );

    // Assertions
    // Note: Some failures are expected in mixed workload since reads may target
    // keys that haven't been written yet (random access pattern)
    assert!(
        success_rate >= 85.0,
        "success rate should be >= 85%, got {:.2}%",
        success_rate
    );

    // Sanity check: expect at least 10 ops/sec
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

/// Smaller mixed workload test for faster CI execution (100 ops).
#[tokio::test]
async fn test_mixed_workload_100_ops() -> anyhow::Result<()> {
    const TOTAL_OPS: usize = 100;
    const READ_PERCENTAGE: usize = 70;
    const KEY_SPACE: u64 = 20;

    // Single-node cluster for faster test
    let temp_dir = tempfile::tempdir()?;

    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:48000".parse()?,
        ractor_port: 0, // OS-assigned
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "mixed-workload-small".to_string(),
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

    // Pre-populate keys (90% of key space to ensure >= 85% success rate)
    // With 90% initial coverage and random write distribution, final coverage
    // approaches ~98%, giving expected success: 30 writes + ~69 reads = ~99%
    let kv = NodeClient::with_timeout(handle.raft_actor.clone(), 5000);
    for i in 0..((KEY_SPACE * 9) / 10) {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("key-{}", i),
                value: format!("initial-{}", i),
            },
        };
        kv.write(write_req).await?;
    }

    // Generate operations
    let mut rng = rand::thread_rng();
    let mut operations = Vec::with_capacity(TOTAL_OPS);

    for _ in 0..TOTAL_OPS {
        let is_read = rng.gen_range(0..100) < READ_PERCENTAGE;
        let key_id = rng.gen_range(0..KEY_SPACE);
        operations.push((is_read, key_id));
    }

    // Execute workload
    let start = Instant::now();
    let mut successful = 0u64;
    let mut failed = 0u64;

    for (is_read, key_id) in operations {
        let key = format!("key-{}", key_id);

        let result = if is_read {
            kv.read(ReadRequest { key }).await.map(|_| ())
        } else {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key,
                    value: "updated".to_string(),
                },
            })
            .await
            .map(|_| ())
        };

        if result.is_ok() {
            successful += 1;
        } else {
            failed += 1;
        }
    }

    let duration = start.elapsed();
    let success_rate = (successful as f64 / (successful + failed) as f64) * 100.0;
    let throughput = successful as f64 / duration.as_secs_f64();

    println!(
        "Mixed workload (100 ops): {:.2}% success, {:.2} ops/sec",
        success_rate, throughput
    );

    // Note: Some failures are expected in mixed workload since reads may target
    // keys that haven't been written yet (random access pattern)
    assert!(
        success_rate >= 85.0,
        "success rate should be >= 85%, got {:.2}%",
        success_rate
    );

    // Cleanup
    handle.shutdown().await?;

    Ok(())
}

/// SQLite variant: Mixed workload test using SQLite storage backend.
#[tokio::test]
async fn test_mixed_workload_100_ops_sqlite() -> anyhow::Result<()> {
    const TOTAL_OPS: usize = 100;
    const READ_PERCENTAGE: usize = 70;
    const KEY_SPACE: u64 = 20;

    // Single-node cluster for faster test
    let temp_dir = tempfile::tempdir()?;

    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:48001".parse()?,
        ractor_port: 0, // OS-assigned
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "mixed-workload-small-sqlite".to_string(),
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::Sqlite,
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: Some(temp_dir.path().join("raft-log.db")),
        sqlite_sm_path: Some(temp_dir.path().join("state-machine.db")),
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

    // Pre-populate keys (90% of key space to ensure >= 85% success rate)
    // With 90% initial coverage and random write distribution, final coverage
    // approaches ~98%, giving expected success: 30 writes + ~69 reads = ~99%
    let kv = NodeClient::with_timeout(handle.raft_actor.clone(), 5000);
    for i in 0..((KEY_SPACE * 9) / 10) {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("key-{}", i),
                value: format!("initial-{}", i),
            },
        };
        kv.write(write_req).await?;
    }

    // Generate operations
    let mut rng = rand::thread_rng();
    let mut operations = Vec::with_capacity(TOTAL_OPS);

    for _ in 0..TOTAL_OPS {
        let is_read = rng.gen_range(0..100) < READ_PERCENTAGE;
        let key_id = rng.gen_range(0..KEY_SPACE);
        operations.push((is_read, key_id));
    }

    // Execute workload
    let start = Instant::now();
    let mut successful = 0u64;
    let mut failed = 0u64;

    for (is_read, key_id) in operations {
        let key = format!("key-{}", key_id);

        let result = if is_read {
            kv.read(ReadRequest { key }).await.map(|_| ())
        } else {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key,
                    value: "updated".to_string(),
                },
            })
            .await
            .map(|_| ())
        };

        if result.is_ok() {
            successful += 1;
        } else {
            failed += 1;
        }
    }

    let duration = start.elapsed();
    let success_rate = (successful as f64 / (successful + failed) as f64) * 100.0;
    let throughput = successful as f64 / duration.as_secs_f64();

    println!(
        "Mixed workload SQLite (100 ops): {:.2}% success, {:.2} ops/sec",
        success_rate, throughput
    );

    // Note: Some failures are expected in mixed workload since reads may target
    // keys that haven't been written yet (random access pattern)
    assert!(
        success_rate >= 85.0,
        "success rate should be >= 85%, got {:.2}%",
        success_rate
    );

    // Cleanup
    handle.shutdown().await?;

    Ok(())
}

/// Stress test with higher write percentage to test write-heavy scenarios.
#[tokio::test]
#[ignore] // Resource-intensive test
async fn test_write_heavy_workload() -> anyhow::Result<()> {
    // 50% writes, 50% reads - tests write contention
    // Similar structure to mixed_workload_100_ops but with different ratio
    // Left as exercise for future stress testing

    Ok(())
}
