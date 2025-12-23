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

use aspen::api::{ReadRequest, WriteCommand, WriteRequest};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

/// Benchmark single-node write with production storage (SQLite + fsync).
fn bench_prod_write_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        common::setup_production_single_node(&temp_dir)
            .await
            .expect("setup single node")
    });

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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        let node = common::setup_production_single_node(&temp_dir)
            .await
            .expect("setup single node");

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
            async move { kv.read(ReadRequest { key }).await.expect("read failed") }
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        common::setup_production_three_node(&temp_dir)
            .await
            .expect("setup 3-node cluster")
    });

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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node(&temp_dir)
            .await
            .expect("setup 3-node cluster");

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
            async move { kv.read(ReadRequest { key }).await.expect("read failed") }
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        common::setup_production_single_node_redb(&temp_dir)
            .await
            .expect("setup single node with redb")
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let node = rt.block_on(async {
        let node = common::setup_production_single_node_redb(&temp_dir)
            .await
            .expect("setup single node with redb");

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
            async move { kv.read(ReadRequest { key }).await.expect("read failed") }
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        common::setup_production_three_node_redb(&temp_dir)
            .await
            .expect("setup 3-node redb cluster")
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let nodes = rt.block_on(async {
        let nodes = common::setup_production_three_node_redb(&temp_dir)
            .await
            .expect("setup 3-node redb cluster");

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
            async move { kv.read(ReadRequest { key }).await.expect("read failed") }
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
criterion_main!(production_benches);
