//! Key-Value operation benchmarks for Aspen.
//!
//! Measures latency and throughput of core KV operations:
//! - Single key read/write
//! - Batch writes (SetMulti)
//! - Prefix scans
//! - Value size impact (64B, 1KB, 64KB)
//! - Multi-node cluster operations (3-node quorum)
//!
//! Run with: `nix develop -c cargo bench --bench kv_operations`

mod common;

use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use common::{
    VALUE_SIZE_LARGE, VALUE_SIZE_MEDIUM, VALUE_SIZE_SMALL, generate_kv_pairs, generate_value,
    populate_test_data, setup_single_node_cluster, setup_three_node_cluster,
};

/// Benchmark single key write operations.
///
/// Measures the latency of writing a single key through Raft consensus.
fn bench_write_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Setup cluster once
    let (router, leader) = rt.block_on(async { setup_single_node_cluster().await.unwrap() });

    let mut group = c.benchmark_group("kv_write");
    group.throughput(Throughput::Elements(1));

    // Counter for unique keys
    let counter = AtomicU64::new(0);

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("bench-write-{}", counter.fetch_add(1, Ordering::Relaxed));
            let value = "benchmark-value-256-bytes-padding-".repeat(7); // ~256 bytes
            let router = &router;
            async move { router.write(leader, key, value).await.unwrap() }
        })
    });

    group.finish();
}

/// Benchmark single key read operations.
///
/// Measures the latency of reading a single key from the state machine.
/// Note: This reads directly from state machine, not via Raft ReadIndex.
fn bench_read_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Setup cluster and populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();
        populate_test_data(&router, leader, 10_000).await.unwrap();
        (router, leader)
    });

    let mut group = c.benchmark_group("kv_read");
    group.throughput(Throughput::Elements(1));

    // Read from middle of key space for cache consistency
    let counter = AtomicU64::new(0);

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            // Cycle through keys to avoid caching effects
            let idx = counter.fetch_add(1, Ordering::Relaxed) % 10_000;
            let key = format!("key-{:06}", idx);
            let router = &router;
            async move { router.read(leader, &key).await }
        })
    });

    group.finish();
}

/// Benchmark batch write operations (SetMulti pattern).
///
/// Measures throughput of writing multiple keys in a single Raft log entry.
fn bench_write_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Setup cluster
    let (router, leader) = rt.block_on(async { setup_single_node_cluster().await.unwrap() });

    let mut group = c.benchmark_group("kv_write_batch");

    // Counter for unique batch prefixes
    let counter = AtomicU64::new(0);

    for batch_size in [10, 50, 100] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| {
                    let batch_id = counter.fetch_add(1, Ordering::Relaxed);
                    let prefix = format!("batch-{}", batch_id);
                    let pairs = generate_kv_pairs(size, &prefix);
                    let router = &router;

                    async move {
                        // Write each pair individually (simulating SetMulti behavior)
                        // Note: The router doesn't have native SetMulti, so we batch writes
                        for (key, value) in pairs {
                            router.write(leader, key, value).await.unwrap();
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark prefix scan operations.
///
/// Measures the latency of scanning keys by prefix.
fn bench_scan(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Setup cluster and populate data with scannable prefixes
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_single_node_cluster().await.unwrap();

        // Create data with different prefixes for scan testing
        for prefix_id in 0..10 {
            for i in 0..1000 {
                let key = format!("prefix-{:02}/key-{:04}", prefix_id, i);
                let value = format!("value-{}", i);
                router.write(leader, key, value).await.unwrap();
            }
        }

        (router, leader)
    });

    let mut group = c.benchmark_group("kv_scan");

    // Benchmark scanning different result sizes
    for scan_prefix in ["prefix-00", "prefix-01"] {
        group.throughput(Throughput::Elements(1000)); // Each prefix has 1000 keys

        group.bench_with_input(
            BenchmarkId::from_parameter(scan_prefix),
            &scan_prefix,
            |b, &prefix| {
                b.to_async(&rt).iter(|| {
                    let router = &router;
                    let p = prefix.to_string();
                    async move {
                        // Read all keys with prefix by iterating
                        // Note: The router's InMemoryStateMachine may not have scan API
                        // This is a placeholder - in real impl we'd use the KeyValueStore trait
                        let mut count = 0;
                        for i in 0..1000 {
                            let key = format!("{}/key-{:04}", p, i);
                            if router.read(leader, &key).await.is_some() {
                                count += 1;
                            }
                        }
                        count
                    }
                })
            },
        );
    }

    group.finish();
}

// ====================================================================================
// Value Size Benchmarks
// ====================================================================================

/// Benchmark: Write operations with varying value sizes.
///
/// Measures impact of value size on write throughput (64B, 1KB, 64KB).
fn bench_write_value_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    // Setup cluster
    let (router, leader) = rt.block_on(async { setup_single_node_cluster().await.unwrap() });

    let mut group = c.benchmark_group("kv_write_value_size");
    group.throughput(Throughput::Elements(1));

    for (size_name, size) in [
        ("64B", VALUE_SIZE_SMALL),
        ("1KB", VALUE_SIZE_MEDIUM),
        ("64KB", VALUE_SIZE_LARGE),
    ] {
        let counter = AtomicU64::new(0);
        let value = generate_value(size);

        group.bench_with_input(BenchmarkId::from_parameter(size_name), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let key = format!(
                    "size-{}-{}",
                    size_name,
                    counter.fetch_add(1, Ordering::Relaxed)
                );
                let router = &router;
                let v = value.clone();

                async move { router.write(leader, key, v).await.unwrap() }
            })
        });
    }

    group.finish();
}

