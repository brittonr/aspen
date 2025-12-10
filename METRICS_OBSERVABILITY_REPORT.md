# Aspen Metrics, Monitoring & Observability Report

## Executive Summary

The Aspen codebase has **limited existing metrics infrastructure** but provides a solid foundation through:

- **OpenRaft's RaftMetrics** - Comprehensive Raft consensus metrics
- **Basic load test measurements** - Duration and throughput tracking
- **Simulation artifact collection** - Event traces and runtime metrics
- **Tiger Style constraints** - Fixed buffers, explicit types, bounded resource usage

There are **no Prometheus, StatsD, or OpenTelemetry integrations** currently, but `iroh-metrics` is available in dependencies for future integration.

---

## 1. Existing Metrics Infrastructure

### 1.1 OpenRaft RaftMetrics (Vendored)

**Location**: `/home/brittonr/git/aspen/openraft/openraft/src/metrics/`

**Available Metrics**:

- **Node State**: `id`, `state` (Leader/Follower/Candidate), `current_leader`, `running_state`
- **Log State**: `last_log_index`, `last_applied`, `snapshot`, `purged`
- **Voting State**: `current_term`, `vote`
- **Leader Metrics** (when leader):
  - `heartbeat`: BTreeMap<NodeId, Option<Instant>> - Last ack times for detecting offline nodes
  - `replication`: BTreeMap<NodeId, Option<LogId>> - Matched log index per follower
  - `last_quorum_acked`: Highest log index acknowledged by quorum
- **Cluster Config**: `membership_config`

**Access Pattern**:

```rust
let metrics_rx = raft.metrics();
let metrics = metrics_rx.borrow_watched();
println!("Node state: {:?}", metrics.state);

// Wait for conditions
raft.wait(None).state(State::Leader, "become leader").await?;
raft.wait(Some(timeout)).log(Some(10), "log-10 applied").await?;
```

**Files**:

- `raft_metrics.rs` - RaftMetrics struct definition
- `metric.rs` - Individual metric types
- `wait.rs` - Metric wait conditions API
- `metric_display.rs` - Display formatting

**Test Coverage**:

- `openraft/tests/tests/metrics/t10_server_metrics_and_data_metrics.rs`
- `openraft/tests/tests/metrics/t20_metrics_state_machine_consistency.rs`
- `openraft/tests/tests/metrics/t30_leader_metrics.rs`
- `openraft/tests/tests/metrics/t40_metrics_wait.rs`

### 1.2 Simulation Artifact Collection

**Location**: `/home/brittonr/git/aspen/src/simulation.rs`

**What It Captures**:

```rust
pub struct SimulationArtifact {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub seed: u64,                          // Deterministic seed
    pub test_name: String,
    pub events: Vec<String>,                // Event trace
    pub metrics: String,                    // Metrics snapshot (JSON serialized)
    pub status: SimulationStatus,           // Passed/Failed
    pub error: Option<String>,
    pub duration_ms: u64,                   // Total test duration
}
```

**Artifact Persistence**:

- Written to `docs/simulations/{run_id}.json`
- Gitignored to avoid bloating repo
- Useful for CI debugging and test analysis

**Usage Pattern**:

```rust
let start = Instant::now();
let artifact = SimulationArtifactBuilder::new("test_name", seed)
    .start()
    .with_metrics("metrics snapshot as string".into())
    .with_duration_ms(start.elapsed().as_millis() as u64)
    .build();
artifact.persist("docs/simulations")?;
```

---

## 2. Memory Tracking Patterns

### 2.1 Current Approaches

**No dedicated memory tracking library** is used in the codebase. Memory management relies on:

1. **Tiger Style Fixed Buffers** - Bounded data structures:

   ```rust
   // From src/raft/storage_sqlite.rs
   const MAX_BATCH_SIZE: u32 = 1000;              // Max entries per batch
   const MAX_SETMULTI_KEYS: u32 = 100;            // Max keys per op
   const DEFAULT_READ_POOL_SIZE: u32 = 10;        // Max concurrent readers
   ```

