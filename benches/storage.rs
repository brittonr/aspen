//! Storage layer benchmarks for Aspen.
//!
//! Isolates storage component performance independent of Raft consensus:
//!
//! ## SqliteStateMachine Benchmarks
//! - `bench_sqlite_write_single`: Single key write
//! - `bench_sqlite_write_batch`: Batch write (10, 50, 100 keys)
//! - `bench_sqlite_read_single`: Single key read from pool
//! - `bench_sqlite_read_concurrent`: Parallel reads (2, 4, 8, 16 readers)
//! - `bench_sqlite_scan`: Prefix scan with various result sizes
//!
//! ## Snapshot Benchmarks
//! - `bench_snapshot_create`: Snapshot creation (1K, 10K keys)
//! - `bench_snapshot_install`: Snapshot installation (1K, 10K keys)
//!
//! Run with: `nix run .#bench storage`

mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen::raft::storage_sqlite::SqliteStateMachine;
use common::generate_value;
use common::setup_sqlite_state_machine;
use common::setup_sqlite_state_machine_with_pool;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;

/// Default value size for benchmarks (256 bytes - realistic KV value).
const BENCH_VALUE_SIZE: usize = 256;

// ====================================================================================
// SQLite State Machine Benchmarks
// ====================================================================================

/// Populate SQLite state machine with test data using direct SQL.
///
/// Tiger Style: Bounded batch operations.
fn populate_sqlite_data(sm: &Arc<SqliteStateMachine>, count: usize, value_size: usize) {
    let value = generate_value(value_size);

    // Access write connection directly for fast population
    let write_conn = sm.write_conn.lock().expect("lock poisoned");

    // Use a transaction for faster bulk insert
    write_conn.execute("BEGIN IMMEDIATE", []).unwrap();

    for i in 0..count {
        let key = format!("key-{:08}", i);
        write_conn
            .execute("INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)", rusqlite::params![
                key, &value
            ])
            .unwrap();
    }

    write_conn.execute("COMMIT", []).unwrap();
}

/// Populate SQLite with hierarchical prefix data for scan benchmarks.
fn populate_sqlite_prefix_data(sm: &Arc<SqliteStateMachine>, prefixes: usize, keys_per_prefix: usize) {
    let value = generate_value(BENCH_VALUE_SIZE);
    let write_conn = sm.write_conn.lock().expect("lock poisoned");

    write_conn.execute("BEGIN IMMEDIATE", []).unwrap();

    for prefix in 0..prefixes {
        for i in 0..keys_per_prefix {
            let key = format!("prefix-{:02}/key-{:04}", prefix, i);
            write_conn
                .execute("INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)", rusqlite::params![
                    key, &value
                ])
                .unwrap();
        }
    }

    write_conn.execute("COMMIT", []).unwrap();
}

/// Benchmark: Single key write to SQLite state machine.
fn bench_sqlite_write_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let mut group = c.benchmark_group("sqlite_write");
    group.throughput(Throughput::Elements(1));

    // Setup state machine
    let (sm, _temp_dir) = setup_sqlite_state_machine().expect("setup failed");

    let counter = AtomicU64::new(0);
    let value = generate_value(BENCH_VALUE_SIZE);

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let key = format!("key-{:08}", counter.fetch_add(1, Ordering::Relaxed));
            let sm = sm.clone();
            let v = value.clone();

            async move {
                // Use direct SQL write to isolate state machine performance
                let write_conn = sm.write_conn.lock().expect("lock poisoned");
                write_conn
                    .execute("INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)", rusqlite::params![
                        key, v
                    ])
                    .unwrap();
            }
        })
    });

    group.finish();
}

/// Benchmark: Batch write to SQLite state machine (10, 50, 100 keys).
fn bench_sqlite_write_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let mut group = c.benchmark_group("sqlite_write_batch");

    for batch_size in [10usize, 50, 100] {
        group.throughput(Throughput::Elements(batch_size as u64));

        // Setup fresh state machine for each batch size
        let (sm, _temp_dir) = setup_sqlite_state_machine().expect("setup failed");
        let counter = AtomicU64::new(0);
        let value = generate_value(BENCH_VALUE_SIZE);

        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &batch_size, |b, &size| {
            b.to_async(&rt).iter(|| {
                let base = counter.fetch_add(size as u64, Ordering::Relaxed);
                let sm = sm.clone();
                let v = value.clone();

                async move {
                    let write_conn = sm.write_conn.lock().expect("lock poisoned");

                    // Use explicit transaction for batch
                    write_conn.execute("BEGIN IMMEDIATE", []).unwrap();

                    for i in 0..size {
                        let key = format!("key-{:08}", base + i as u64);
                        write_conn
                            .execute(
                                "INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                                rusqlite::params![key, &v],
                            )
                            .unwrap();
                    }

                    write_conn.execute("COMMIT", []).unwrap();
                }
            })
        });
    }

    group.finish();
}

