# Metrics & Observability Quick Reference

## OpenRaft RaftMetrics

Access Raft node metrics via watch channel:

```rust
let metrics_rx = raft.metrics();
let current = metrics_rx.borrow_watched();

// Check node state
println!("State: {:?}", current.state);           // Leader/Follower/Candidate
println!("Leader: {:?}", current.current_leader);
println!("Term: {}", current.current_term);

// Log state
println!("Last applied: {:?}", current.last_applied);
println!("Last log index: {:?}", current.last_log_index);

// Wait for specific conditions
raft.wait(None)
    .state(State::Leader, "wait for leadership")
    .await?;

raft.wait(Some(Duration::from_secs(10)))
    .log(Some(100), "wait for log-100 applied")
    .await?;
```

**Files**:
- `/home/brittonr/git/aspen/openraft/openraft/src/metrics/raft_metrics.rs`
- `/home/brittonr/git/aspen/openraft/openraft/src/metrics/wait.rs`

---

## Current Latency Measurement Pattern

From existing load tests:

```rust
// Measure entire operation sequence
let start = Instant::now();
for i in 0..NUM_OPS {
    kv.write(request).await?;
}
let duration = start.elapsed();

// Calculate metrics
let throughput = NUM_OPS as f64 / duration.as_secs_f64();
let avg_latency_ms = duration.as_millis() as f64 / NUM_OPS as f64;

println!("Throughput: {:.2} ops/sec", throughput);
println!("Avg latency: {:.2} ms/op", avg_latency_ms);
```

**Files**:
- `/home/brittonr/git/aspen/tests/load/test_sustained_write_load.rs`
- `/home/brittonr/git/aspen/tests/load/test_concurrent_read_load.rs`
- `/home/brittonr/git/aspen/tests/load/test_mixed_workload.rs`

---

## Simulation Artifact Collection

Capture test metrics for CI analysis:

```rust
use aspen::simulation::{SimulationArtifactBuilder, SimulationStatus};
use std::time::Instant;

let start = Instant::now();

// ... run test ...

let duration_ms = start.elapsed().as_millis() as u64;

let artifact = SimulationArtifactBuilder::new("my_test", seed)
    .start()
    .add_event("initialization_complete".into())
    .add_event("test_execution_started".into())
    .with_metrics("test-specific metrics JSON".into())
    .with_duration_ms(duration_ms)
    .build();

artifact.persist("docs/simulations")?;
```

**File**: `/home/brittonr/git/aspen/src/simulation.rs`

---

## Tiger Style Constraints (Fixed Limits in Storage)

From `src/raft/storage_sqlite.rs`:

```rust
// Fixed operation limits - prevent unbounded resource usage
const MAX_BATCH_SIZE: u32 = 1000;         // Max entries per batch
const MAX_SETMULTI_KEYS: u32 = 100;       // Max keys per operation
const DEFAULT_READ_POOL_SIZE: u32 = 10;   // Max concurrent readers

// Pre-allocate to avoid re-allocation
let mut operations = Vec::with_capacity(TOTAL_OPS);
```

---

## Dependencies Available

**Already in Cargo.toml**:
- `tracing` - Structured logging
- `tracing-subscriber` - Log configuration
- `iroh-metrics` - Unused but available
- `serde_json` - JSON serialization

**Not in Cargo.toml**:
- prometheus
- statsd
- opentelemetry

---

## For Soak Tests: Recommended Pattern

```rust
// 1. Define fixed-size metrics struct
#[derive(Clone)]
pub struct SoakMetrics {
    pub total_ops: u64,
    pub successful: u32,
    pub failed: u32,
    pub latency_buckets_ms: [u32; 6],  // Fixed size, not Vec
}

// 2. Thread-safe collector
pub struct MetricsCollector {
    metrics: Arc<Mutex<SoakMetrics>>,
}

// 3. Record operations
metrics.record_success(latency_ms);
metrics.record_failure();

// 4. Log progress periodically
if op_count % 1000 == 0 {
    let snapshot = metrics.snapshot();
    println!("Progress: {:.2}% complete, {:.2} ops/sec",
             percent, throughput);
}

// 5. Persist results
artifact.persist("docs/simulations")?;
```

**Examples**: `/home/brittonr/git/aspen/METRICS_IMPLEMENTATION_EXAMPLES.md`

---

## Key Metrics to Collect

1. **Throughput**: ops/sec
2. **Latency**: Min, max, average, percentiles (p50, p99, p99.9)
3. **Success Rate**: % of ops that succeeded
4. **Duration**: Total test time
5. **Memory**: RSS growth if applicable
6. **Errors**: Error count and types

---

## Memory Tracking (Linux)

```rust
// Capture current process memory
use std::fs;

let status = fs::read_to_string("/proc/self/status")?;
for line in status.lines() {
    if line.starts_with("VmRSS:") {
        let kb: u64 = line.split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        println!("Memory: {} MB", kb / 1024);
    }
}
```

See `METRICS_IMPLEMENTATION_EXAMPLES.md` Example 3 for full implementation.

---

## Files Summary

| File | Purpose |
|------|---------|
| `openraft/openraft/src/metrics/` | OpenRaft RaftMetrics |
| `src/simulation.rs` | Artifact collection and persistence |
| `tests/load/` | Load test measurement examples |
| `src/raft/storage_sqlite.rs` | Tiger Style fixed limits |
| `benches/` | Performance benchmarks |

---

## Running Tests

```bash
# Standard load tests
cargo test test_sustained_write_100_ops

# Large load tests (ignored by default)
cargo test test_sustained_write_1000_ops -- --ignored

# Soak tests (if implemented)
cargo test soak_sustained_load_24h -- --ignored

# With logging
RUST_LOG=debug cargo test --lib
```

---

## Next Steps for Implementation

1. Add `src/metrics.rs` with `SoakMetrics` struct
2. Extend load tests to use metrics collector
3. Create `tests/soak/` directory for long-running tests
4. Add memory monitoring for multi-hour tests
5. Set up CI schedule to run soak tests weekly
6. Export metrics to `artifacts/` directory for analysis

See `METRICS_IMPLEMENTATION_EXAMPLES.md` for complete code.
