//! Production-realistic benchmarks using real storage and networking.
//!
//! Unlike kv_operations.rs which uses InMemoryStateMachine for protocol testing,
//! these benchmarks use NodeBuilder with SQLite/Redb storage and real Iroh networking
//! to measure production-like latencies.
//!
//! ## Benchmark Groups
//!
//! **SQLite (dual-fsync)**:
//! - `prod_write`: Single-node write through Raft with SQLite + fsync
//! - `prod_read`: Single-node read with ReadIndex linearizability
//! - `prod_write_3node`: 3-node write with real Iroh QUIC quorum
//! - `prod_read_3node`: 3-node read with real network
//!
//! **Redb (single-fsync)**:
//! - `redb_write`: Single-node write with SharedRedbStorage
//! - `redb_read`: Single-node read from Redb
//! - `redb_write_3node`: 3-node write with single-fsync architecture
//! - `redb_read_3node`: 3-node read from Redb cluster
//!
//! ## Actual Results (2025-12-23)
//!
//! | Benchmark | SQLite | Redb | Improvement |
//! |-----------|--------|------|-------------|
//! | Single write | 9.9 ms | 1.65 ms | 6x |
//! | Single read | 9.0 µs | 5.0 µs | 1.8x |
//! | 3-node write | 18.5 ms | 3.2 ms | 5.8x |
//! | 3-node read | 39.5 µs | 31.0 µs | 1.3x |
//!
//! ## Comparison with Synthetic Benchmarks
//!
//! The kv_operations.rs benchmarks show 170ns reads and 34us writes because
//! they use in-memory storage (BTreeMap) with simulated networking. Those
//! benchmarks test the Raft protocol correctness, not production performance.

mod common;

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen::api::ReadRequest;
#[cfg(feature = "sql")]
use aspen::api::SqlConsistency;
#[cfg(feature = "sql")]
use aspen::api::SqlQueryExecutor;
#[cfg(feature = "sql")]
use aspen::api::SqlQueryRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
#[cfg(feature = "sql")]
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use tempfile::TempDir;

/// Benchmark single-node write with production storage (SQLite + fsync).
fn bench_prod_write_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async { common::setup_production_single_node(&temp_dir).await.expect("setup single node") });

    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("prod_write");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
            let kv = node.kv_store();
            async move {
                kv.write(WriteRequest {
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

    group.finish();

    // Cleanup
    rt.block_on(async {
        node.shutdown().await.expect("shutdown failed");
    });
}

/// Benchmark single-node read with production storage (SQLite + ReadIndex).
fn bench_prod_read_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        let node = common::setup_production_single_node(&temp_dir).await.expect("setup single node");

        // Pre-populate with 1000 keys
        let kv = node.kv_store();
        for i in 0..1000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("read-key-{:06}", i),
                    value: format!("read-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        node
    });

    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("prod_read");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            // Cycle through pre-populated keys
            let key_idx = counter.fetch_add(1, Ordering::Relaxed) % 1000;
            let key = format!("read-key-{:06}", key_idx);
            let kv = node.kv_store();
            async move { kv.read(ReadRequest::new(key)).await.expect("read failed") }
        })
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        node.shutdown().await.expect("shutdown failed");
    });
}

