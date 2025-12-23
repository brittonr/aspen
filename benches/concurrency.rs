//! Concurrency benchmarks for Aspen.
//!
//! Measures contention and throughput under concurrent load:
//! - `bench_parallel_writers`: 2, 4, 8 concurrent writers
//! - `bench_parallel_readers`: 2, 4, 8 concurrent readers
//! - `bench_mixed_concurrent`: Writers + readers together
//!
//! Run with: `nix run .#bench concurrency`

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use common::{generate_value, populate_test_data, setup_single_node_cluster};

/// Value size for concurrency benchmarks.
const BENCH_VALUE_SIZE: usize = 256;

/// Number of operations per concurrent task.
const OPS_PER_TASK: usize = 10;

/// Pre-populated key space size.
const KEY_SPACE: usize = 10_000;

/// Benchmark: Parallel writers (2, 4, 8 concurrent writers).
///
/// Measures aggregate write throughput with concurrent Raft proposals.
fn bench_parallel_writers(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    // Setup cluster
    let (router, leader) = rt.block_on(async { setup_single_node_cluster().await.unwrap() });
    let router = Arc::new(router);

    let mut group = c.benchmark_group("concurrent_writers");

    for num_writers in [2usize, 4, 8] {
        let total_ops = num_writers * OPS_PER_TASK;
        group.throughput(Throughput::Elements(total_ops as u64));

        let counter = Arc::new(AtomicU64::new(0));
        let value = generate_value(BENCH_VALUE_SIZE);

        group.bench_with_input(
            BenchmarkId::from_parameter(num_writers),
            &num_writers,
            |b, &writers| {
                b.to_async(&rt).iter(|| {
                    let router = router.clone();
                    let counter = counter.clone();
                    let value = value.clone();

                    async move {
                        let mut handles = Vec::with_capacity(writers);

                        for _writer_id in 0..writers {
                            let router = router.clone();
                            let counter = counter.clone();
                            let value = value.clone();

                            handles.push(tokio::spawn(async move {
                                for _ in 0..OPS_PER_TASK {
                                    let key = format!(
                                        "concurrent-{:012}",
                                        counter.fetch_add(1, Ordering::Relaxed)
                                    );
                                    router.write(leader, key, value.clone()).await.unwrap();
                                }
                            }));
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Parallel readers (2, 4, 8 concurrent readers).
///
/// Measures aggregate read throughput with concurrent state machine reads.
fn bench_parallel_readers(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    // Setup cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, KEY_SPACE)
            .await
            .unwrap();
        (router, leader)
    });
    let router = Arc::new(router);

    let mut group = c.benchmark_group("concurrent_readers");

    for num_readers in [2usize, 4, 8] {
        let total_ops = num_readers * OPS_PER_TASK;
        group.throughput(Throughput::Elements(total_ops as u64));

        let counter = Arc::new(AtomicU64::new(0));

        group.bench_with_input(
            BenchmarkId::from_parameter(num_readers),
            &num_readers,
            |b, &readers| {
                b.to_async(&rt).iter(|| {
                    let router = router.clone();
                    let counter = counter.clone();

                    async move {
                        let mut handles = Vec::with_capacity(readers);

                        for _reader_id in 0..readers {
                            let router = router.clone();
                            let counter = counter.clone();

                            handles.push(tokio::spawn(async move {
                                for _ in 0..OPS_PER_TASK {
                                    let idx =
                                        counter.fetch_add(1, Ordering::Relaxed) % KEY_SPACE as u64;
                                    let key = format!("key-{:06}", idx);
                                    router.read(leader, &key).await;
                                }
                            }));
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Mixed concurrent operations (readers + writers together).
///
/// Simulates realistic workload with concurrent read and write operations.
fn bench_mixed_concurrent(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    // Setup cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, KEY_SPACE)
            .await
            .unwrap();
        (router, leader)
    });
    let router = Arc::new(router);

    let mut group = c.benchmark_group("concurrent_mixed");

    // Test different reader/writer ratios
    for (readers, writers) in [(4, 2), (2, 4), (4, 4)] {
        let total_ops = (readers + writers) * OPS_PER_TASK;
        group.throughput(Throughput::Elements(total_ops as u64));

        let read_counter = Arc::new(AtomicU64::new(0));
        let write_counter = Arc::new(AtomicU64::new(0));
        let value = generate_value(BENCH_VALUE_SIZE);

        let config_name = format!("r{}_w{}", readers, writers);

        group.bench_with_input(
            BenchmarkId::from_parameter(&config_name),
            &(readers, writers),
            |b, &(r, w)| {
                b.to_async(&rt).iter(|| {
                    let router = router.clone();
                    let read_counter = read_counter.clone();
                    let write_counter = write_counter.clone();
                    let value = value.clone();

                    async move {
                        let mut handles = Vec::with_capacity(r + w);

                        // Spawn readers
                        for _ in 0..r {
                            let router = router.clone();
                            let counter = read_counter.clone();

                            handles.push(tokio::spawn(async move {
                                for _ in 0..OPS_PER_TASK {
                                    let idx =
                                        counter.fetch_add(1, Ordering::Relaxed) % KEY_SPACE as u64;
                                    let key = format!("key-{:06}", idx);
                                    router.read(leader, &key).await;
                                }
                            }));
                        }

                        // Spawn writers
                        for _ in 0..w {
                            let router = router.clone();
                            let counter = write_counter.clone();
                            let value = value.clone();

                            handles.push(tokio::spawn(async move {
                                for _ in 0..OPS_PER_TASK {
                                    let key = format!(
                                        "mixed-{:012}",
                                        counter.fetch_add(1, Ordering::Relaxed)
                                    );
                                    router.write(leader, key, value.clone()).await.unwrap();
                                }
                            }));
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    concurrency_benches,
    bench_parallel_writers,
    bench_parallel_readers,
    bench_mixed_concurrent,
);

criterion_main!(concurrency_benches);
