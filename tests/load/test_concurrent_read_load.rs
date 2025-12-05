//! Test: Concurrent Read Load
//!
//! Validates that the cluster can handle concurrent read operations
//! and measures read throughput under load.
//!
//! # Test Strategy
//!
//! 1. Start 3-node cluster
//! 2. Pre-populate 100 keys
//! 3. Spawn 100 concurrent read tasks
//! 4. Measure total time and throughput
//! 5. Verify all reads succeed
//!
//! # Tiger Style Compliance
//!
//! - Fixed concurrent readers: 100 tasks
//! - Fixed data set: 100 keys
//! - Bounded test duration: expected < 30 seconds
//! - Explicit measurement: total time, throughput

use std::sync::Arc;
use std::time::{Duration, Instant};

use aspen::api::{
    ClusterController, ClusterNode, InitRequest, KeyValueStore, ReadRequest, WriteCommand,
    WriteRequest,
};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ClusterBootstrapConfig, ControlBackend, IrohConfig};
use aspen::kv::KvClient;
use aspen::raft::RaftControlClient;
use tokio::task::JoinSet;

/// Test concurrent read load with 100 parallel readers.
#[tokio::test]
#[ignore] // Resource-intensive test, run explicitly with --ignored
async fn test_concurrent_read_100_readers() -> anyhow::Result<()> {
    const NUM_KEYS: u64 = 100;
    const NUM_READERS: usize = 100;

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
            http_addr: format!("127.0.0.1:{}", 37000 + node_id as u16).parse()?,
            ractor_port: 0, // OS-assigned
            data_dir: Some(data_dir.to_path_buf()),
            cookie: "concurrent-read-load".to_string(),
            heartbeat_interval_ms: 500,
            election_timeout_min_ms: 1500,
            election_timeout_max_ms: 3000,
            iroh: IrohConfig::default(),
            peers: vec![],
            storage_backend: aspen::raft::storage::StorageBackend::default(),
            redb_log_path: None,
            redb_sm_path: None,
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
            ClusterNode::new(1, "127.0.0.1:26000", Some("iroh://placeholder1".into())),
            ClusterNode::new(2, "127.0.0.1:26001", Some("iroh://placeholder2".into())),
            ClusterNode::new(3, "127.0.0.1:26002", Some("iroh://placeholder3".into())),
        ],
    };
    cluster.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Pre-populate keys
    let kv = KvClient::with_timeout(handles[0].raft_actor.clone(), 5000);

    println!("Pre-populating {} keys...", NUM_KEYS);
    for i in 0..NUM_KEYS {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("read-test-key-{}", i),
                value: format!("value-{}", i),
            },
        };
        kv.write(write_req).await?;
    }

    println!("Pre-population complete");
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Spawn concurrent readers
    let kv_arc = Arc::new(kv);
    let mut join_set = JoinSet::new();

    println!("Spawning {} concurrent readers...", NUM_READERS);
    let start = Instant::now();

    for reader_id in 0..NUM_READERS {
        let kv_clone = Arc::clone(&kv_arc);
        join_set.spawn(async move {
            // Each reader reads all keys
            let mut successful_reads = 0u64;
            let mut failed_reads = 0u64;

            for i in 0..NUM_KEYS {
                let read_req = ReadRequest {
                    key: format!("read-test-key-{}", i),
                };

                match kv_clone.read(read_req).await {
                    Ok(response) => {
                        // Verify data
                        let expected_value = format!("value-{}", i);
                        if response.value == expected_value {
                            successful_reads += 1;
                        } else {
                            eprintln!(
                                "Reader {} key {}: unexpected value {:?}",
                                reader_id, i, response.value
                            );
                            failed_reads += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("Reader {} key {} failed: {}", reader_id, i, e);
                        failed_reads += 1;
                    }
                }
            }

            (successful_reads, failed_reads)
        });
    }

    // Wait for all readers to complete
    let mut total_successful = 0u64;
    let mut total_failed = 0u64;

    while let Some(result) = join_set.join_next().await {
        let (successful, failed) = result?;
        total_successful += successful;
        total_failed += failed;
    }

    let duration = start.elapsed();

    // Calculate metrics
    let duration_secs = duration.as_secs_f64();
    let total_reads = total_successful + total_failed;
    let throughput = total_successful as f64 / duration_secs;

    // Log results
    println!("\n=== Concurrent Read Load Test Results ===");
    println!("Concurrent readers: {}", NUM_READERS);
    println!("Reads per reader:   {}", NUM_KEYS);
    println!("Total reads:        {}", total_reads);
    println!("Successful reads:   {}", total_successful);
    println!("Failed reads:       {}", total_failed);
    println!("Duration:           {:.2} seconds", duration_secs);
    println!("Throughput:         {:.2} reads/sec", throughput);
    println!(
        "Average latency:    {:.2} ms/read",
        duration.as_millis() as f64 / total_reads as f64
    );

    // Assertions
    let expected_total = NUM_READERS as u64 * NUM_KEYS;
    assert_eq!(total_successful, expected_total, "all reads should succeed");
    assert_eq!(total_failed, 0, "no reads should fail");

    // Sanity check: expect at least 100 reads/sec with concurrency
    assert!(
        throughput >= 100.0,
        "throughput should be >= 100 reads/sec, got {:.2}",
        throughput
    );

    // Cleanup
    for handle in handles {
        handle.shutdown().await?;
    }

    Ok(())
}

/// Smaller concurrent read test for faster CI execution (10 readers, 10 keys).
#[tokio::test]
async fn test_concurrent_read_10_readers() -> anyhow::Result<()> {
    const NUM_KEYS: u64 = 10;
    const NUM_READERS: usize = 10;

    // Single-node cluster for faster test
    let temp_dir = tempfile::tempdir()?;

    let config = ClusterBootstrapConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:47000".parse()?,
        ractor_port: 0, // OS-assigned
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "concurrent-read-small".to_string(),
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config).await?;

    // Initialize single-node cluster
    let cluster = RaftControlClient::new(handle.raft_actor.clone());
    let init_req = InitRequest {
        initial_members: vec![ClusterNode::new(
            1,
            "127.0.0.1:26000",
            Some("iroh://placeholder".into()),
        )],
    };
    cluster.init(init_req).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Pre-populate keys
    let kv = KvClient::with_timeout(handle.raft_actor.clone(), 5000);
    for i in 0..NUM_KEYS {
        let write_req = WriteRequest {
            command: WriteCommand::Set {
                key: format!("key-{}", i),
                value: format!("value-{}", i),
            },
        };
        kv.write(write_req).await?;
    }

    // Concurrent reads
    let kv_arc = Arc::new(kv);
    let mut join_set = JoinSet::new();
    let start = Instant::now();

    for _ in 0..NUM_READERS {
        let kv_clone = Arc::clone(&kv_arc);
        join_set.spawn(async move {
            let mut count = 0u64;
            for i in 0..NUM_KEYS {
                let read_req = ReadRequest {
                    key: format!("key-{}", i),
                };
                if kv_clone.read(read_req).await.is_ok() {
                    count += 1;
                }
            }
            count
        });
    }

    let mut total_successful = 0u64;
    while let Some(result) = join_set.join_next().await {
        total_successful += result?;
    }

    let duration = start.elapsed();
    let throughput = total_successful as f64 / duration.as_secs_f64();

    println!(
        "Concurrent read ({} readers × {} keys): {:.2} reads/sec",
        NUM_READERS, NUM_KEYS, throughput
    );

    assert_eq!(total_successful, NUM_READERS as u64 * NUM_KEYS);

    // Cleanup
    handle.shutdown().await?;

    Ok(())
}