2. **Arc/Mutex for Reference Counting**:

   ```rust
   use std::sync::Arc;
   use std::sync::Mutex;
   use std::collections::BTreeMap;
   ```

3. **Vec Capacity Pre-allocation** (in load tests):

   ```rust
   // From tests/load/test_mixed_workload.rs
   let mut operations = Vec::with_capacity(TOTAL_OPS);  // Pre-allocate
   ```

4. **Atomic Counters** (SQLite implementation):

   ```rust
   use std::sync::atomic::{AtomicU64, Ordering};
   let counter = Arc::new(AtomicU64::new(0));
   ```

### 2.2 Memory Accounting in Storage

**SQLite Storage** (`src/raft/storage_sqlite.rs`):

- Uses `r2d2` connection pool with fixed size
- Bounded transaction batch sizes
- Automatic cleanup via Drop/RAII

**Redb Storage** (legacy):

- In-memory B-tree based
- No explicit memory limits (deprecated)

---

## 3. Latency Measurement Approaches

### 3.1 Load Test Latency Patterns

**Location**: `tests/load/`

#### Pattern 1: Single-Operation Latency

```rust
// From test_sustained_write_load.rs
let start = Instant::now();
for i in 0..TOTAL_WRITES {
    kv.write(write_req).await?;
}
let duration = start.elapsed();
let avg_latency_ms = duration.as_millis() as f64 / TOTAL_WRITES as f64;
println!("Average latency: {:.2} ms/op", avg_latency_ms);
```

#### Pattern 2: Throughput Calculation

```rust
let duration_secs = duration.as_secs_f64();
let throughput = successful_writes as f64 / duration_secs;
println!("Throughput: {:.2} ops/sec", throughput);
```

#### Pattern 3: Per-Reader Latency (Concurrent Reads)

```rust
// From test_concurrent_read_load.rs
let start = Instant::now();
while let Some(result) = join_set.join_next().await {
    total_successful += result?;
}
let duration = start.elapsed();
let latency = duration.as_millis() as f64 / total_reads as f64;
println!("Average latency: {:.2} ms/read", latency);
```

### 3.2 Simulation Duration Tracking

```rust
let start = Instant::now();
// ... run simulation ...
let duration_ms = start.elapsed().as_millis() as u64;

artifact.with_duration_ms(duration_ms)
```

### 3.3 Test-Specific Latency

**Failure Detection** (`src/raft/node_failure_detection.rs`):

```rust
pub fn get_unreachable_duration(&self, node_id: u64) -> Option<Duration>
```

**Snapshot Building** (`tests/router_snapshot_t10_build_snapshot.rs`):

```rust
let start = Instant::now();
while !snapshot_complete && start.elapsed() < Duration::from_secs(10) {
    // poll for completion
}
```

---

## 4. Observability Dependencies

### 4.1 Currently Used

- **`tracing`** (0.1) - Structured logging
- **`tracing-subscriber`** (0.3) - Log filtering and output
- **`iroh-metrics`** (0.37) - Available but unused
- **`serde_json`** (1.0) - Metrics serialization

### 4.2 Not Currently Used (But Available)

- **`prometheus`** - Not in dependencies
- **`statsd`** - Not in dependencies
- **`opentelemetry`** - Not in dependencies

---

## 5. Recommendations for Soak Test Metrics Collection

### 5.1 Tiger Style Compliant Metrics Struct

Following Tiger Style principles (fixed sizes, explicit types, bounded resources):

