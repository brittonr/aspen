use std::sync::Arc;
use std::time::Instant;

use aspen::raft::storage_sqlite::SqliteStateMachine;
use rusqlite::{Connection, params};
use tokio::task::JoinSet;

/// Benchmark read concurrency with connection pooling.
///
/// Tests:
/// 1. Sequential reads (baseline)
/// 2. Concurrent reads with pool size 1 (simulates single connection)
/// 3. Concurrent reads with pool size 10 (demonstrates pooling benefit)
///
/// Expected results:
/// - Pool size 10 should significantly outperform pool size 1 for concurrent reads
/// - Sequential reads establish baseline performance
#[tokio::main]
async fn main() {
    println!("SQLite Storage Read Concurrency Benchmark");
    println!("==========================================\n");

    // Prepare test data
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("benchmark.db");

    // Initialize database with test data
    println!("Initializing database with 1000 key-value pairs...");

    // Populate database directly (before creating SqliteStateMachine)
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS state_machine_kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        for i in 0..1000 {
            conn.execute(
                "INSERT INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                params![format!("key_{}", i), format!("value_{}", i)],
            )
            .unwrap();
        }
    }

    let sm = SqliteStateMachine::new(&db_path).unwrap();
    println!("Database initialized.\n");

    // Benchmark 1: Sequential reads (baseline)
    println!("Benchmark 1: Sequential Reads (baseline)");
    println!("-----------------------------------------");
    let sm_seq = Arc::clone(&sm);
    bench_sequential_reads(sm_seq, 1000).await;

    // Benchmark 2: Concurrent reads with pool size 1 (simulates old behavior)
    println!("\nBenchmark 2: Concurrent Reads (10 readers, pool size 1)");
    println!("--------------------------------------------------------");
    let temp_dir_p1 = tempfile::tempdir().unwrap();
    let db_path_p1 = temp_dir_p1.path().join("benchmark_p1.db");

    // Copy database
    std::fs::copy(&db_path, &db_path_p1).unwrap();

    let sm_p1 = SqliteStateMachine::with_pool_size(&db_path_p1, 1).unwrap();
    bench_concurrent_reads(sm_p1, 10, 1000).await;

    // Benchmark 3: Concurrent reads with pool size 10 (demonstrates pooling)
    println!("\nBenchmark 3: Concurrent Reads (10 readers, pool size 10)");
    println!("---------------------------------------------------------");
    let temp_dir_p10 = tempfile::tempdir().unwrap();
    let db_path_p10 = temp_dir_p10.path().join("benchmark_p10.db");

    // Copy database
    std::fs::copy(&db_path, &db_path_p10).unwrap();

    let sm_p10 = SqliteStateMachine::with_pool_size(&db_path_p10, 10).unwrap();
    bench_concurrent_reads(sm_p10, 10, 1000).await;

    println!("\n==========================================");
    println!("Benchmark Complete");
    println!("==========================================");
}

/// Benchmark sequential reads.
async fn bench_sequential_reads(sm: Arc<SqliteStateMachine>, num_reads: u32) {
    let start = Instant::now();

    for i in 0..num_reads {
        let key = format!("key_{}", i % 1000);
        let _ = sm.get(&key).await.unwrap();
    }

    let elapsed = start.elapsed();
    let ops_per_sec = num_reads as f64 / elapsed.as_secs_f64();

    println!("Reads: {}", num_reads);
    println!("Duration: {:?}", elapsed);
    println!("Throughput: {:.0} ops/sec", ops_per_sec);
}

/// Benchmark concurrent reads with multiple readers.
async fn bench_concurrent_reads(
    sm: Arc<SqliteStateMachine>,
    num_readers: u32,
    reads_per_reader: u32,
) {
    let start = Instant::now();
    let mut join_set = JoinSet::new();

    for reader_id in 0..num_readers {
        let sm_clone = Arc::clone(&sm);
        join_set.spawn(async move {
            for i in 0..reads_per_reader {
                let key = format!("key_{}", (reader_id * reads_per_reader + i) % 1000);
                let _ = sm_clone.get(&key).await.unwrap();
            }
        });
    }

    // Wait for all readers to complete
    while join_set.join_next().await.is_some() {}

    let elapsed = start.elapsed();
    let total_reads = num_readers * reads_per_reader;
    let ops_per_sec = total_reads as f64 / elapsed.as_secs_f64();

    println!("Readers: {}", num_readers);
    println!("Reads per reader: {}", reads_per_reader);
    println!("Total reads: {}", total_reads);
    println!("Duration: {:?}", elapsed);
    println!("Throughput: {:.0} ops/sec", ops_per_sec);
}