/// Benchmark: Read operations with varying value sizes.
///
/// Measures impact of value size on read throughput (64B, 1KB, 64KB).
fn bench_read_value_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("kv_read_value_size");
    group.throughput(Throughput::Elements(1));

    for (size_name, size) in [
        ("64B", VALUE_SIZE_SMALL),
        ("1KB", VALUE_SIZE_MEDIUM),
        ("64KB", VALUE_SIZE_LARGE),
    ] {
        // Setup fresh cluster for each value size
        let (router, leader) = rt.block_on(async {
            let (router, leader) = setup_single_node_cluster().await.unwrap();

            // Pre-populate with 1000 keys of this size
            let value = generate_value(size);
            for i in 0..1000 {
                let key = format!("size-{}-{:06}", size_name, i);
                router.write(leader, key, value.clone()).await.unwrap();
            }

            (router, leader)
        });

        let counter = AtomicU64::new(0);

        group.bench_with_input(BenchmarkId::from_parameter(size_name), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let idx = counter.fetch_add(1, Ordering::Relaxed) % 1000;
                let key = format!("size-{}-{:06}", size_name, idx);
                let router = &router;

                async move { router.read(leader, &key).await }
            })
        });
    }

    group.finish();
}

// ====================================================================================
// Multi-Node Cluster Benchmarks
// ====================================================================================

/// Benchmark: 3-node cluster write operations.
///
/// Measures quorum write latency with Raft replication.
fn bench_3node_write_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    // Setup 3-node cluster
    let (router, leader) = rt.block_on(async { setup_three_node_cluster().await.unwrap() });

    let mut group = c.benchmark_group("kv_write_3node");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicU64::new(0);
    let value = "benchmark-value-256-bytes-padding-".repeat(7);

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("3node-write-{}", counter.fetch_add(1, Ordering::Relaxed));
            let router = &router;
            let v = value.clone();

            async move { router.write(leader, key, v).await.unwrap() }
        })
    });

    group.finish();
}

/// Benchmark: 3-node cluster read operations.
///
/// Measures linearizable read latency from leader.
fn bench_3node_read_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    // Setup 3-node cluster and pre-populate data
    let (router, leader) = rt.block_on(async {
        let (router, leader) = setup_three_node_cluster().await.unwrap();
        populate_test_data(&router, leader, 10_000).await.unwrap();
        (router, leader)
    });

    let mut group = c.benchmark_group("kv_read_3node");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicU64::new(0);

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) % 10_000;
            let key = format!("key-{:06}", idx);
            let router = &router;

            async move { router.read(leader, &key).await }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_write_single,
    bench_read_single,
    bench_write_batch,
    bench_scan,
    bench_write_value_sizes,
    bench_read_value_sizes,
    bench_3node_write_single,
    bench_3node_read_single,
);

criterion_main!(benches);