/// Benchmark 3-node write with real Iroh networking (quorum replication).
fn bench_prod_write_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes =
        rt.block_on(async { common::setup_production_three_node(&temp_dir).await.expect("setup 3-node cluster") });

    // Use node[0] as the leader for writes
    let leader = &nodes[0];
    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("prod_write_3node");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
            let kv = leader.kv_store();
            async move {
                kv.write(WriteRequest {
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

    group.finish();

    // Cleanup all nodes
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark 3-node read with real Iroh networking (ReadIndex over network).
fn bench_prod_read_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node(&temp_dir).await.expect("setup 3-node cluster");

        // Pre-populate with 1000 keys through leader
        let kv = nodes[0].kv_store();
        for i in 0..1000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("read-key-{:06}", i),
                    value: format!("read-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        // Wait for replication using OpenRaft Wait API
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(10)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    // Read from leader (includes ReadIndex overhead)
    let leader = &nodes[0];
    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("prod_read_3node");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            // Cycle through pre-populated keys
            let key_idx = counter.fetch_add(1, Ordering::Relaxed) % 1000;
            let key = format!("read-key-{:06}", key_idx);
            let kv = leader.kv_store();
            async move { kv.read(ReadRequest::new(key)).await.expect("read failed") }
        })
    });

    group.finish();

    // Cleanup all nodes
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark single-node write with Redb single-fsync storage.
///
/// This uses SharedRedbStorage which bundles log and state machine
/// operations into a single transaction with one fsync, expected to
/// reduce latency from ~9ms (SQLite) to ~2-3ms.
fn bench_redb_write_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        common::setup_production_single_node_redb(&temp_dir).await.expect("setup single node with redb")
    });

    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("redb_write");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
            let kv = node.kv_store();
            async move {
                kv.write(WriteRequest {
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

    group.finish();

    // Cleanup
    rt.block_on(async {
        node.shutdown().await.expect("shutdown failed");
    });
}

/// Benchmark single-node read with Redb single-fsync storage.
fn bench_redb_read_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        let node = common::setup_production_single_node_redb(&temp_dir).await.expect("setup single node with redb");

        // Pre-populate with 1000 keys
        let kv = node.kv_store();
        for i in 0..1000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("read-key-{:06}", i),
                    value: format!("read-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }
        node
    });

    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("redb_read");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key_idx = counter.fetch_add(1, Ordering::Relaxed) % 1000;
            let key = format!("read-key-{:06}", key_idx);
            let kv = node.kv_store();
            async move { kv.read(ReadRequest::new(key)).await.expect("read failed") }
        })
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        node.shutdown().await.expect("shutdown failed");
    });
}

/// Benchmark 3-node write with Redb single-fsync storage.
///
/// This measures write latency with quorum replication over real Iroh QUIC
/// connections, using SharedRedbStorage's single-fsync architecture.
/// Expected to significantly outperform SQLite-based cluster (~5-8ms vs ~19ms).
fn bench_redb_write_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster")
    });

    let leader = &nodes[0];
    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("redb_write_3node");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let idx = counter.fetch_add(1, Ordering::Relaxed);
            let key = format!("bench-key-{}", idx);
            let kv = leader.kv_store();
            async move {
                kv.write(WriteRequest {
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

    group.finish();

    // Cleanup all nodes
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark 3-node read with Redb single-fsync storage.
///
/// This measures read latency from a 3-node cluster with pre-populated data.
/// Reads go through the leader with ReadIndex linearizability.
fn bench_redb_read_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

        // Pre-populate with 1000 keys through leader
        let kv = nodes[0].kv_store();
        for i in 0..1000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("read-key-{:06}", i),
                    value: format!("read-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        // Wait for replication using OpenRaft Wait API
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(10)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    // Read from leader (includes ReadIndex overhead)
    let leader = &nodes[0];
    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("redb_read_3node");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            // Cycle through pre-populated keys
            let key_idx = counter.fetch_add(1, Ordering::Relaxed) % 1000;
            let key = format!("read-key-{:06}", key_idx);
            let kv = leader.kv_store();
            async move { kv.read(ReadRequest::new(key)).await.expect("read failed") }
        })
    });

    group.finish();

    // Cleanup all nodes
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

// ====================================================================================
// 3-Node SQL Benchmarks (DataFusion on Redb via Raft)
// ====================================================================================

