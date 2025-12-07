# Metrics & Observability Documentation

This directory contains comprehensive documentation about metrics, monitoring, and observability in Aspen.

## Quick Start

Start with **`METRICS_QUICK_REFERENCE.md`** for quick code snippets and common patterns.

## Documents

### 1. METRICS_OBSERVABILITY_REPORT.md
**Comprehensive analysis of existing and recommended metrics infrastructure**

- What metrics infrastructure currently exists
- OpenRaft RaftMetrics detailed breakdown
- Simulation artifact collection system
- Current memory tracking patterns
- Latency measurement approaches
- Available dependencies
- Tiger Style recommendations for soak test metrics
- Complete file references

**Read this for**: Understanding the full landscape of metrics capabilities

### 2. METRICS_IMPLEMENTATION_EXAMPLES.md
**Ready-to-use code examples**

- Example 1: Simple latency histogram (minimal, fixed-size)
- Example 2: Complete 24-hour soak test implementation
- Example 3: Memory monitoring using /proc/self/status
- Example 4: Concurrent mixed workload (100 workers)
- Example 5: JSON artifact serialization for CI
- Integration checklist for implementation

**Read this for**: Copy-paste code to implement metrics in your tests

### 3. METRICS_QUICK_REFERENCE.md
**Quick lookup guide with snippets**

- OpenRaft RaftMetrics access patterns
- Current latency measurement patterns
- Simulation artifact collection
- Tiger Style constraints
- Available dependencies summary
- Memory tracking code snippet
- Test command reference
- Next steps checklist

**Read this for**: Daily reference while implementing metrics

## Key Concepts

### OpenRaft RaftMetrics
Comprehensive built-in metrics for Raft consensus including node state, log state, replication progress, and leadership information.

Location: `/home/brittonr/git/aspen/openraft/openraft/src/metrics/`

### Simulation Artifacts
Automatic collection and persistence of test metadata (seed, duration, events, metrics) for debugging and CI analysis.

Location: `/home/brittonr/git/aspen/src/simulation.rs`

### Tiger Style Metrics
Fixed-size data structures, explicit types, O(1) operations, and bounded resource usage for production reliability.

Pattern: Use `[u32; N]` arrays for histograms, not `Vec`. Use `Arc<Mutex>` for thread-safe access.

### Load Tests
Three existing load test examples showing baseline measurement patterns:
- `test_sustained_write_load.rs` - Sequential write throughput
- `test_concurrent_read_load.rs` - Concurrent read throughput
- `test_mixed_workload.rs` - 70% read, 30% write mixed workload

## Implementation Roadmap

### Phase 1: Understand Current Infrastructure
1. Read `METRICS_OBSERVABILITY_REPORT.md` sections 1-4
2. Examine existing load tests in `tests/load/`
3. Review OpenRaft metrics in `openraft/openraft/src/metrics/`

### Phase 2: Create Metrics Module
1. Create `src/metrics.rs` using `METRICS_IMPLEMENTATION_EXAMPLES.md` Example 1
2. Implement `SoakMetrics` struct with fixed-size histogram
3. Implement `MetricsCollector` wrapper for thread-safe access
4. Add to `src/lib.rs`

### Phase 3: Extend Load Tests
1. Add metrics collection to existing load tests
2. Use `MetricsCollector::new()` to create collector
3. Call `record_success()` and `record_failure()` in operation loop
4. Print metrics summary at end

### Phase 4: Create Soak Tests
1. Create `tests/soak/` directory
2. Implement first soak test using `METRICS_IMPLEMENTATION_EXAMPLES.md` Example 2
3. Add memory monitoring using Example 3
4. Mark tests with `#[ignore]` and document run instructions

### Phase 5: Persist & Analyze
1. Serialize metrics to JSON using Example 5
2. Use `SimulationArtifactBuilder` to persist to `docs/simulations/`
3. Set up CI schedule to run soak tests weekly
4. Create dashboard or analysis script for historical metrics

## Tiger Style Principles (Key for Soak Tests)

1. **Fixed-Size Buffers**: Use arrays `[u32; N]` for latency histograms, never Vec
2. **Explicit Types**: u64 for totals (prevent overflow), u32 for counts
3. **Bounded Computation**: O(1) insertion, no allocations in hot path
4. **Fail-Fast**: Early returns on errors, explicit error handling
5. **Resource Limits**: Fixed pool sizes, batch limits, connection limits
6. **Clear Measurements**: Duration, throughput, success rate, latency percentiles
7. **Thread-Safe**: Arc<Mutex> for sharing metrics across worker tasks

