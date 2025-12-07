use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::stream;
use openraft::entry::RaftEntry;
use openraft::storage::{RaftSnapshotBuilder, RaftStateMachine};
use openraft::testing::log_id;
use tempfile::TempDir;

use aspen::raft::storage::{RedbStateMachine, StateMachineStore};
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::{AppRequest, AppTypeConfig};

/// Prepare a state machine with N key-value pairs.
/// Returns (state_machine, temp_dir) where temp_dir must be kept alive.
async fn prepare_inmemory_sm(size: u32) -> Arc<StateMachineStore> {
    let sm = StateMachineStore::new();

    // Apply entries to populate the state machine
    for i in 0..size {
        let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, 1, i as u64),
            AppRequest::Set {
                key: format!("key_{}", i),
                value: format!("value_{}", i),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

        let mut sm_clone = Arc::clone(&sm);
        sm_clone.apply(entries).await.unwrap();
    }

    sm
}

/// Prepare a redb state machine with N key-value pairs.
/// Returns (state_machine, temp_dir) where temp_dir must be kept alive.
#[allow(deprecated)]
async fn prepare_redb_sm(size: u32) -> (Arc<RedbStateMachine>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let sm_path = temp_dir.path().join("state_machine.redb");
    let sm = RedbStateMachine::new(&sm_path).unwrap();

    // Apply entries to populate the state machine
    for i in 0..size {
        let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, 1, i as u64),
            AppRequest::Set {
                key: format!("key_{}", i),
                value: format!("value_{}", i),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

        let mut sm_clone = Arc::clone(&sm);
        sm_clone.apply(entries).await.unwrap();
    }

    (sm, temp_dir)
}

/// Prepare a SQLite state machine with N key-value pairs.
/// Returns (state_machine, temp_dir) where temp_dir must be kept alive.
async fn prepare_sqlite_sm(size: u32) -> (Arc<SqliteStateMachine>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let sm_path = temp_dir.path().join("state_machine.db");
    let sm = SqliteStateMachine::new(&sm_path).unwrap();

    // Apply entries to populate the state machine
    for i in 0..size {
        let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, 1, i as u64),
            AppRequest::Set {
                key: format!("key_{}", i),
                value: format!("value_{}", i),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

        let mut sm_clone = Arc::clone(&sm);
        sm_clone.apply(entries).await.unwrap();
    }

    (sm, temp_dir)
}

/// Benchmark 1: Write throughput comparison
fn write_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_throughput");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Benchmark InMemory
    group.bench_function("inmemory_1000_writes", |b| {
        b.to_async(&runtime).iter(|| async {
            let sm = StateMachineStore::new();

            for i in 0..1000 {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    log_id::<AppTypeConfig>(1, 1, i),
                    AppRequest::Set {
                        key: format!("key_{}", i),
                        value: format!("value_{}", i),
                    },
                );
                let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

                let mut sm_clone = Arc::clone(&sm);
                sm_clone.apply(entries).await.unwrap();
            }

            black_box(sm)
        });
    });

    // Benchmark redb
    #[allow(deprecated)]
    group.bench_function("redb_1000_writes", |b| {
        b.to_async(&runtime).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let sm_path = temp_dir.path().join("bench.redb");
            let sm = RedbStateMachine::new(&sm_path).unwrap();

            for i in 0..1000 {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    log_id::<AppTypeConfig>(1, 1, i),
                    AppRequest::Set {
                        key: format!("key_{}", i),
                        value: format!("value_{}", i),
                    },
                );
                let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

                let mut sm_clone = Arc::clone(&sm);
                sm_clone.apply(entries).await.unwrap();
            }

            black_box((sm, temp_dir))
        });
    });

    // Benchmark SQLite
    group.bench_function("sqlite_1000_writes", |b| {
        b.to_async(&runtime).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let sm_path = temp_dir.path().join("bench.db");
            let sm = SqliteStateMachine::new(&sm_path).unwrap();

            for i in 0..1000 {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    log_id::<AppTypeConfig>(1, 1, i),
                    AppRequest::Set {
                        key: format!("key_{}", i),
                        value: format!("value_{}", i),
                    },
                );
                let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

                let mut sm_clone = Arc::clone(&sm);
                sm_clone.apply(entries).await.unwrap();
            }

            black_box((sm, temp_dir))
        });
    });

    group.finish();
}

/// Benchmark 2: Read latency comparison
fn read_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_latency");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Pre-populate with 10,000 entries
    let sm_inmemory = runtime.block_on(prepare_inmemory_sm(10000));

    #[allow(deprecated)]
    let (sm_redb, _temp_dir_redb) = runtime.block_on(prepare_redb_sm(10000));

    let (sm_sqlite, _temp_dir_sqlite) = runtime.block_on(prepare_sqlite_sm(10000));

    // Benchmark random reads across backends
    group.bench_function("inmemory_random_reads", |b| {
        b.to_async(&runtime).iter(|| async {
            // 100 random reads
            for i in 0..100 {
                let key = format!("key_{}", (i * 97) % 10000); // pseudo-random
                let value = sm_inmemory.get(&key).await;
                black_box(value);
            }
        });
    });

    #[allow(deprecated)]
    group.bench_function("redb_random_reads", |b| {
        b.to_async(&runtime).iter(|| async {
            // 100 random reads
            for i in 0..100 {
                let key = format!("key_{}", (i * 97) % 10000); // pseudo-random
                let value = sm_redb.get(&key).await.unwrap();
                black_box(value);
            }
        });
    });

    group.bench_function("sqlite_random_reads", |b| {
        b.to_async(&runtime).iter(|| async {
            // 100 random reads
            for i in 0..100 {
                let key = format!("key_{}", (i * 97) % 10000); // pseudo-random
                let value = sm_sqlite.get(&key).await.unwrap();
                black_box(value);
            }
        });
    });

    group.finish();
}

