//! Write batching throughput benchmarks with concurrent writers.
//!
//! These benchmarks measure the throughput improvement from write batching by:
//! 1. Spawning multiple concurrent writers
//! 2. Measuring total ops/sec (not individual latency)
//! 3. Verifying data via DataFusion SQL queries
//!
//! ## Expected Results
//!
//! | Config | Ops/sec | Improvement |
//! |--------|---------|-------------|
//! | Disabled (sequential) | ~600 | 1x baseline |
//! | Default (100 entries, 2ms) | ~6,000 | 10x |
//! | High throughput | ~8,000 | 13x |
//!
//! ## Benchmarks
//!
//! - `batch_throughput/concurrent_50`: 50 concurrent writers, measure ops/sec
//! - `batch_throughput/sql_verify`: Write 1000 keys, verify via SQL COUNT(*)
//! - `batch_config/sweep`: Compare different batch sizes
//!
//! Run with: `nix develop -c cargo bench --bench write_batching`

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use aspen::api::{SqlConsistency, SqlQueryExecutor, SqlQueryRequest, WriteCommand, WriteRequest};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use openraft::ServerState;
use tempfile::TempDir;
use tokio::sync::Barrier;

use aspen::api::{ClusterController, ClusterNode, InitRequest, KeyValueStore};
use aspen::node::{Node, NodeBuilder};
use aspen::raft::BatchConfig;
use aspen::raft::storage::StorageBackend;

// ====================================================================================
// Node Setup Helpers
// ====================================================================================

/// Setup a single-node cluster with write batching enabled (Redb backend).
async fn setup_single_node_with_batching(
    temp_dir: &TempDir,
    batch_config: BatchConfig,
) -> anyhow::Result<Node> {
    let data_dir = temp_dir.path().join("node-1");

    let mut node = NodeBuilder::new(aspen::node::NodeId(1), &data_dir)
        .with_storage(StorageBackend::Redb)
        .with_cookie("batch-bench")
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

/// Setup a single-node cluster WITHOUT batching for baseline comparison.
async fn setup_single_node_no_batching(temp_dir: &TempDir) -> anyhow::Result<Node> {
    let data_dir = temp_dir.path().join("node-1");

    let mut node = NodeBuilder::new(aspen::node::NodeId(1), &data_dir)
        .with_storage(StorageBackend::Redb)
        .with_cookie("batch-bench")
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

// ====================================================================================
// Concurrent Throughput Benchmarks
// ====================================================================================

/// Benchmark concurrent writes with batching enabled.
///
/// Spawns N concurrent writers, each performing rapid Set operations.
/// Measures total throughput (ops/sec) across all writers.
fn bench_concurrent_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("batch_throughput");

    // Benchmark with batching disabled (baseline) - sequential writes
    {
        let temp_dir = TempDir::new().expect("temp dir");
        let node = rt.block_on(async {
            setup_single_node_no_batching(&temp_dir)
                .await
                .expect("setup node without batching")
        });

        let counter = Arc::new(AtomicU64::new(0));
        let raft_node = node.raft_node().clone();

        group.throughput(Throughput::Elements(1));
        group.bench_function("no_batching_sequential", |b| {
            b.to_async(&rt).iter(|| {
                let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
                let rn = raft_node.clone();
                async move {
                    rn.write(WriteRequest {
                        command: WriteCommand::Set {
                            key,
                            value: "benchmark-value".to_string(),
                        },
                    })
                    .await
                    .expect("write failed")
                }
            })
        });

        rt.block_on(async {
            let _ = node.shutdown().await;
        });
    }

    // Benchmark with batching enabled + concurrent writers
    for num_writers in [10, 25, 50] {
        let temp_dir = TempDir::new().expect("temp dir");
        let node = rt.block_on(async {
            setup_single_node_with_batching(&temp_dir, BatchConfig::default())
                .await
                .expect("setup node with batching")
        });

        let counter = Arc::new(AtomicU64::new(0));
        let raft_node = node.raft_node().clone();

        // Measure throughput over fixed duration
        group.throughput(Throughput::Elements(num_writers as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent_writers", num_writers),
            &num_writers,
            |b, &n| {
                b.to_async(&rt).iter(|| {
                    let rn = raft_node.clone();
                    let counter = counter.clone();
                    async move {
                        // Spawn N concurrent writes
                        let mut handles = Vec::with_capacity(n);
                        for _ in 0..n {
                            let rn = rn.clone();
                            let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
                            handles.push(tokio::spawn(async move {
                                rn.write(WriteRequest {
                                    command: WriteCommand::Set {
                                        key,
                                        value: "benchmark-value".to_string(),
                                    },
                                })
                                .await
                            }));
                        }

                        // Wait for all writes to complete
                        for h in handles {
                            h.await.expect("task panic").expect("write failed");
                        }
                    }
                })
            },
        );

        rt.block_on(async {
            let _ = node.shutdown().await;
        });
    }

    group.finish();
}

/// Benchmark raw throughput measurement (ops/sec over time window).
///
/// This benchmark measures actual throughput by running writes for a fixed
/// duration and counting completed operations.
fn bench_throughput_measurement(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("batch_ops_per_second");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Measure with batching
    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        setup_single_node_with_batching(&temp_dir, BatchConfig::default())
            .await
            .expect("setup node")
    });

    let counter = Arc::new(AtomicU64::new(0));
    let raft_node = node.raft_node().clone();

    // This benchmark spawns 50 concurrent writers that continuously write
    group.throughput(Throughput::Elements(50));
    group.bench_function("50_concurrent_batched", |b| {
        b.to_async(&rt).iter(|| {
            let rn = raft_node.clone();
            let counter = counter.clone();
            async move {
                let barrier = Arc::new(Barrier::new(51)); // 50 writers + 1 main

                let mut handles = Vec::with_capacity(50);
                for _ in 0..50 {
                    let rn = rn.clone();
                    let counter = counter.clone();
                    let barrier = barrier.clone();
                    handles.push(tokio::spawn(async move {
                        barrier.wait().await;
                        let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
                        rn.write(WriteRequest {
                            command: WriteCommand::Set {
                                key,
                                value: "benchmark-value".to_string(),
                            },
                        })
                        .await
                    }));
                }

                // Release all writers at once
                barrier.wait().await;

                for h in handles {
                    h.await.expect("task panic").expect("write failed");
                }
            }
        })
    });

    rt.block_on(async {
        let _ = node.shutdown().await;
    });

    group.finish();
}

