//! DataFusion SQL layer benchmarks.
//!
//! Measures SQL query performance on Redb storage:
//! - Full table scans
//! - Exact key lookups (filter pushdown)
//! - Prefix scans (LIKE with filter pushdown)
//! - Aggregations (COUNT, etc.)
//! - ORDER BY operations
//!
//! Run with: `nix develop -c cargo bench --bench sql`

use std::sync::Arc;

use aspen::raft::storage_shared::KvEntry;
use aspen::sql::RedbSqlExecutor;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use redb::{Database, ReadableTable, TableDefinition};
use tempfile::TempDir;

/// State machine KV table (must match storage_shared.rs)
const SM_KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_kv");

/// Setup a Redb database with N test entries.
fn setup_redb_with_data(temp_dir: &TempDir, count: usize) -> Arc<Database> {
    let db_path = temp_dir.path().join("bench.redb");
    let db = Database::create(&db_path).expect("create db");

    // Initialize table
    {
        let write_txn = db.begin_write().expect("begin write");
        {
            let _table = write_txn.open_table(SM_KV_TABLE).expect("open table");
        }
        write_txn.commit().expect("commit init");
    }

    // Insert test data
    {
        let write_txn = db.begin_write().expect("begin write");
        {
            let mut table = write_txn.open_table(SM_KV_TABLE).expect("open table");
            for i in 0..count {
                let key = format!("key_{:08}", i);
                let entry = KvEntry {
                    value: format!("value_{}", i),
                    version: 1,
                    create_revision: i as i64,
                    mod_revision: i as i64,
                    expires_at_ms: None,
                    lease_id: None,
                };
                let bytes = bincode::serialize(&entry).expect("serialize");
                table
                    .insert(key.as_bytes(), bytes.as_slice())
                    .expect("insert");
            }
        }
        write_txn.commit().expect("commit data");
    }

    Arc::new(db)
}

/// Setup Redb with prefixed data for scan benchmarks.
fn setup_redb_with_prefixed_data(
    temp_dir: &TempDir,
    prefixes: usize,
    keys_per_prefix: usize,
) -> Arc<Database> {
    let db_path = temp_dir.path().join("bench_prefix.redb");
    let db = Database::create(&db_path).expect("create db");

    // Initialize table
    {
        let write_txn = db.begin_write().expect("begin write");
        {
            let _table = write_txn.open_table(SM_KV_TABLE).expect("open table");
        }
        write_txn.commit().expect("commit init");
    }

    // Insert prefixed data
    {
        let write_txn = db.begin_write().expect("begin write");
        {
            let mut table = write_txn.open_table(SM_KV_TABLE).expect("open table");
            let mut revision = 0i64;
            for prefix_id in 0..prefixes {
                for key_id in 0..keys_per_prefix {
                    let key = format!("prefix_{:02}:key_{:06}", prefix_id, key_id);
                    let entry = KvEntry {
                        value: format!("value_{}_{}", prefix_id, key_id),
                        version: 1,
                        create_revision: revision,
                        mod_revision: revision,
                        expires_at_ms: None,
                        lease_id: None,
                    };
                    revision += 1;
                    let bytes = bincode::serialize(&entry).expect("serialize");
                    table
                        .insert(key.as_bytes(), bytes.as_slice())
                        .expect("insert");
                }
            }
        }
        write_txn.commit().expect("commit data");
    }

    Arc::new(db)
}

// ====================================================================================
// Full Table Scan Benchmarks
// ====================================================================================

/// Benchmark: SELECT * FROM kv (full table scan)
fn bench_sql_select_all(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 1000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_select_all");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("1000_rows", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute("SELECT * FROM kv", &[], Some(10000), Some(30000))
                    .await
                    .expect("query failed")
            }
        })
    });

    group.finish();
}

/// Benchmark: SELECT * with varying table sizes
fn bench_sql_scan_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("sql_scan_sizes");

    for size in [100, 1000, 5000, 10000] {
        let temp_dir = TempDir::new().expect("temp dir");
        let db = setup_redb_with_data(&temp_dir, size);
        let executor = RedbSqlExecutor::new(db);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let exec = &executor;
                async move {
                    exec.execute("SELECT * FROM kv", &[], Some(10000), Some(30000))
                        .await
                        .expect("query failed")
                }
            })
        });
    }

    group.finish();
}

// ====================================================================================
// Filter Pushdown Benchmarks
// ====================================================================================

/// Benchmark: WHERE key = 'exact' (exact key lookup with pushdown)
fn bench_sql_exact_key(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 10000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_filter_exact");
    group.throughput(Throughput::Elements(1));

    group.bench_function("point_lookup", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute(
                    "SELECT key, value FROM kv WHERE key = 'key_00005000'",
                    &[],
                    Some(1),
                    Some(5000),
                )
                .await
                .expect("query failed")
            }
        })
    });

    group.finish();
}

/// Benchmark: WHERE key LIKE 'prefix%' (prefix scan with pushdown)
fn bench_sql_prefix_scan(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    // 10 prefixes, 1000 keys each = 10,000 total
    let db = setup_redb_with_prefixed_data(&temp_dir, 10, 1000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_filter_prefix");
    group.throughput(Throughput::Elements(1000)); // Each prefix has 1000 keys

    group.bench_function("prefix_1000_keys", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute(
                    "SELECT key, value FROM kv WHERE key LIKE 'prefix_05:%'",
                    &[],
                    Some(10000),
                    Some(30000),
                )
                .await
                .expect("query failed")
            }
        })
    });

    group.finish();
}