/// Benchmark 3: Snapshot speed comparison
fn snapshot_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_speed");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Test with different dataset sizes: 100, 1000, 10000 entries
    for size in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("inmemory_snapshot", size),
            &size,
            |b, &size| {
                let sm = runtime.block_on(prepare_inmemory_sm(size));

                b.to_async(&runtime).iter(|| async {
                    let mut sm_clone = Arc::clone(&sm);
                    let snapshot = sm_clone.build_snapshot().await.unwrap();
                    black_box(snapshot)
                });
            },
        );

        #[allow(deprecated)]
        group.bench_with_input(
            BenchmarkId::new("redb_snapshot", size),
            &size,
            |b, &size| {
                let (sm, _temp_dir) = runtime.block_on(prepare_redb_sm(size));

                b.to_async(&runtime).iter(|| async {
                    let mut sm_clone = Arc::clone(&sm);
                    let snapshot = sm_clone.build_snapshot().await.unwrap();
                    black_box(snapshot)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sqlite_snapshot", size),
            &size,
            |b, &size| {
                let (sm, _temp_dir) = runtime.block_on(prepare_sqlite_sm(size));

                b.to_async(&runtime).iter(|| async {
                    let mut sm_clone = Arc::clone(&sm);
                    let snapshot = sm_clone.build_snapshot().await.unwrap();
                    black_box(snapshot)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 4: Mixed workload (70% reads, 30% writes)
fn mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Benchmark InMemory with 70% reads, 30% writes
    group.bench_function("inmemory_mixed_70_30", |b| {
        b.to_async(&runtime).iter(|| async {
            let sm = runtime.block_on(prepare_inmemory_sm(1000));

            // 100 operations: 70 reads + 30 writes
            for i in 0..100 {
                if i < 70 {
                    // Read operation
                    let key = format!("key_{}", i % 1000);
                    let value = sm.get(&key).await;
                    black_box(value);
                } else {
                    // Write operation
                    let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                        log_id::<AppTypeConfig>(1, 1, 1000 + i),
                        AppRequest::Set {
                            key: format!("key_new_{}", i),
                            value: format!("value_new_{}", i),
                        },
                    );
                    let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

                    let mut sm_clone = Arc::clone(&sm);
                    sm_clone.apply(entries).await.unwrap();
                }
            }

            black_box(sm)
        });
    });

    // Benchmark SQLite with 70% reads, 30% writes
    group.bench_function("sqlite_mixed_70_30", |b| {
        b.to_async(&runtime).iter(|| async {
            let (sm, _temp_dir) = runtime.block_on(prepare_sqlite_sm(1000));

            // 100 operations: 70 reads + 30 writes
            for i in 0..100 {
                if i < 70 {
                    // Read operation
                    let key = format!("key_{}", i % 1000);
                    let value = sm.get(&key).await.unwrap();
                    black_box(value);
                } else {
                    // Write operation
                    let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                        log_id::<AppTypeConfig>(1, 1, 1000 + i),
                        AppRequest::Set {
                            key: format!("key_new_{}", i),
                            value: format!("value_new_{}", i),
                        },
                    );
                    let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

                    let mut sm_clone = Arc::clone(&sm);
                    sm_clone.apply(entries).await.unwrap();
                }
            }

            black_box((sm, _temp_dir))
        });
    });

    group.finish();
}

/// Benchmark 5: Transaction overhead (SQLite only)
fn transaction_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_overhead");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Single write per transaction
    group.bench_function("sqlite_transaction_1_write", |b| {
        b.to_async(&runtime).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let sm_path = temp_dir.path().join("bench.db");
            let sm = SqliteStateMachine::new(&sm_path).unwrap();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, 1, 1),
                AppRequest::Set {
                    key: "key_1".to_string(),
                    value: "value_1".to_string(),
                },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

            let mut sm_clone = Arc::clone(&sm);
            sm_clone.apply(entries).await.unwrap();

            black_box((sm, temp_dir))
        });
    });

    // 10 writes in one transaction (via SetMulti)
    group.bench_function("sqlite_transaction_10_writes", |b| {
        b.to_async(&runtime).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let sm_path = temp_dir.path().join("bench.db");
            let sm = SqliteStateMachine::new(&sm_path).unwrap();

            let pairs: Vec<(String, String)> = (0..10)
                .map(|i| (format!("key_{}", i), format!("value_{}", i)))
                .collect();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, 1, 1),
                AppRequest::SetMulti { pairs },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

            let mut sm_clone = Arc::clone(&sm);
            sm_clone.apply(entries).await.unwrap();

            black_box((sm, temp_dir))
        });
    });

    // 100 writes in one transaction (via SetMulti)
    group.bench_function("sqlite_transaction_100_writes", |b| {
        b.to_async(&runtime).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let sm_path = temp_dir.path().join("bench.db");
            let sm = SqliteStateMachine::new(&sm_path).unwrap();

            let pairs: Vec<(String, String)> = (0..100)
                .map(|i| (format!("key_{}", i), format!("value_{}", i)))
                .collect();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, 1, 1),
                AppRequest::SetMulti { pairs },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));

            let mut sm_clone = Arc::clone(&sm);
            sm_clone.apply(entries).await.unwrap();

            black_box((sm, temp_dir))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    write_throughput,
    read_latency,
    snapshot_speed,
    mixed_workload,
    transaction_overhead
);
criterion_main!(benches);
