# Metrics Implementation Examples for Aspen Soak Tests

This document provides ready-to-use code examples for implementing Tiger Style metrics collection in soak tests.

## Example 1: Simple Latency Histogram (Minimal Implementation)

```rust
// src/metrics.rs - Tiger Style metrics collection

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Minimal metrics struct with fixed-size histogram
/// Designed for soak tests following Tiger Style principles
#[derive(Debug, Clone)]
pub struct SoakMetrics {
    /// Total operations executed
    pub total_ops: u64,
    /// Successfully completed operations
    pub successful_ops: u32,
    /// Failed operations
    pub failed_ops: u32,
    /// Latency histogram: [0-1ms, 1-10ms, 10-100ms, 100-1s, 1-10s, 10s+]
    pub latency_buckets: [u32; 6],
    /// Total test duration
    pub total_duration_secs: f64,
}

impl SoakMetrics {
    pub fn new() -> Self {
        Self {
            total_ops: 0,
            successful_ops: 0,
            failed_ops: 0,
            latency_buckets: [0u32; 6],
            total_duration_secs: 0.0,
        }
    }

    /// Record a successful operation with its latency
    pub fn record_op_ms(&mut self, latency_ms: u32) {
        self.total_ops += 1;
        self.successful_ops = self.successful_ops.saturating_add(1);

        let bucket = match latency_ms {
            0..=1 => 0,
            2..=10 => 1,
            11..=100 => 2,
            101..=1000 => 3,
            1001..=10000 => 4,
            _ => 5,
        };

        if self.latency_buckets[bucket] < u32::MAX {
            self.latency_buckets[bucket] += 1;
        }
    }

    pub fn record_failure(&mut self) {
        self.total_ops += 1;
        self.failed_ops = self.failed_ops.saturating_add(1);
    }

    pub fn success_rate_percent(&self) -> f64 {
        if self.total_ops == 0 {
            0.0
        } else {
            (self.successful_ops as f64 / self.total_ops as f64) * 100.0
        }
    }

    pub fn throughput_ops_per_sec(&self) -> f64 {
        if self.total_duration_secs > 0.0 {
            self.successful_ops as f64 / self.total_duration_secs
        } else {
            0.0
        }
    }

    pub fn print_summary(&self) {
        println!("\n===== Soak Test Metrics =====");
        println!("Total operations:   {}", self.total_ops);
        println!("Successful:         {} ({:.2}%)",
                 self.successful_ops, self.success_rate_percent());
        println!("Failed:             {}", self.failed_ops);
        println!("Duration:           {:.2}s", self.total_duration_secs);
        println!("Throughput:         {:.2} ops/sec", self.throughput_ops_per_sec());
        println!("\nLatency Distribution:");
        println!("  0-1ms:     {} ops", self.latency_buckets[0]);
        println!("  1-10ms:    {} ops", self.latency_buckets[1]);
        println!("  10-100ms:  {} ops", self.latency_buckets[2]);
        println!("  100-1s:    {} ops", self.latency_buckets[3]);
        println!("  1-10s:     {} ops", self.latency_buckets[4]);
        println!("  10s+:      {} ops", self.latency_buckets[5]);
    }
}

/// Thread-safe metrics collector
pub struct MetricsCollector {
    metrics: Arc<Mutex<SoakMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(SoakMetrics::new())),
        }
    }

    pub fn record_success(&self, latency_ms: u32) {
        if let Ok(mut m) = self.metrics.lock() {
            m.record_op_ms(latency_ms);
        }
    }

    pub fn record_failure(&self) {
        if let Ok(mut m) = self.metrics.lock() {
            m.record_failure();
        }
    }

    pub fn snapshot(&self) -> Option<SoakMetrics> {
        self.metrics.lock().ok().map(|m| m.clone())
    }

    pub fn finalize(&self, duration: Duration) {
        if let Ok(mut m) = self.metrics.lock() {
            m.total_duration_secs = duration.as_secs_f64();
        }
    }
}
```

## Example 2: Soak Test Using Metrics