/// Benchmark 3-node SQL SELECT * (full table scan).
///
/// This measures SQL query latency through the full Raft stack:
/// - Query goes through RaftNode.execute_sql()
/// - Uses ReadIndex for linearizable consistency
/// - DataFusion executes against Redb storage
#[cfg(feature = "sql")]
fn bench_sql_select_all_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

        // Pre-populate with 1000 keys through leader
        let kv = nodes[0].kv_store();
        for i in 0..1000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("bench-key-{:06}", i),
                    value: format!("bench-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        // Wait for replication
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(10)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    let leader = &nodes[0];
    let mut group = c.benchmark_group("sql_3node");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("select_all_1000", |b| {
        b.to_async(&rt).iter(|| {
            let raft_node = leader.raft_node();
            async move {
                raft_node
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT * FROM kv".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(10000),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed")
            }
        })
    });

    group.finish();

    // Cleanup all nodes
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark 3-node SQL point lookup (WHERE key = 'exact').
///
/// Tests filter pushdown performance for exact key matching.
#[cfg(feature = "sql")]
fn bench_sql_point_lookup_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

        // Pre-populate with 10000 keys
        let kv = nodes[0].kv_store();
        for i in 0..10000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("key-{:08}", i),
                    value: format!("value-{:08}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        // Wait for replication
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(30)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    let leader = &nodes[0];
    let counter = AtomicU64::new(0);
    let mut group = c.benchmark_group("sql_3node");
    group.throughput(Throughput::Elements(1));

    group.bench_function("point_lookup", |b| {
        b.to_async(&rt).iter(|| {
            let key_idx = counter.fetch_add(1, Ordering::Relaxed) % 10000;
            let key = format!("key-{:08}", key_idx);
            let query = format!("SELECT * FROM kv WHERE key = '{}'", key);
            let raft_node = leader.raft_node();
            async move {
                raft_node
                    .execute_sql(SqlQueryRequest {
                        query,
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(1),
                        timeout_ms: Some(5000),
                    })
                    .await
                    .expect("sql query failed")
            }
        })
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark 3-node SQL prefix scan (WHERE key LIKE 'prefix:%').
///
/// Tests filter pushdown for prefix matching.
#[cfg(feature = "sql")]
fn bench_sql_prefix_scan_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

        // Pre-populate: 10 prefixes x 100 keys = 1000 total
        let kv = nodes[0].kv_store();
        for prefix in 0..10 {
            for key_id in 0..100 {
                kv.write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("prefix_{:02}:key_{:04}", prefix, key_id),
                        value: format!("value_{}_{}", prefix, key_id),
                    },
                })
                .await
                .expect("populate failed");
            }
        }

        // Wait for replication
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(10)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    let leader = &nodes[0];
    let mut group = c.benchmark_group("sql_3node");
    group.throughput(Throughput::Elements(100)); // Each prefix has 100 keys

    group.bench_function("prefix_scan_100", |b| {
        b.to_async(&rt).iter(|| {
            let raft_node = leader.raft_node();
            async move {
                raft_node
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT * FROM kv WHERE key LIKE 'prefix_05:%'".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(1000),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed")
            }
        })
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark 3-node SQL COUNT(*) aggregation.
///
/// Tests aggregation performance on replicated data.
#[cfg(feature = "sql")]
fn bench_sql_count_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

        // Pre-populate with 5000 keys
        let kv = nodes[0].kv_store();
        for i in 0..5000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("count-key-{:06}", i),
                    value: format!("count-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        // Wait for replication
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(20)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    let leader = &nodes[0];
    let mut group = c.benchmark_group("sql_3node");
    group.throughput(Throughput::Elements(5000));

    group.bench_function("count_5000", |b| {
        b.to_async(&rt).iter(|| {
            let raft_node = leader.raft_node();
            async move {
                raft_node
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT COUNT(*) FROM kv".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(1),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed")
            }
        })
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark 3-node SQL with Stale consistency (local read, no ReadIndex).
///
/// Compares Linearizable vs Stale consistency overhead.
#[cfg(feature = "sql")]
fn bench_sql_stale_vs_linearizable_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

        // Pre-populate with 1000 keys
        let kv = nodes[0].kv_store();
        for i in 0..1000 {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("consistency-key-{:06}", i),
                    value: format!("consistency-value-{:06}", i),
                },
            })
            .await
            .expect("populate failed");
        }

        // Wait for replication
        let raft1 = nodes[0].raft_node();
        let metrics = raft1.raft().metrics().borrow().clone();
        let current_index = metrics.last_log_index.unwrap_or(0);
        raft1
            .raft()
            .wait(Some(std::time::Duration::from_secs(10)))
            .applied_index_at_least(Some(current_index), "pre-populated data replicated")
            .await
            .expect("replication wait");

        nodes
    });

    let leader = &nodes[0];
    let mut group = c.benchmark_group("sql_3node_consistency");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("linearizable", |b| {
        b.to_async(&rt).iter(|| {
            let raft_node = leader.raft_node();
            async move {
                raft_node
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT * FROM kv".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Linearizable,
                        limit: Some(10000),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed")
            }
        })
    });

    group.bench_function("stale", |b| {
        b.to_async(&rt).iter(|| {
            let raft_node = leader.raft_node();
            async move {
                raft_node
                    .execute_sql(SqlQueryRequest {
                        query: "SELECT * FROM kv".to_string(),
                        params: vec![],
                        consistency: SqlConsistency::Stale,
                        limit: Some(10000),
                        timeout_ms: Some(30000),
                    })
                    .await
                    .expect("sql query failed")
            }
        })
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        for node in nodes {
            let _ = node.shutdown().await;
        }
    });
}

