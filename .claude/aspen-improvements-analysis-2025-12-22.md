# Aspen Gap Analysis and Improvement Recommendations

**Analysis Date**: 2025-12-22
**Based On**: comparison-foundationdb-tikv-etcd-cockroachdb-2025-12-22.md
**Aspen Version**: v3 branch (commit b4b73c2a)

---

## Executive Summary

After a thorough analysis of Aspen against FoundationDB, TiKV, etcd, and CockroachDB, this document identifies **14 key gaps** organized into Critical, High, Medium, and Low priority. The analysis draws from:

1. The comparison document's feature matrices
2. Deep exploration of the Aspen codebase
3. Best practices from competitor implementations
4. Industry-standard testing and reliability patterns

### Priority Summary

| Priority | Count | Effort Range | Impact |
|----------|-------|--------------|--------|
| Critical | 3 | High | Production readiness |
| High | 4 | Medium-High | Competitive parity |
| Medium | 4 | Medium | Feature completeness |
| Low | 3 | Low-Medium | Nice-to-have |

---

## Critical Gaps (Must Fix for Production)

### 1. No Benchmark Suite

**Current State**: Zero benchmarking infrastructure. No way to measure or track performance.

**Why It Matters**:

- The comparison doc shows "TBD" for Aspen throughput/latency
- Cannot validate performance claims or detect regressions
- Competitors have extensive benchmarks (FDB: 720K writes/s documented, YCSB for others)

**Evidence from codebase**:

```
$ grep -r "benchmark\|criterion" src/
# Only found in vendored openraft/cluster_benchmark - not Aspen-specific
```

**Recommendation**:

1. Create `benches/` directory with Criterion benchmarks
2. Implement YCSB-like workload patterns (Read, Update, Scan, Insert)
3. Add single-node and multi-node benchmark suites
4. Integrate into CI with performance tracking

**Reference Implementations**:

