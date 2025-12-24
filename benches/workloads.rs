//! YCSB-style workload benchmarks for Aspen.
//!
//! Industry-standard mixed workload patterns:
//! - `bench_workload_read_heavy`: 95% read, 5% write (photo tagging pattern)
//! - `bench_workload_write_heavy`: 5% read, 95% write (ingest pattern)
//! - `bench_workload_mixed`: 50% read, 50% write (session store pattern)
//!
//! Run with: `nix run .#bench workloads`

mod common;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use common::Operation;
use common::WorkloadPattern;
use common::generate_workload;
use common::populate_test_data;
use common::setup_single_node_cluster;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;

/// Number of operations per workload benchmark iteration.
const WORKLOAD_OPS: usize = 1_000;

/// Key space size for workload benchmarks.
const KEY_SPACE: usize = 10_000;

/// Benchmark: Read-heavy workload (95% read, 5% write).
///
/// Simulates photo tagging, user profile reads, and similar read-dominated patterns.
fn bench_workload_read_heavy(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Setup cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, KEY_SPACE).await.unwrap();
        (router, leader)
    });

    let mut group = c.benchmark_group("workload");
    group.throughput(Throughput::Elements(WORKLOAD_OPS as u64));

    // Generate deterministic workload
    let ops = generate_workload(WorkloadPattern::MostlyRead, WORKLOAD_OPS, KEY_SPACE);
    let op_idx = AtomicUsize::new(0);

    group.bench_function("read_heavy_95_5", |b| {
        b.to_async(&rt).iter(|| {
            let router = &router;
            let ops = &ops;
            let idx = op_idx.fetch_add(1, Ordering::Relaxed) % ops.len();

            async move {
                match &ops[idx] {
                    Operation::Read { key } => {
                        router.read(leader, key).await;
                    }
                    Operation::Write { key, value } => {
                        router.write(leader, key.clone(), value.clone()).await.unwrap();
                    }
                }
            }
        })
    });

    group.finish();
}

/// Benchmark: Write-heavy workload (5% read, 95% write).
///
/// Simulates log ingestion, event streaming, and write-dominated patterns.
fn bench_workload_write_heavy(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Setup cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, KEY_SPACE).await.unwrap();
        (router, leader)
    });

    let mut group = c.benchmark_group("workload");
    group.throughput(Throughput::Elements(WORKLOAD_OPS as u64));

    // Generate deterministic workload
    let ops = generate_workload(WorkloadPattern::WriteHeavy, WORKLOAD_OPS, KEY_SPACE);
    let op_idx = AtomicUsize::new(0);

    group.bench_function("write_heavy_5_95", |b| {
        b.to_async(&rt).iter(|| {
            let router = &router;
            let ops = &ops;
            let idx = op_idx.fetch_add(1, Ordering::Relaxed) % ops.len();

            async move {
                match &ops[idx] {
                    Operation::Read { key } => {
                        router.read(leader, key).await;
                    }
                    Operation::Write { key, value } => {
                        router.write(leader, key.clone(), value.clone()).await.unwrap();
                    }
                }
            }
        })
    });

    group.finish();
}

/// Benchmark: Mixed workload (50% read, 50% write).
///
/// Simulates session stores, shopping carts, and balanced read/write patterns.
fn bench_workload_mixed(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Setup cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, KEY_SPACE).await.unwrap();
        (router, leader)
    });

    let mut group = c.benchmark_group("workload");
    group.throughput(Throughput::Elements(WORKLOAD_OPS as u64));

    // Generate deterministic workload
    let ops = generate_workload(WorkloadPattern::Mixed, WORKLOAD_OPS, KEY_SPACE);
    let op_idx = AtomicUsize::new(0);

    group.bench_function("mixed_50_50", |b| {
        b.to_async(&rt).iter(|| {
            let router = &router;
            let ops = &ops;
            let idx = op_idx.fetch_add(1, Ordering::Relaxed) % ops.len();

            async move {
                match &ops[idx] {
                    Operation::Read { key } => {
                        router.read(leader, key).await;
                    }
                    Operation::Write { key, value } => {
                        router.write(leader, key.clone(), value.clone()).await.unwrap();
                    }
                }
            }
        })
    });

    group.finish();
}

/// Benchmark: Batch workload execution (measure aggregate throughput).
///
/// Runs a full workload batch to measure sustained throughput.
fn bench_workload_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Setup cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, KEY_SPACE).await.unwrap();
        (router, leader)
    });

    let mut group = c.benchmark_group("workload_batch");

    for (pattern_name, pattern) in [
        ("read_heavy", WorkloadPattern::MostlyRead),
        ("write_heavy", WorkloadPattern::WriteHeavy),
        ("mixed", WorkloadPattern::Mixed),
    ] {
        // Smaller batch for full execution benchmark
        let batch_size = 100;
        group.throughput(Throughput::Elements(batch_size as u64));

        let ops = generate_workload(pattern, batch_size, KEY_SPACE);

        group.bench_with_input(BenchmarkId::from_parameter(pattern_name), &ops, |b, ops| {
            b.to_async(&rt).iter(|| {
                let router = &router;
                let ops = ops.clone();

                async move {
                    for op in ops {
                        match op {
                            Operation::Read { key } => {
                                router.read(leader, &key).await;
                            }
                            Operation::Write { key, value } => {
                                router.write(leader, key, value).await.unwrap();
                            }
                        }
                    }
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    workload_benches,
    bench_workload_read_heavy,
    bench_workload_write_heavy,
    bench_workload_mixed,
    bench_workload_batch,
);

criterion_main!(workload_benches);