/// Benchmark: Range queries (WHERE key >= 'a' AND key < 'b')
fn bench_sql_range_query(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 10000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_filter_range");
    // Range covers ~1000 keys (key_00001000 to key_00002000)
    group.throughput(Throughput::Elements(1000));

    group.bench_function("range_1000_keys", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute(
                    "SELECT key, value FROM kv WHERE key >= 'key_00001000' AND key < 'key_00002000'",
                    &[],
                    Some(10000),
                    Some(30000),
                )
                .await
                .expect("query failed")
            }
        })
    });

    group.finish();
}

// ====================================================================================
// Aggregation Benchmarks
// ====================================================================================

/// Benchmark: COUNT(*) aggregation
fn bench_sql_count(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut group = c.benchmark_group("sql_aggregation_count");

    for size in [1000, 5000, 10000] {
        let temp_dir = TempDir::new().expect("temp dir");
        let db = setup_redb_with_data(&temp_dir, size);
        let executor = RedbSqlExecutor::new(db);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let exec = &executor;
                async move {
                    exec.execute("SELECT COUNT(*) FROM kv", &[], Some(1), Some(30000))
                        .await
                        .expect("query failed")
                }
            })
        });
    }

    group.finish();
}

/// Benchmark: COUNT with WHERE clause (filter then count)
fn bench_sql_count_filtered(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_prefixed_data(&temp_dir, 10, 1000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_aggregation_count_filtered");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("count_prefix_1000", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute(
                    "SELECT COUNT(*) FROM kv WHERE key LIKE 'prefix_03:%'",
                    &[],
                    Some(1),
                    Some(30000),
                )
                .await
                .expect("query failed")
            }
        })
    });

    group.finish();
}

// ====================================================================================
// Projection Benchmarks
// ====================================================================================

/// Benchmark: Column projection (SELECT key only vs SELECT *)
fn bench_sql_projection(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 5000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_projection");
    group.throughput(Throughput::Elements(5000));

    group.bench_function("select_all_columns", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute("SELECT * FROM kv", &[], Some(10000), Some(30000))
                    .await
                    .expect("query failed")
            }
        })
    });

    group.bench_function("select_key_value_only", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute("SELECT key, value FROM kv", &[], Some(10000), Some(30000))
                    .await
                    .expect("query failed")
            }
        })
    });

    group.bench_function("select_key_only", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute("SELECT key FROM kv", &[], Some(10000), Some(30000))
                    .await
                    .expect("query failed")
            }
        })
    });

    group.finish();
}

// ====================================================================================
// ORDER BY Benchmarks
// ====================================================================================

/// Benchmark: ORDER BY operations
fn bench_sql_order_by(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 5000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_order_by");
    group.throughput(Throughput::Elements(5000));

    group.bench_function("no_order", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute("SELECT key, value FROM kv", &[], Some(10000), Some(30000))
                    .await
                    .expect("query failed")
            }
        })
    });

    group.bench_function("order_by_key_asc", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute(
                    "SELECT key, value FROM kv ORDER BY key ASC",
                    &[],
                    Some(10000),
                    Some(30000),
                )
                .await
                .expect("query failed")
            }
        })
    });

    group.bench_function("order_by_key_desc", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute(
                    "SELECT key, value FROM kv ORDER BY key DESC",
                    &[],
                    Some(10000),
                    Some(30000),
                )
                .await
                .expect("query failed")
            }
        })
    });

    group.finish();
}

// ====================================================================================
// LIMIT Benchmarks
// ====================================================================================

/// Benchmark: LIMIT clause performance
fn bench_sql_limit(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 10000);
    let executor = RedbSqlExecutor::new(db);

    let mut group = c.benchmark_group("sql_limit");

    for limit in [10, 100, 1000, 5000] {
        group.throughput(Throughput::Elements(limit as u64));

        group.bench_with_input(BenchmarkId::from_parameter(limit), &limit, |b, &lim| {
            b.to_async(&rt).iter(|| {
                let exec = &executor;
                async move {
                    exec.execute("SELECT * FROM kv", &[], Some(lim as u32), Some(30000))
                        .await
                        .expect("query failed")
                }
            })
        });
    }

    group.finish();
}

// ====================================================================================
// Comparison: Raw Redb vs DataFusion
// ====================================================================================

/// Benchmark: Compare raw Redb scan vs DataFusion SQL scan
fn bench_sql_vs_raw_redb(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let temp_dir = TempDir::new().expect("temp dir");
    let db = setup_redb_with_data(&temp_dir, 1000);
    let executor = RedbSqlExecutor::new(db.clone());

    let mut group = c.benchmark_group("sql_vs_raw_redb");
    group.throughput(Throughput::Elements(1000));

    // Raw Redb scan
    group.bench_function("raw_redb_scan", |b| {
        b.iter(|| {
            let read_txn = db.begin_read().expect("begin read");
            let table = read_txn.open_table(SM_KV_TABLE).expect("open table");
            let mut count = 0usize;
            for result in table.iter().expect("iter") {
                let (key, value) = result.expect("entry");
                let _key = key.value();
                let _entry: KvEntry = bincode::deserialize(value.value()).expect("deserialize");
                count += 1;
            }
            count
        })
    });

    // DataFusion SQL scan
    group.bench_function("datafusion_sql_scan", |b| {
        b.to_async(&rt).iter(|| {
            let exec = &executor;
            async move {
                exec.execute("SELECT * FROM kv", &[], Some(10000), Some(30000))
                    .await
                    .expect("query failed")
            }
        })
    });

    group.finish();
}

criterion_group!(
    sql_benches,
    bench_sql_select_all,
    bench_sql_scan_sizes,
    bench_sql_exact_key,
    bench_sql_prefix_scan,
    bench_sql_range_query,
    bench_sql_count,
    bench_sql_count_filtered,
    bench_sql_projection,
    bench_sql_order_by,
    bench_sql_limit,
    bench_sql_vs_raw_redb,
);

criterion_main!(sql_benches);