```rust
use std::time::Duration;
use std::sync::Arc;
use std::sync::Mutex;

/// Tiger Style: Fixed-size metrics buffer with bounded storage
pub struct SoakTestMetrics {
    /// Operation count (u64 to prevent overflow with large counts)
    pub total_ops: u64,

    /// Success/failure counts (u32 sufficient for most scenarios)
    pub successful_ops: u32,
    pub failed_ops: u32,

    /// Fixed-size latency histogram (10 buckets: 1ms, 10ms, 100ms, etc.)
    /// Tiger Style: Fixed limits prevent unbounded memory allocation
    pub latency_buckets_ms: [u32; 10],  // [0-1ms, 1-10ms, 10-100ms, 100ms-1s, ...]

    /// Throughput snapshot (ops/sec)
    pub throughput_ops_per_sec: f64,

    /// Total duration
    pub duration: Duration,

    /// Resource utilization (if available)
    pub memory_used_bytes: Option<u64>,
    pub cpu_time_ms: Option<u64>,
}

impl SoakTestMetrics {
    pub fn new(duration: Duration) -> Self {
        Self {
            total_ops: 0,
            successful_ops: 0,
            failed_ops: 0,
            latency_buckets_ms: [0u32; 10],
            throughput_ops_per_sec: 0.0,
            duration,
            memory_used_bytes: None,
            cpu_time_ms: None,
        }
    }

    /// Record operation latency into appropriate bucket
    /// Tiger Style: O(1) latency, bounded computation
    pub fn record_latency_ms(&mut self, latency_ms: u32) {
        let bucket_index = match latency_ms {
            0..=1 => 0,
            2..=10 => 1,
            11..=100 => 2,
            101..=1000 => 3,
            1001..=10000 => 4,
            _ => 9,  // Cap at 10000+ ms bucket
        };

        // Prevent bucket overflow
        if self.latency_buckets_ms[bucket_index] < u32::MAX {
            self.latency_buckets_ms[bucket_index] += 1;
        }
    }

    pub fn finalize(&mut self, duration: Duration) {
        self.duration = duration;
        if duration.as_secs_f64() > 0.0 {
            self.throughput_ops_per_sec =
                self.total_ops as f64 / duration.as_secs_f64();
        }
    }
}

/// Thread-safe metrics collector
pub struct SoakTestMetricsCollector {
    metrics: Arc<Mutex<SoakTestMetrics>>,
}

impl SoakTestMetricsCollector {
    pub fn new(duration: Duration) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(SoakTestMetrics::new(duration))),
        }
    }

    pub fn record_success(&self, latency_ms: u32) {
        if let Ok(mut m) = self.metrics.lock() {
            m.total_ops += 1;
            m.successful_ops = m.successful_ops.saturating_add(1);
            m.record_latency_ms(latency_ms);
        }
    }

    pub fn record_failure(&self) {
        if let Ok(mut m) = self.metrics.lock() {
            m.total_ops += 1;
            m.failed_ops = m.failed_ops.saturating_add(1);
        }
    }

    pub fn snapshot(&self) -> Option<SoakTestMetrics> {
        self.metrics.lock().ok().map(|m| m.clone())
    }
}
```

### 5.2 Usage in Soak Tests

```rust
#[tokio::test]
#[ignore] // Soak test, run with --ignored
async fn test_soak_sustained_load() -> anyhow::Result<()> {
    const DURATION_SECS: u64 = 3600;  // 1 hour
    const OPS_PER_BATCH: u32 = 100;

    let duration = Duration::from_secs(DURATION_SECS);
    let metrics = SoakTestMetricsCollector::new(duration);

    let start = Instant::now();
    let mut batch_count = 0u32;

    while start.elapsed() < duration {
        for _ in 0..OPS_PER_BATCH {
            let op_start = Instant::now();
            match kv.write(write_req).await {
                Ok(_) => {
                    let latency_ms = op_start.elapsed().as_millis() as u32;
                    metrics.record_success(latency_ms);
                }
                Err(_) => metrics.record_failure(),
            }
        }

        batch_count += 1;
        if batch_count % 10 == 0 {  // Log every 1000 ops
            let snapshot = metrics.snapshot().unwrap();
            println!(
                "Progress: {} ops, {:.2} ops/sec, success rate: {:.2}%",
                snapshot.total_ops,
                snapshot.throughput_ops_per_sec,
                (snapshot.successful_ops as f64 / snapshot.total_ops as f64) * 100.0
            );
        }
    }

    let final_metrics = metrics.snapshot().unwrap();

    // Log histogram
    println!("\nLatency Distribution (ms):");
    println!("  0-1ms:       {} ops", final_metrics.latency_buckets_ms[0]);
    println!("  1-10ms:      {} ops", final_metrics.latency_buckets_ms[1]);
    println!("  10-100ms:    {} ops", final_metrics.latency_buckets_ms[2]);
    println!("  100-1000ms:  {} ops", final_metrics.latency_buckets_ms[3]);
    println!("  1000+ ms:    {} ops", final_metrics.latency_buckets_ms[9]);

    // Persist to simulation artifact
    let artifact = SimulationArtifactBuilder::new("soak_test_1hour", seed)
        .with_metrics(format!("{:#?}", final_metrics))
        .with_duration_ms(final_metrics.duration.as_millis() as u64)
        .build();

    artifact.persist("docs/simulations")?;

    Ok(())
}
```