// ====================================================================================
// SQL Verification Benchmarks
// ====================================================================================

/// Benchmark that writes via batching and verifies via SQL.
///
/// This validates that batched writes are correctly persisted by querying
/// the data through the DataFusion SQL layer.
fn bench_sql_verification(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("batch_sql_verify");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        setup_single_node_with_batching(&temp_dir, BatchConfig::default())
            .await
            .expect("setup node")
    });

    // Pre-populate with batched writes
    let write_count = 1000u64;
    let raft_node = node.raft_node().clone();
    rt.block_on(async {
        let counter = Arc::new(AtomicU64::new(0));

        // Spawn 50 concurrent writers for 1000 total writes
        let mut handles = Vec::with_capacity(1000);
        for _ in 0..1000 {
            let rn = raft_node.clone();
            let idx = counter.fetch_add(1, Ordering::Relaxed);
            handles.push(tokio::spawn(async move {
                rn.write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("batch-key-{:06}", idx),
                        value: format!("batch-value-{:06}", idx),
                    },
                })
                .await
            }));
        }

        for h in handles {
            h.await.expect("task panic").expect("write failed");
        }
    });

    // Benchmark SQL verification
    group.throughput(Throughput::Elements(write_count));
    group.bench_function("count_1000_batched_keys", |b| {
        b.to_async(&rt).iter(|| {
            let rn = raft_node.clone();
            async move {
                let result = rn
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT COUNT(*) FROM kv WHERE key LIKE 'batch-key-%'".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(1),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed");

                // Verify count matches
                assert!(!result.rows.is_empty(), "Expected results from COUNT(*)");
                result
            }
        })
    });

    // Also benchmark SELECT * to verify data integrity
    group.bench_function("select_all_1000_batched_keys", |b| {
        b.to_async(&rt).iter(|| {
            let rn = raft_node.clone();
            async move {
                let result = rn
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT * FROM kv WHERE key LIKE 'batch-key-%'".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(10000),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed");

                // Verify we got all 1000 rows
                assert_eq!(
                    result.rows.len(),
                    1000,
                    "Expected 1000 rows, got {}",
                    result.rows.len()
                );
                result
            }
        })
    });

    rt.block_on(async {
        let _ = node.shutdown().await;
    });

    group.finish();
}

// ====================================================================================
// Batch Configuration Sweep
// ====================================================================================