/// Benchmark SQL query with varying data sizes.
#[cfg(feature = "sql")]
fn bench_sql_data_sizes_3node(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let mut group = c.benchmark_group("sql_3node_sizes");

    for size in [100, 500, 1000, 2000] {
        let temp_dir = TempDir::new().expect("temp dir");
        let nodes = rt.block_on(async {
            let nodes = common::setup_production_three_node_redb(&temp_dir).await.expect("setup 3-node redb cluster");

            // Pre-populate with `size` keys
            let kv = nodes[0].kv_store();
            for i in 0..size {
                kv.write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("size-key-{:06}", i),
                        value: format!("size-value-{:06}", i),
                    },
                })
                .await
                .expect("populate failed");
            }

            // Wait for replication
            let raft1 = nodes[0].raft_node();
            let metrics = raft1.raft().metrics().borrow().clone();
            let current_index = metrics.last_log_index.unwrap_or(0);
            raft1
                .raft()
                .wait(Some(std::time::Duration::from_secs(15)))
                .applied_index_at_least(Some(current_index), "pre-populated data replicated")
                .await
                .expect("replication wait");

            nodes
        });

        let leader = &nodes[0];
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let raft_node = leader.raft_node();
                async move {
                    raft_node
                        .execute_sql(SqlQueryRequest {
                            query: "SELECT * FROM kv".to_string(),
                            params: vec![],
                            consistency: SqlConsistency::Linearizable,
                            limit: Some(10000),
                            timeout_ms: Some(30000),
                        })
                        .await
                        .expect("sql query failed")
                }
            })
        });

        // Cleanup nodes for this iteration
        rt.block_on(async {
            for node in nodes {
                let _ = node.shutdown().await;
            }
        });
    }

    group.finish();
}

criterion_group!(
    production_benches,
    bench_prod_write_single,
    bench_prod_read_single,
    bench_prod_write_3node,
    bench_prod_read_3node,
    bench_redb_write_single,
    bench_redb_read_single,
    bench_redb_write_3node,
    bench_redb_read_3node,
);

#[cfg(feature = "sql")]
criterion_group!(
    sql_3node_benches,
    bench_sql_select_all_3node,
    bench_sql_point_lookup_3node,
    bench_sql_prefix_scan_3node,
    bench_sql_count_3node,
    bench_sql_stale_vs_linearizable_3node,
    bench_sql_data_sizes_3node,
);

#[cfg(feature = "sql")]
criterion_main!(production_benches, sql_3node_benches);

#[cfg(not(feature = "sql"))]
criterion_main!(production_benches);