```rust
// tests/soak/sustained_load_24h.rs

use std::time::{Duration, Instant};
use std::sync::Arc;

// Import from your metrics module
use aspen::metrics::{MetricsCollector};

#[tokio::test]
#[ignore] // Run with: cargo test soak_sustained_load_24h -- --ignored
async fn soak_sustained_load_24h() -> anyhow::Result<()> {
    const SOAK_DURATION: Duration = Duration::from_secs(86400); // 24 hours
    const LOG_INTERVAL: u32 = 1000; // Log every 1000 ops

    let metrics = MetricsCollector::new();

    // Setup cluster (3 nodes)
    let cluster = setup_test_cluster().await?;
    let kv = cluster.create_kv_client();

    println!("Starting 24-hour soak test...");
    let test_start = Instant::now();
    let mut op_count = 0u32;

    while test_start.elapsed() < SOAK_DURATION {
        let op_start = Instant::now();

        // Execute operation
        let key = format!("soak-key-{}", op_count);
        let value = format!("soak-value-{}", op_count);

        match kv.write(WriteRequest {
            command: WriteCommand::Set { key, value },
        }).await {
            Ok(_) => {
                let latency_ms = op_start.elapsed().as_millis() as u32;
                metrics.record_success(latency_ms);
            }
            Err(e) => {
                eprintln!("Op {} failed: {}", op_count, e);
                metrics.record_failure();
            }
        }

        op_count += 1;

        // Periodic logging
        if op_count % LOG_INTERVAL == 0 {
            if let Some(snapshot) = metrics.snapshot() {
                println!(
                    "[{:.2}h] {} ops completed, {:.2} ops/sec, success: {:.2}%",
                    test_start.elapsed().as_secs_f64() / 3600.0,
                    snapshot.total_ops,
                    snapshot.throughput_ops_per_sec(),
                    snapshot.success_rate_percent()
                );
            }
        }
    }

    // Finalize and print summary
    metrics.finalize(test_start.elapsed());
    if let Some(snapshot) = metrics.snapshot() {
        snapshot.print_summary();

        // Assertions
        assert!(
            snapshot.success_rate_percent() > 99.5,
            "Expected > 99.5% success rate, got {:.2}%",
            snapshot.success_rate_percent()
        );
    }

    cluster.shutdown().await?;
    Ok(())
}
```

## Example 3: Memory-Aware Metrics (Linux)

```rust
// src/metrics.rs - Extended version with memory tracking

use std::fs;
use std::path::Path;

/// Process memory snapshot
#[derive(Debug, Clone, Copy)]
pub struct MemorySnapshot {
    pub rss_mb: u64,        // Resident set size in MB
    pub vms_mb: u64,        // Virtual memory size in MB
    pub threads: u32,       // Thread count
}

impl MemorySnapshot {
    /// Capture current process memory (Linux /proc/self/status)
    pub fn capture() -> anyhow::Result<Self> {
        let status = fs::read_to_string("/proc/self/status")?;

        let mut rss_kb = 0u64;
        let mut vms_kb = 0u64;
        let mut threads = 0u32;

        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                rss_kb = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("VmSize:") {
                vms_kb = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("Threads:") {
                threads = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }

        Ok(MemorySnapshot {
            rss_mb: rss_kb / 1024,
            vms_mb: vms_kb / 1024,
            threads,
        })
    }

    pub fn delta(&self, baseline: &MemorySnapshot) -> (i64, i64) {
        let rss_delta = self.rss_mb as i64 - baseline.rss_mb as i64;
        let vms_delta = self.vms_mb as i64 - baseline.vms_mb as i64;
        (rss_delta, vms_delta)
    }
}

/// Enhanced metrics with memory tracking
#[derive(Debug, Clone)]
pub struct SoakMetricsWithMemory {
    pub metrics: SoakMetrics,
    pub memory_start: Option<MemorySnapshot>,
    pub memory_end: Option<MemorySnapshot>,
    pub peak_memory_mb: u64,
}

impl SoakMetricsWithMemory {
    pub fn new() -> Self {
        let mem_start = MemorySnapshot::capture().ok();

        Self {
            metrics: SoakMetrics::new(),
            memory_start: mem_start,
            memory_end: None,
            peak_memory_mb: 0,
        }
    }

    pub fn finalize(&mut self, duration: Duration) {
        self.metrics.total_duration_secs = duration.as_secs_f64();
        self.memory_end = MemorySnapshot::capture().ok();
    }

    pub fn update_peak_memory(&mut self) {
        if let Ok(current) = MemorySnapshot::capture() {
            if current.rss_mb > self.peak_memory_mb {
                self.peak_memory_mb = current.rss_mb;
            }
        }
    }

    pub fn print_summary(&self) {
        self.metrics.print_summary();

        // Memory summary
        if let (Some(start), Some(end)) = (self.memory_start, self.memory_end) {
            let (rss_delta, vms_delta) = end.delta(&start);
            println!("\n===== Memory Usage =====");
            println!("Start RSS:   {} MB", start.rss_mb);
            println!("End RSS:     {} MB", end.rss_mb);
            println!("RSS Delta:   {} MB", rss_delta);
            println!("Peak RSS:    {} MB", self.peak_memory_mb);
            println!("VMS Delta:   {} MB", vms_delta);
            println!("Threads:     {}", end.threads);
        }
    }
}
```