### 5.3 Memory Monitoring Pattern (Linux-specific)

```rust
use std::fs;

/// Tiger Style: Fixed fields, no allocations after initialization
pub struct ProcessMetrics {
    pub rss_bytes: u64,           // Resident set size
    pub vms_bytes: u64,           // Virtual memory size
    pub threads: u32,             // Thread count
}

impl ProcessMetrics {
    /// Read current process memory from /proc/self/status
    /// Tiger Style: Fail-fast, explicit error handling
    pub fn capture() -> anyhow::Result<Self> {
        let status = fs::read_to_string("/proc/self/status")?;

        let mut rss_bytes = 0u64;
        let mut vms_bytes = 0u64;
        let mut threads = 0u32;

        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                rss_bytes = kb * 1024;
            } else if line.starts_with("VmSize:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                vms_bytes = kb * 1024;
            } else if line.starts_with("Threads:") {
                threads = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }

        Ok(ProcessMetrics {
            rss_bytes,
            vms_bytes,
            threads,
        })
    }
}

// Usage in soak test
let mem_start = ProcessMetrics::capture()?;
// ... run soak test ...
let mem_end = ProcessMetrics::capture()?;

println!(
    "Memory growth: {} MB (RSS) / {} MB (VMS)",
    (mem_end.rss_bytes - mem_start.rss_bytes) / 1_000_000,
    (mem_end.vms_bytes - mem_start.vms_bytes) / 1_000_000
);
```

---

## 6. Files Reference

### Core Metrics

- `/home/brittonr/git/aspen/openraft/openraft/src/metrics/mod.rs`
- `/home/brittonr/git/aspen/openraft/openraft/src/metrics/raft_metrics.rs`
- `/home/brittonr/git/aspen/src/simulation.rs`

### Load Tests (Current Measurement Examples)

- `/home/brittonr/git/aspen/tests/load/test_sustained_write_load.rs`
- `/home/brittonr/git/aspen/tests/load/test_concurrent_read_load.rs`
- `/home/brittonr/git/aspen/tests/load/test_mixed_workload.rs`

### Storage (Fixed Limits)

- `/home/brittonr/git/aspen/src/raft/storage_sqlite.rs` (lines 18-28: MAX_BATCH_SIZE, etc.)

### Benchmarks

- `/home/brittonr/git/aspen/benches/storage_comparison.rs`
- `/home/brittonr/git/aspen/benches/storage_read_concurrency.rs`

---

## 7. Summary: Tiger Style Metrics Collection

**Key Principles for Soak Tests**:

1. **Fixed-Size Buffers**: Use arrays `[u32; N]` for latency histograms, not Vec
2. **Explicit Types**: Use u32 for counts (sufficient), u64 for totals (prevent overflow)
3. **Bounded Computation**: O(1) operations, no dynamic allocations in hot path
4. **Fail-Fast**: Return early on errors, explicit error handling
5. **Resource Limits**: Set upper bounds on memory (connection pools, batch sizes)
6. **Clear Measurements**: Track duration, throughput, success rate, latency percentiles
7. **Atomic Operations**: Use Arc<Mutex> for thread-safe counters in concurrent tests

**Recommended Integration**:

- Add optional metrics collection to load tests using pattern above
- Persist metrics to simulation artifacts for historical analysis
- Use OpenRaft RaftMetrics for cluster-level observability
- Consider `iroh-metrics` integration for distributed metrics if needed in future