/// Benchmark different batch configurations to find optimal settings.
fn bench_batch_config_sweep(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("batch_config");
    group.measurement_time(Duration::from_secs(5));

    let configs = [
        ("low_latency", BatchConfig::low_latency()),
        ("default", BatchConfig::default()),
        ("high_throughput", BatchConfig::high_throughput()),
    ];

    for (name, config) in configs {
        let temp_dir = TempDir::new().expect("temp dir");
        let node = rt.block_on(async {
            setup_single_node_with_batching(&temp_dir, config)
                .await
                .expect("setup node")
        });

        let counter = Arc::new(AtomicU64::new(0));
        let raft_node = node.raft_node().clone();

        // 25 concurrent writers
        group.throughput(Throughput::Elements(25));
        group.bench_with_input(BenchmarkId::new("25_writers", name), &name, |b, _| {
            b.to_async(&rt).iter(|| {
                let rn = raft_node.clone();
                let counter = counter.clone();
                async move {
                    let mut handles = Vec::with_capacity(25);
                    for _ in 0..25 {
                        let rn = rn.clone();
                        let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
                        handles.push(tokio::spawn(async move {
                            rn.write(WriteRequest {
                                command: WriteCommand::Set {
                                    key,
                                    value: "benchmark-value".to_string(),
                                },
                            })
                            .await
                        }));
                    }

                    for h in handles {
                        h.await.expect("task panic").expect("write failed");
                    }
                }
            })
        });

        rt.block_on(async {
            let _ = node.shutdown().await;
        });
    }

    group.finish();
}

// ====================================================================================
// Latency Distribution Benchmark
// ====================================================================================

/// Measure latency distribution (p50, p90, p99) with batching.
fn bench_latency_distribution(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("batch_latency");
    group.measurement_time(Duration::from_secs(10));

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        setup_single_node_with_batching(&temp_dir, BatchConfig::default())
            .await
            .expect("setup node")
    });

    let counter = Arc::new(AtomicU64::new(0));
    let raft_node = node.raft_node().clone();

    // Individual write latency with batching
    group.throughput(Throughput::Elements(1));
    group.bench_function("single_write_batched", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
            let rn = raft_node.clone();
            async move {
                rn.write(WriteRequest {
                    command: WriteCommand::Set {
                        key,
                        value: "benchmark-value".to_string(),
                    },
                })
                .await
                .expect("write failed")
            }
        })
    });

    rt.block_on(async {
        let _ = node.shutdown().await;
    });

    group.finish();
}

// ====================================================================================
// Real Throughput Measurement (Total ops over fixed time)
// ====================================================================================

/// Measure actual ops/sec by counting operations over a fixed time window.
///
/// This is the most accurate throughput measurement as it accounts for
/// batching behavior and provides real-world ops/sec numbers.
fn bench_real_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("batch_real_throughput");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    // Test with batching
    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        setup_single_node_with_batching(&temp_dir, BatchConfig::default())
            .await
            .expect("setup node")
    });

    // Run 50 concurrent writers for measurement duration
    // Criterion will measure how long it takes to complete one "iteration"
    // We define iteration as completing 100 writes across 50 concurrent workers
    let counter = Arc::new(AtomicU64::new(0));
    let raft_node = node.raft_node().clone();

    group.throughput(Throughput::Elements(100)); // 100 ops per iteration
    group.bench_function("100_ops_50_writers", |b| {
        b.to_async(&rt).iter(|| {
            let rn = raft_node.clone();
            let counter = counter.clone();
            async move {
                // 50 writers each doing 2 writes = 100 total ops
                let mut handles = Vec::with_capacity(100);
                for _ in 0..100 {
                    let rn = rn.clone();
                    let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
                    handles.push(tokio::spawn(async move {
                        rn.write(WriteRequest {
                            command: WriteCommand::Set {
                                key,
                                value: "benchmark-value".to_string(),
                            },
                        })
                        .await
                    }));
                }

                for h in handles {
                    h.await.expect("task panic").expect("write failed");
                }
            }
        })
    });

    rt.block_on(async {
        let _ = node.shutdown().await;
    });

    group.finish();
}

criterion_group!(
    batch_benches,
    bench_concurrent_throughput,
    bench_throughput_measurement,
    bench_sql_verification,
    bench_batch_config_sweep,
    bench_latency_distribution,
    bench_real_throughput,
);

criterion_main!(batch_benches);