## Metrics to Collect

### Minimum Set
- Total operations executed
- Successful operations
- Failed operations
- Total duration
- Throughput (ops/sec)
- Average latency

### Recommended Set
- Min/max/average latency
- Latency percentiles (p50, p99, p99.9)
- Success rate (%)
- Error types and counts
- Memory usage (RSS delta)
- Thread count

### Advanced
- Per-operation latency histogram (6 buckets)
- Tail latencies
- CPU time usage
- Disk I/O patterns
- GC pause times (if applicable)

## Common Patterns

### Measuring Latency
```rust
let start = Instant::now();
operation().await?;
let latency_ms = start.elapsed().as_millis() as u32;
metrics.record_success(latency_ms);
```

### Thread-Safe Recording
```rust
let metrics = Arc::new(MetricsCollector::new());
for worker_id in 0..NUM_WORKERS {
    let metrics_clone = Arc::clone(&metrics);
    tokio::spawn(async move {
        metrics_clone.record_success(latency_ms);
    });
}
```

### Progress Reporting
```rust
if op_count % 1000 == 0 {
    let snapshot = metrics.snapshot().unwrap();
    println!("Progress: {:.2}% done, {:.2} ops/sec",
             percent, snapshot.throughput_ops_per_sec());
}
```

### Artifact Persistence
```rust
let artifact = SimulationArtifactBuilder::new("soak_test", seed)
    .with_metrics(format!("{:#?}", final_metrics))
    .with_duration_ms(duration.as_millis() as u64)
    .build();
artifact.persist("docs/simulations")?;
```

## Existing Infrastructure Summary

| Component | Location | Status | Notes |
|-----------|----------|--------|-------|
| RaftMetrics | openraft/src/metrics/ | Active | Comprehensive consensus metrics |
| Simulation artifacts | src/simulation.rs | Active | Event & metric persistence |
| Load tests | tests/load/ | Active | 3 existing test examples |
| Tiger Style limits | src/raft/storage_sqlite.rs | Active | Fixed batch/pool sizes |
| tracing | Cargo.toml | Active | Structured logging available |
| iroh-metrics | Cargo.toml | Available | Not currently used |

## Files by Purpose

### Understanding Metrics
- `METRICS_OBSERVABILITY_REPORT.md` - Comprehensive analysis
- `METRICS_QUICK_REFERENCE.md` - Quick lookup

### Implementing Metrics
- `METRICS_IMPLEMENTATION_EXAMPLES.md` - Code examples
- `tests/load/*.rs` - Reference implementations

### OpenRaft Metrics
- `openraft/openraft/src/metrics/raft_metrics.rs` - Metrics definition
- `openraft/openraft/src/metrics/wait.rs` - Metric waiting API

### Simulation Artifacts
- `src/simulation.rs` - Artifact builder and persistence

### Storage Constraints
- `src/raft/storage_sqlite.rs` - Fixed limits examples

## Glossary

**Throughput**: Operations per second (ops/sec)

**Latency**: Time taken for single operation (ms)

**Success Rate**: Percentage of operations that completed successfully

**Histogram**: Distribution of latencies across fixed buckets

**Artifact**: Persisted test metadata (seed, duration, events, metrics)

**Tiger Style**: Design principles emphasizing safety, performance, and DX

**Hot Path**: Frequently executed code section where performance matters

**O(1) Operation**: Constant-time operation regardless of data size

## Questions?

Refer to:
1. **"How do I..."** → `METRICS_QUICK_REFERENCE.md`
2. **"Show me example code"** → `METRICS_IMPLEMENTATION_EXAMPLES.md`
3. **"What exists already?"** → `METRICS_OBSERVABILITY_REPORT.md`
4. **"I want to understand it deeply"** → All three documents

---

Last updated: 2025-12-06

This documentation covers:
- Existing metrics infrastructure (OpenRaft, simulation artifacts, load tests)
- Memory tracking patterns (Arc, Mutex, fixed buffers)
- Latency measurement approaches
- Tiger Style metrics recommendations
- Ready-to-use code examples
- Implementation roadmap