- [TiKV go-ycsb](https://github.com/pingcap/go-ycsb)
- [FoundationDB Performance Testing](https://apple.github.io/foundationdb/performance-testing.html)
- [etcd Benchmark Tool](https://etcd.io/docs/v3.5/op-guide/performance/#benchmarks)

---

### 2. Limited BUGGIFY/Fault Injection Coverage

**Current State**: Basic `BuggifyConfig` structure exists in `madsim_tester.rs` but:

- Only 5 fault types defined (NetworkDelay, NetworkDrop, ProcessCrash, DiskError, ClockSkew)
- Not integrated throughout production code
- No BUGGIFY macros in critical paths

**Why It Matters**:

- FoundationDB's BUGGIFY is in the **production binary**, not just tests
- FDB's simulation runs ~1 trillion CPU-hours equivalent
- Results: Only 1-2 customer-reported bugs ever

**Evidence from codebase**:

```rust
// src/testing/madsim_tester.rs:355-375
pub enum BuggifyFault {
    NetworkDelay,
    NetworkDrop,
    ProcessCrash,
    DiskError,
    ClockSkew,
}
```

No BUGGIFY calls found in:

- `src/raft/` (consensus layer)
- `src/cluster/` (coordination layer)
- `src/api/` (API layer)

**Recommendation**:

1. Create a `buggify!()` macro that compiles to no-op in release builds
2. Add BUGGIFY points to critical code paths:
   - Raft message handling (delay, drop, corrupt)
   - Storage operations (slow writes, read errors)
   - Network connections (timeouts, resets)
   - Transaction processing (delay commits)
3. Add runtime flag to enable BUGGIFY in test builds
4. Track BUGGIFY triggers in `SimulationMetrics`

**Reference**:

- [BUGGIFY Deep Dive](https://transactional.blog/simulation/buggify)
- [FoundationDB Simulation Testing](https://apple.github.io/foundationdb/testing.html)

---

### 3. No Jepsen or Linearizability Verification

**Current State**: No formal consistency verification. madsim tests exist but don't verify linearizability.

**Why It Matters**:

- All competitors (TiKV, etcd, CRDB) have been Jepsen-tested
- FoundationDB is so confident they challenged Kyle Kingsbury to test it
- Without verification, consistency claims are unproven

**Evidence from codebase**:

```
$ grep -r "lineariz\|Jepsen\|consistency.*check" tests/
# No results
```

**Recommendation**:

1. Implement a linearizability checker using [Porcupine](https://github.com/anishathalye/porcupine) or [Elle](https://github.com/jepsen-io/elle)
2. Add history recording to madsim tests
3. Verify operations against sequential specification
4. Document consistency guarantees with test evidence

---

## High Priority Gaps (Competitive Parity)

### 4. No Load-Based Sharding/Hotspot Detection

**Current State**: Consistent hash sharding is implemented (Dec 2025) but:

- Static shard assignment only
- No load monitoring per shard
- No automatic rebalancing
- No hotspot detection

**Why It Matters**:

- TiKV splits regions automatically based on QPS/size
- CockroachDB rebalances ranges based on load
- Without this, one hot key can saturate a shard

**Evidence from codebase**:

```rust
// src/cluster/sharding.rs - Uses consistent hashing
pub fn key_to_shard_id(key: &str, num_shards: u16) -> ShardId {
    // Hash-based routing, no load awareness
}
```

**Recommendation**:

1. Add per-shard metrics collection (QPS, latency, size)
2. Implement hotspot detection algorithm
3. Add shard split/merge operations
4. Create rebalancing scheduler with configurable thresholds

**Reference**:

- [TiKV Region Scheduling](https://tikv.org/deep-dive/key-manager/region-scheduling/)
- [CockroachDB Rebalancing](https://www.cockroachlabs.com/docs/stable/architecture/replication-layer)

---

### 5. Watch API Lacks Guaranteed Delivery Semantics

**Current State**: `LogSubscriberProtocolHandler` exists with:

- Prefix filtering
- Buffered delivery
- But: "subscribers that lag too far behind will miss entries"

**Why It Matters**:

- etcd Watch has clear guarantees about ordering and delivery
- Missing events breaks applications relying on watch
- etcd provides compact revision tracking to detect gaps

**Evidence from codebase**:

```rust
// src/raft/storage_sqlite.rs:693
// This is intentional: subscribers that lag too far behind will miss entries
```

**Recommendation**:

1. Add compaction revision tracking
2. Return `ErrCompacted` when client requests entries before compaction
3. Implement watch resume from revision
4. Add per-subscription backpressure instead of dropping
5. Document ordering guarantees

**Reference**:

- [etcd Watch API Guarantees](https://etcd.io/docs/v3.5/learning/api_guarantees/#watch-apis)

---

### 6. No Follower Reads / Leaseholder Optimization

**Current State**: All reads go through leader via ReadIndex protocol.

**Why It Matters**:

- CockroachDB follower reads reduce cross-region latency dramatically
- For geo-distributed deployments, this is essential
- ReadIndex adds latency even for stale-tolerant reads

**Evidence from codebase**:

```rust
// src/api/mod.rs:1306-1319
pub enum SqlConsistency {
    Linearizable,  // Default - uses ReadIndex
    Stale,         // Local read, no guarantee
}
```

Missing: `AS OF SYSTEM TIME` semantics for bounded staleness.

**Recommendation**:

1. Implement closed timestamp tracking (like CockroachDB)
2. Add `ClosedTimestampRead` consistency level
3. Allow follower nodes to serve reads at closed timestamp
4. Add leaseholder optimization (skip consensus for lease holder reads)

**Reference**:

- [CockroachDB Follower Reads](https://www.cockroachlabs.com/blog/follower-reads-stale-data/)
- [CockroachDB Closed Timestamps RFC](https://github.com/cockroachdb/cockroach/blob/master/docs/RFCS/20181227_follower_reads_implementation.md)

---

### 7. No Coprocessor/Push-Down Computation Framework

**Current State**: All computation happens at the client or leader node.

**Why It Matters**:

- TiKV's coprocessor pushes computation to data location
- Reduces network traffic for aggregations by orders of magnitude
- Essential for analytical workloads

**Evidence from codebase**:

```
$ grep -r "coprocessor\|push.down\|aggregate" src/
# No results
```

**Recommendation**:

1. Design coprocessor protocol for Aspen
2. Start with simple aggregations (COUNT, SUM, MIN, MAX)
3. Consider Arrow-based columnar format for results
4. Add SQL function push-down for the SQLite backend

**Reference**:

- [TiKV Coprocessor Guide](https://tikv.github.io/tikv-dev-guide/understanding-tikv/coprocessor/intro.html)

---

## Medium Priority Gaps (Feature Completeness)

### 8. No Client Library for Other Languages

**Current State**: Rust-only. No bindings for Go, Python, Java, TypeScript.

**Why It Matters**:

- All competitors have multi-language support
- Limits adoption to Rust-only projects
- etcd's Go client is ubiquitous (every Kubernetes cluster)

**Recommendation**:

1. Define stable wire protocol using postcard/bincode
2. Create C FFI bindings as base layer
3. Generate Go, Python, TypeScript clients from FFI
4. Consider gRPC as alternative (industry standard, auto-generated clients)

---

### 9. No Geo-Partitioning

**Current State**: Multi-shard support exists but no region awareness.

**Why It Matters**:

- CockroachDB's geo-partitioning is a key differentiator
- Required for data residency compliance (GDPR, etc.)
- Essential for global deployments

**Recommendation**:

1. Add region labels to nodes
2. Implement region-aware shard placement
3. Add locality-first routing for reads
4. Consider regional leader preference

---

### 10. Transaction Duration Unbounded

**Current State**: No transaction timeout (comparison shows "No limit").

**Why It Matters**:

- Long-running transactions can block compaction
- FoundationDB's 5-second limit forces fast transactions
- Unbounded transactions can cause cascading failures

**Recommendation**:

1. Add configurable transaction timeout (default: 30s)
2. Implement transaction age tracking
3. Auto-abort transactions exceeding timeout
4. Document design constraints

---

### 11. Snapshot/Backup Not Production-Ready

**Current State**: Raft snapshots exist but no coordinated backup.

**Why It Matters**:

- CockroachDB has incremental backup
- TiKV has PD-managed backups
- Production requires point-in-time recovery

**Recommendation**:

1. Implement coordinated cluster snapshot
2. Add incremental backup support
3. Integrate with iroh-blobs for snapshot storage
4. Add restore procedure

---

## Low Priority Gaps (Nice-to-Have)

### 12. No SQL Layer

**Current State**: SQLite backend allows raw SQL queries, but no distributed SQL.

**Recommendation**: Consider adding a SQL layer similar to CockroachDB's parser, but this is significant scope.

---

### 13. No CNCF Affiliation

**Current State**: Standalone project.

**Recommendation**: Consider CNCF sandbox submission for broader adoption and community.

---

### 14. Limited Observability

**Current State**: Basic metrics via OpenRaft, no distributed tracing.

**Recommendation**: Add OpenTelemetry integration for tracing and metrics.

---

## Improvement Roadmap

### Phase 1: Production Readiness (Immediate)

1. **Benchmark Suite** - Create comprehensive benchmarks
2. **BUGGIFY Integration** - Add fault injection to critical paths
3. **Linearizability Testing** - Implement Porcupine/Elle verification

### Phase 2: Competitive Parity (Short-term)

4. **Load-Based Sharding** - Add hotspot detection
5. **Watch API Improvements** - Guaranteed delivery
6. **Follower Reads** - Closed timestamp tracking

### Phase 3: Feature Expansion (Medium-term)

7. **Coprocessor Framework** - Push-down computation
8. **Client Libraries** - Go, Python bindings
9. **Geo-Partitioning** - Region-aware placement
10. **Production Backup** - Coordinated snapshots

### Phase 4: Polish (Long-term)

11. **Transaction Timeouts** - Bounded transactions
12. **SQL Layer** - Distributed SQL
13. **CNCF Submission** - Community building
14. **Observability** - OpenTelemetry integration

---

## Implementation Notes

### Benchmark Suite Design

```rust
// benches/kv_ops.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn benchmark_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("kv_write");
    group.throughput(Throughput::Elements(1));

    // Single write benchmark
    group.bench_function("single_write", |b| {
        b.iter(|| {
            // Write operation
        })
    });

    // Batch write benchmark
    group.bench_function("batch_write_100", |b| {
        b.iter(|| {
            // Batch of 100 writes
        })
    });

    group.finish();
}
```

### BUGGIFY Macro Design

```rust
// src/testing/buggify.rs
#[macro_export]
macro_rules! buggify {
    ($probability:expr, $action:expr) => {
        #[cfg(feature = "simulation")]
        if $crate::testing::buggify::should_trigger($probability) {
            $action
        }
    };
}

// Usage in production code:
// buggify!(0.01, { tokio::time::sleep(Duration::from_millis(100)).await; });
```

### Linearizability Checker Integration

```rust
// tests/linearizability_test.rs
use porcupine::{LinearizabilityChecker, Operation};

struct KvModel;

impl porcupine::Model for KvModel {
    type State = HashMap<String, String>;
    type Input = KvOp;
    type Output = KvResult;

    fn partition(history: &[Operation<Self::Input, Self::Output>])
        -> Vec<Vec<Operation<Self::Input, Self::Output>>> {
        // Partition by key
    }

    fn step(state: &mut Self::State, input: &Self::Input, output: &Self::Output) -> bool {
        // Verify operation matches sequential spec
    }
}
```

---

## Sources

### FoundationDB

- [BUGGIFY Deep Dive](https://transactional.blog/simulation/buggify)
- [Simulation and Testing](https://apple.github.io/foundationdb/testing.html)
- [DST Learning Resources](https://pierrezemb.fr/posts/learn-about-dst/)

### TiKV

- [Coprocessor Guide](https://tikv.github.io/tikv-dev-guide/understanding-tikv/coprocessor/intro.html)
- [Deep Dive TiKV](https://tikv.github.io/deep-dive-tikv/)
- [Region Scheduling](https://tikv.org/deep-dive/key-manager/region-scheduling/)

### CockroachDB

- [Follower Reads Epic](https://www.cockroachlabs.com/blog/follower-reads-stale-data/)
- [Follower Reads RFC](https://github.com/cockroachdb/cockroach/blob/master/docs/RFCS/20181227_follower_reads_implementation.md)
- [Multi-Region Topology](https://www.cockroachlabs.com/blog/multi-region-topology-patterns/)

### etcd

- [Watch API Guarantees](https://etcd.io/docs/v3.5/learning/api_guarantees/#watch-apis)
- [Performance Benchmarks](https://etcd.io/docs/v3.5/op-guide/performance/)

### Testing

- [Porcupine Linearizability Checker](https://github.com/anishathalye/porcupine)
- [Elle Consistency Checker](https://github.com/jepsen-io/elle)
- [Awesome DST Resources](https://github.com/ivanyu/awesome-deterministic-simulation-testing)
