//! Production-realistic benchmarks using real storage and networking.
//!
//! Unlike kv_operations.rs which uses InMemoryStateMachine for protocol testing,
//! these benchmarks use NodeBuilder with SQLite storage and real Iroh networking
//! to measure production-like latencies.
//!
//! ## Benchmark Groups
//!
//! - `prod_write`: Single-node write through Raft with SQLite + fsync
//! - `prod_read`: Single-node read with ReadIndex linearizability
//! - `prod_write_3node`: 3-node write with real Iroh QUIC quorum
//! - `prod_read_3node`: 3-node read with real network
//!
//! ## Expected Results
//!
//! - Single-node writes: 2-5ms (SQLite fsync dominates)
//! - Single-node reads: 50-200us (SQLite pool + ReadIndex)
//! - 3-node writes: 5-15ms (fsync + network RTT + quorum)
//! - 3-node reads: 100-500us (ReadIndex over network)
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

        // Wait for replication
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

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

criterion_group!(
    production_benches,
    bench_prod_write_single,
    bench_prod_read_single,
    bench_prod_write_3node,
    bench_prod_read_3node,
);
criterion_main!(production_benches);