## Example 4: Concurrent Workload with Metrics

```rust
// tests/soak/concurrent_mixed_workload.rs

#[tokio::test]
#[ignore]
async fn soak_concurrent_mixed_workload() -> anyhow::Result<()> {
    const NUM_WORKERS: usize = 100;
    const OPS_PER_WORKER: u32 = 10000;
    const READ_PERCENT: u32 = 70;

    let metrics = Arc::new(MetricsCollector::new());
    let cluster = setup_test_cluster().await?;
    let kv = Arc::new(cluster.create_kv_client());

    println!("Starting concurrent mixed workload: {} workers, {} ops each",
             NUM_WORKERS, OPS_PER_WORKER);

    let test_start = Instant::now();
    let mut handles = Vec::new();

    // Pre-populate some keys
    for i in 0..1000 {
        let key = format!("key-{}", i);
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key,
                value: "initial".to_string(),
            },
        }).await?;
    }

    // Spawn worker tasks
    for worker_id in 0..NUM_WORKERS {
        let kv_clone = Arc::clone(&kv);
        let metrics_clone = Arc::clone(&metrics);

        let handle = tokio::spawn(async move {
            let mut rng = rand::thread_rng();

            for op_num in 0..OPS_PER_WORKER {
                let op_start = Instant::now();
                let is_read = (rng.gen::<u32>() % 100) < READ_PERCENT;
                let key_id = rng.gen::<u32>() % 1000;
                let key = format!("key-{}", key_id);

                let result = if is_read {
                    kv_clone.read(ReadRequest { key }).await
                } else {
                    kv_clone.write(WriteRequest {
                        command: WriteCommand::Set {
                            key,
                            value: format!("value-{}", op_num),
                        },
                    }).await.map(|_| ())
                };

                let latency_ms = op_start.elapsed().as_millis() as u32;

                match result {
                    Ok(_) => metrics_clone.record_success(latency_ms),
                    Err(_) => metrics_clone.record_failure(),
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all workers
    for handle in handles {
        handle.await?;
    }

    let duration = test_start.elapsed();
    metrics.finalize(duration);

    // Print results
    if let Some(snapshot) = metrics.snapshot() {
        snapshot.print_summary();

        // Verify expectations
        assert!(
            snapshot.success_rate_percent() > 98.0,
            "Success rate below threshold"
        );
    }

    cluster.shutdown().await?;
    Ok(())
}
```

## Example 5: Metrics Serialization to JSON

```rust
// Serialize metrics to JSON artifact for CI analysis

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricsArtifact {
    pub test_name: String,
    pub timestamp: String,
    pub duration_secs: f64,
    pub total_ops: u64,
    pub successful_ops: u32,
    pub failed_ops: u32,
    pub success_rate_percent: f64,
    pub throughput_ops_per_sec: f64,
    pub latency_buckets: [u32; 6],
    pub memory_delta_mb: Option<i64>,
}

impl MetricsArtifact {
    pub fn from_metrics(
        test_name: &str,
        metrics: &SoakMetrics,
        memory_delta: Option<i64>,
    ) -> Self {
        Self {
            test_name: test_name.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_secs: metrics.total_duration_secs,
            total_ops: metrics.total_ops,
            successful_ops: metrics.successful_ops,
            failed_ops: metrics.failed_ops,
            success_rate_percent: metrics.success_rate_percent(),
            throughput_ops_per_sec: metrics.throughput_ops_per_sec(),
            latency_buckets: metrics.latency_buckets,
            memory_delta_mb: memory_delta,
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// Usage in test
if let Some(snapshot) = metrics.snapshot() {
    let artifact = MetricsArtifact::from_metrics("soak_24h", &snapshot, None);
    artifact.save(Path::new("artifacts/soak_metrics.json"))?;
}
```

---

## Integration Checklist

- [ ] Copy `SoakMetrics` and `MetricsCollector` to `src/metrics.rs`
- [ ] Add `pub mod metrics;` to `src/lib.rs`
- [ ] Add `#[derive(Clone)]` to structs that need cloning for threading
- [ ] Create `tests/soak/` directory for long-running tests
- [ ] Mark soak tests with `#[ignore]` and document run command
- [ ] Create `artifacts/` directory in gitignore for metric outputs
- [ ] Update CI/GitHub Actions to optionally run soak tests on schedule
- [ ] Add memory monitoring for sustained tests (Example 3)