/// Benchmark: Single key read from SQLite state machine.
fn bench_sqlite_read_single(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Pre-populate with 10,000 keys
    let (sm, _temp_dir) = setup_sqlite_state_machine().expect("setup failed");
    populate_sqlite_data(&sm, 10_000, BENCH_VALUE_SIZE);

    let mut group = c.benchmark_group("sqlite_read");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicU64::new(0);

    group.bench_function("single", |b| {
        b.to_async(&rt).iter(|| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) % 10_000;
            let key = format!("key-{:08}", idx);
            let sm = sm.clone();

            async move { sm.get(&key).await.unwrap() }
        })
    });

    group.finish();
}

/// Benchmark: Concurrent reads from SQLite state machine (2, 4, 8, 16 readers).
fn bench_sqlite_read_concurrent(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Pre-populate with 10,000 keys, use larger pool for concurrency testing
    let (sm, _temp_dir) = setup_sqlite_state_machine_with_pool(32).expect("setup failed");
    populate_sqlite_data(&sm, 10_000, BENCH_VALUE_SIZE);

    let mut group = c.benchmark_group("sqlite_read_concurrent");

    for num_readers in [2usize, 4, 8, 16] {
        group.throughput(Throughput::Elements(num_readers as u64));

        group.bench_with_input(BenchmarkId::from_parameter(num_readers), &num_readers, |b, &readers| {
            b.to_async(&rt).iter(|| {
                let sm = sm.clone();

                async move {
                    let mut handles = Vec::with_capacity(readers);

                    for reader_id in 0..readers {
                        let sm = sm.clone();
                        handles.push(tokio::spawn(async move {
                            let key = format!("key-{:08}", (reader_id * 100) % 10_000);
                            sm.get(&key).await.unwrap()
                        }));
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                }
            })
        });
    }

    group.finish();
}

/// Benchmark: Prefix scan from SQLite state machine.
fn bench_sqlite_scan(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    // Pre-populate with hierarchical keys: 10 prefixes x 1000 keys each
    let (sm, _temp_dir) = setup_sqlite_state_machine().expect("setup failed");
    populate_sqlite_prefix_data(&sm, 10, 1000);

    let mut group = c.benchmark_group("sqlite_scan");

    // Scan different prefixes (each has 1000 keys)
    for prefix_id in [0, 5, 9] {
        group.throughput(Throughput::Elements(1000));

        let prefix = format!("prefix-{:02}/", prefix_id);
        group.bench_with_input(BenchmarkId::new("prefix", prefix_id), &prefix, |b, prefix| {
            b.to_async(&rt).iter(|| {
                let sm = sm.clone();
                let p = prefix.clone();

                async move { sm.scan_kv_with_prefix_async(&p).await.unwrap() }
            })
        });
    }

    // Scan with various limits
    for limit in [100usize, 500, 1000] {
        group.throughput(Throughput::Elements(limit as u64));

        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &lim| {
            b.to_async(&rt).iter(|| {
                let sm = sm.clone();

                async move { sm.scan("prefix-00/", None, Some(lim)).await.unwrap() }
            })
        });
    }

    group.finish();
}

// ====================================================================================
// Snapshot Benchmarks
// ====================================================================================

/// Benchmark: Snapshot creation for various state sizes.
fn bench_snapshot_create(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

    let mut group = c.benchmark_group("snapshot_create");

    for key_count in [1_000usize, 10_000] {
        group.throughput(Throughput::Elements(key_count as u64));

        // Setup and populate state machine
        let (sm, _temp_dir) = setup_sqlite_state_machine().expect("setup failed");
        populate_sqlite_data(&sm, key_count, BENCH_VALUE_SIZE);

        group.bench_with_input(BenchmarkId::from_parameter(key_count), &key_count, |b, &_count| {
            b.to_async(&rt).iter(|| {
                let mut sm = sm.clone();

                async move {
                    use openraft::storage::RaftSnapshotBuilder;
                    // Build snapshot
                    sm.build_snapshot().await.unwrap()
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    storage_benches,
    bench_sqlite_write_single,
    bench_sqlite_write_batch,
    bench_sqlite_read_single,
    bench_sqlite_read_concurrent,
    bench_sqlite_scan,
    bench_snapshot_create,
);

criterion_main!(storage_benches);
