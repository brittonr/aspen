# Aspen Improvement Recommendations: Competitive Gap Analysis

**Analysis Date**: 2025-12-22
**Based on**: Comparison with FoundationDB, TiKV, etcd, and CockroachDB

---

## Executive Summary

Aspen is a production-ready distributed orchestration layer with strong foundations in Rust, Iroh P2P networking, and deterministic simulation testing. However, compared to mature competitors, it has significant gaps in four key areas:

1. **Testing Depth**: Byzantine failure coverage, message-level tracing, BUGGIFY integration in production code
2. **Sharding Maturity**: Dynamic shard management, load-based redistribution, range query optimization
3. **Transaction Features**: Bounded staleness reads, lease keepalive protocol, watch API filtering
4. **Observability & Performance**: Leaseholder optimization, benchmark infrastructure, distributed tracing

This document prioritizes improvements by impact and provides actionable implementation guidance.

---

## Gap Analysis Summary

| Category | Current State | Industry Standard | Gap Severity |
|----------|--------------|-------------------|--------------|
| **Deterministic Simulation** | madsim-based, 16 seeds | FoundationDB: 100k seeds/PR | Medium |
| **BUGGIFY Fault Injection** | 8 types in tester only | FDB: 50+ types woven in code | Critical |
| **Byzantine Testing** | <5 tests | TigerBeetle: 50+ scenarios | Critical |
| **Shard Splitting** | Fixed at bootstrap | TiKV: Size-based auto-split | Critical |
| **Load Redistribution** | None | TiKV: PD-managed balancing | Critical |
| **Follower Reads** | Binary (stale/linearizable) | CRDB: Bounded staleness | High |
| **Leaseholder Optimization** | None | CRDB: 3-10x read speedup | High |
| **Benchmark Suite** | None | All competitors have YCSB | Critical |
| **Distributed Tracing** | Local tracing only | CRDB: Jaeger integration | High |
| **Watch Filtering** | Prefix only | etcd: Condition-based | Medium |

---

## Priority 1: Critical Improvements (Production Blockers)

### 1.1 Benchmark Infrastructure

**Why Critical**: Cannot validate performance claims or measure improvement impact without baselines.

**Current State**: Zero benchmarks. Only integration tests exist.

**Implementation**:

```rust
// benches/kv_throughput.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_write_throughput(c: &mut Criterion) {
    // Measure single-key writes, batch writes, reads
}

fn benchmark_scan_latency(c: &mut Criterion) {
    // Measure prefix scans with varying result sizes
}

fn benchmark_raft_consensus(c: &mut Criterion) {
    // Measure leader election, log replication latency
}
```

**Deliverables**:

1. Micro-benchmarks for KV operations (read, write, scan, delete)
2. Cluster benchmarks (3-node, 5-node throughput)
3. Latency histograms (p50, p95, p99, p99.9)
4. YCSB workload compatibility (A, B, C, D, F)

**References**:

- [etcd benchmark](https://etcd.io/docs/v3.5/op-guide/performance/)
- [CockroachDB TPC-C](https://www.cockroachlabs.com/docs/stable/performance-benchmarking-with-tpc-c)

---

### 1.2 Byzantine Failure Testing

**Why Critical**: FoundationDB's reputation for correctness comes from exhaustive Byzantine testing. Aspen has <5 Byzantine tests vs industry standard of 50+.

**Current State**: Only basic partition/crash tests. No message corruption, vote manipulation, or state divergence tests.

**Missing Scenarios**:

```
+----------------------------------+
| Byzantine Scenario               |
+----------------------------------+
| Message bit-flip during vote     |
| Leader spoofing (false claims)   |
| State machine divergence         |
| Vote duplication attack          |
| Log entry corruption             |
| Snapshot tampering               |
| Clock manipulation attacks       |
+----------------------------------+
```

**Implementation**:

```rust
// tests/byzantine_scenarios.rs
#[madsim::test]
async fn test_byzantine_message_corruption() {
    // Inject bit flips into AppendEntries messages
    // Verify cluster rejects corrupted messages
}

#[madsim::test]
async fn test_byzantine_state_divergence() {
    // Apply different commands to different nodes
    // Verify divergence detected and cluster recovers
}
```

**Deliverables**:

1. 20+ Byzantine scenario tests
2. State machine divergence detection
3. Message integrity verification hooks
4. Comprehensive coverage of Raft attack surface

---

### 1.3 Message-Level Tracing

**Why Critical**: FoundationDB's deterministic replay capability depends on complete message traces. Without this, debugging distributed failures is nearly impossible.

**Current State**: Event traces capture high-level operations ("leader elected", "3 nodes created") but no individual RPC messages.

**Implementation**:

```rust
// src/raft/network.rs
pub struct TracedRaftNetwork<N> {
    inner: N,
    trace: Arc<RwLock<MessageTrace>>,
}

impl<N: RaftNetwork> RaftNetwork for TracedRaftNetwork<N> {
    async fn send_append_entries(&self, target: NodeId, msg: AppendEntriesRequest)
        -> Result<AppendEntriesResponse>
    {
        let trace_id = self.trace.write().record_send(target, &msg);
        let response = self.inner.send_append_entries(target, msg).await;
        self.trace.write().record_response(trace_id, &response);
        response
    }
}
```

**Deliverables**:

1. Message trace capture in `SimulationArtifact`
2. State snapshots at Raft decision points
3. Deterministic replay from captured traces
4. Causality graph generation for debugging

---

### 1.4 Dynamic Shard Management

**Why Critical**: Current static sharding prevents production use. Clusters cannot scale without downtime.

**Current State**: Shard count fixed at bootstrap. Adding shards requires cluster restart + full key migration.

**What TiKV Does** ([reference](https://tikv.org/docs/3.0/concepts/architecture/)):

- Regions split when size exceeds 64MB
- Load-based splitting when QPS > threshold
- Automatic hotspot detection via bucket statistics
- PD (Placement Driver) orchestrates rebalancing

**Implementation Plan**:

```rust
// src/sharding/split.rs
pub struct ShardSplitter {
    /// Threshold for size-based split (default: 64GB)
    size_threshold_bytes: u64,
    /// Threshold for load-based split (QPS)
    qps_threshold: u64,
}

impl ShardSplitter {
    /// Check if shard should split, returns split point
    pub async fn should_split(&self, shard: &Shard) -> Option<SplitPoint> {
        if shard.size_bytes() > self.size_threshold_bytes {
            return Some(self.find_split_point(shard));
        }
        if shard.qps() > self.qps_threshold {
            return Some(self.find_hot_split_point(shard));
        }
        None
    }

    /// Execute split operation (atomic via Raft)
    pub async fn execute_split(&self, shard: ShardId, point: SplitPoint)
        -> Result<(ShardId, ShardId)>;
}
```

**Deliverables**:

1. Shard splitting algorithm (size-based)
2. Shard merging for under-utilized shards
3. Metadata versioning for topology changes
4. Online key migration protocol

---

## Priority 2: High-Impact Improvements

### 2.1 Leaseholder Optimization

**Why High**: CockroachDB's leaseholder pattern reduces read latency by 3-10x for read-heavy workloads.

**Current State**: All reads go through Raft ReadIndex, adding 2-3ms latency even when data is local.

**How CockroachDB Does It** ([reference](https://www.cockroachlabs.com/blog/follow-the-workload/)):

- One replica designated as "leaseholder" for each range
- Leaseholder handles all reads without consensus
- Timestamp cache prevents stale reads
- Lease transfers based on load distribution

**Implementation**:

```rust
// src/raft/leaseholder.rs
pub struct ReadLease {
    holder: NodeId,
    start_time: Instant,
    duration: Duration,
}

impl RaftNode {
    /// Fast path: read from leaseholder without consensus
    pub async fn leaseholder_read(&self, key: &str) -> Result<Option<Value>> {
        if self.is_leaseholder() && !self.lease_expired() {
            self.timestamp_cache.record_read(key);
            return self.local_read(key).await;
        }
        self.raft_read(key).await  // Fallback to consensus
    }
}
```

**Deliverables**:

1. Lease acquisition/renewal protocol
2. Timestamp cache for read tracking
3. Lease transfer based on request locality
4. Metrics for lease utilization

---

### 2.2 Bounded Staleness Reads

**Why High**: Current binary choice (stale/linearizable) forces users to choose between performance and consistency. Bounded staleness provides middle ground.

**Current State**: SQL queries support `Stale` or `Linearizable`. No intermediate options.

**How CockroachDB Does It** ([reference](https://www.cockroachlabs.com/blog/bounded-staleness-reads/)):

- `AS OF SYSTEM TIME` clause with configurable staleness
- Dynamic timestamp selection minimizes lag while ensuring locality
- Closed timestamp propagation enables follower reads

**Implementation**:

```rust
// src/api/mod.rs
pub enum ReadConsistency {
    Linearizable,           // Raft ReadIndex (current)
    BoundedStaleness {      // New: read from follower if lag < bound
        max_staleness_ms: u64,
    },
    Stale,                  // Local read, no guarantees
}
```

**Deliverables**:

1. Extend `SqlConsistency` enum with bounded option
2. Track replication lag per node
3. Route reads to followers when within staleness bound
4. Metrics for staleness distribution

---

### 2.3 BUGGIFY Integration in Production Code

**Why High**: FoundationDB's BUGGIFY is woven throughout production code, not just test code. This finds bugs that targeted testing misses.

**Current State**: BUGGIFY exists in `AspenRaftTester` (test code only). No fault injection points in production code paths.

**How FoundationDB Does It** ([reference](https://transactional.blog/simulation/buggify)):

- `BUGGIFY` macro in every dangerous code path
- Only evaluates to true in simulation
- 25% probability when enabled (per-invocation)
- First 18 months of FDB was simulation-only

**Implementation**:

```rust
// src/buggify.rs
#[cfg(feature = "simulation")]
macro_rules! buggify {
    () => {
        crate::simulation::should_buggify()
    };
    ($prob:expr) => {
        crate::simulation::should_buggify_with_prob($prob)
    };
}

#[cfg(not(feature = "simulation"))]
macro_rules! buggify {
    () => { false };
    ($prob:expr) => { false };
}

// Usage in production code:
// src/raft/node.rs
async fn send_append_entries(&self, target: NodeId, msg: AppendEntries) {
    if buggify!(0.01) {  // 1% chance in simulation
        tokio::time::sleep(Duration::from_millis(100)).await;  // Inject delay
    }
    self.network.send(target, msg).await
}
```

**Deliverables**:

1. `buggify!` macro with feature flag
2. Integrate into Raft RPC paths (20+ injection points)
3. Integrate into storage operations (10+ injection points)
4. CI pipeline runs with BUGGIFY enabled

---

### 2.4 Distributed Tracing (OpenTelemetry)

**Why High**: Multi-node cluster debugging is impossible without distributed trace correlation.

**Current State**: Local tracing via `tracing` crate. No cross-node correlation.

**Implementation**:

```rust
// src/tracing/otel.rs
use opentelemetry::trace::{Tracer, TracerProvider};
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn init_distributed_tracing() -> Result<()> {
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("aspen")
        .install_simple()?;

    tracing_subscriber::registry()
        .with(OpenTelemetryLayer::new(tracer))
        .init();
    Ok(())
}
```

**Deliverables**:

1. OpenTelemetry integration
2. Trace context propagation in Raft RPCs
3. Jaeger exporter configuration
4. Correlation IDs in log messages

---

## Priority 3: Medium-Impact Improvements

### 3.1 Watch API Filtering

**Why Medium**: etcd's watch API supports rich filtering by condition, revision, and lease. Aspen only supports prefix filtering.

**Current State**: `WatchSession.subscribe(prefix, start_index)` - prefix and start index only.

**What etcd Does** ([reference](https://etcd.io/docs/v3.5/learning/api_guarantees/)):

- Filter by key range, single key, or prefix
- Start from specific revision
- Watch only create/update/delete events selectively
- Progress notifications for long-polling

**Implementation**:

```rust
// src/client/watch.rs
pub struct WatchFilter {
    pub prefix: Option<String>,
    pub key_range: Option<(String, String)>,
    pub event_types: HashSet<EventType>,
    pub start_revision: Option<u64>,
    pub progress_notify: bool,
}
```

**Deliverables**:

1. Extended filter struct
2. Server-side filtering logic
3. Progress notification support
4. Documentation with etcd comparison

---

### 3.2 Lease Keepalive Protocol

**Why Medium**: etcd clients can extend lease TTL via keepalive RPC. Aspen leases have fixed TTL set at creation.

**Current State**: `LeaseGrant(id, ttl_seconds)` creates lease. No renewal mechanism.

**Implementation**:

```rust
// src/api/mod.rs
pub enum WriteCommand {
    // Existing...
    LeaseKeepalive { lease_id: i64 },  // New: extend TTL
}

// Client API
impl LeaseClient {
    /// Keep lease alive, extending TTL by original duration
    pub async fn keepalive(&self, lease_id: i64) -> Result<LeaseInfo>;

    /// Stream for continuous keepalive
    pub fn keepalive_stream(&self, lease_id: i64)
        -> impl Stream<Item = Result<LeaseInfo>>;
}
```

**Deliverables**:

1. `LeaseKeepalive` command in state machine
2. Client keepalive RPC
3. Streaming keepalive for long-lived leases
4. Lease TTL extension semantics

---

### 3.3 Per-Shard Metrics

**Why Medium**: Sharding was added but has no observability. Cannot identify hot shards or imbalanced distribution.

**Current State**: Only cluster-wide Raft metrics. No per-shard visibility.

**Implementation**:

```rust
// src/sharding/metrics.rs
pub struct ShardMetrics {
    pub shard_id: ShardId,
    pub key_count: u64,
    pub size_bytes: u64,
    pub read_qps: f64,
    pub write_qps: f64,
    pub replication_lag_ms: u64,
}

impl ShardedKeyValueStore {
    pub async fn get_shard_metrics(&self) -> Vec<ShardMetrics>;
}
```

**Deliverables**:

1. Per-shard metrics collection
2. Prometheus export with shard labels
3. Hot shard detection alerts
4. Dashboard templates for Grafana

---

### 3.4 Range-Based Sharding Option

**Why Medium**: Hash-based sharding requires scanning ALL shards for prefix queries. Range-based would enable efficient range scans.

**Current State**: Jump hash only. Prefix queries scale O(num_shards).

**What TiKV Does** ([reference](https://tikv.org/deep-dive/scalability/data-sharding/)):

- Range-based sharding for scan-friendliness
- Split/merge only changes metadata, not data
- Supports both range and hash strategies

**Implementation**:

```rust
// src/sharding/range_router.rs
pub struct RangeRouter {
    ranges: BTreeMap<String, ShardId>,  // start_key -> shard
}

impl RangeRouter {
    /// O(log n) lookup for range-based routing
    pub fn get_shard_for_key(&self, key: &str) -> ShardId;

    /// Efficient: only returns shards overlapping prefix
    pub fn get_shards_for_prefix(&self, prefix: &str) -> Vec<ShardId>;
}
```

**Deliverables**:

1. Range-based router implementation
2. Configuration to choose hash vs range
3. Hybrid mode (hash for uniform, range for ordered)
4. Migration path from hash to range

---

## Priority 4: Nice-to-Have Improvements

### 4.1 Coprocessor Framework

Push-down computation to reduce network traffic. Low priority because KV-focused workloads don't benefit as much as SQL workloads.

### 4.2 Multi-Region Deployment Patterns

Documented topologies for geo-distributed clusters. Deferred until sharding stabilizes.

### 4.3 Client Libraries (Go, Python, TypeScript)

Foreign language bindings. Deferred until Rust API stabilizes.

### 4.4 Adaptive Batch Sizing

Dynamic batch size based on load. Marginal improvement over fixed limits.

---

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)

| Task | Effort | Dependencies |
|------|--------|--------------|
| Benchmark infrastructure | 2 weeks | None |
| Byzantine test suite | 2 weeks | None |
| Message-level tracing | 1 week | Simulation artifacts |
| BUGGIFY macro + 10 injection points | 1 week | None |

### Phase 2: Sharding Maturity (Weeks 5-8)

| Task | Effort | Dependencies |
|------|--------|--------------|
| Shard splitting (size-based) | 2 weeks | Per-shard metrics |
| Per-shard metrics | 1 week | None |
| Metadata versioning | 1 week | Shard splitting |
| Key migration protocol | 2 weeks | Shard splitting |

### Phase 3: Performance Optimization (Weeks 9-12)

| Task | Effort | Dependencies |
|------|--------|--------------|
| Leaseholder optimization | 2 weeks | Benchmarks |
| Bounded staleness reads | 1 week | Replication lag tracking |
| Distributed tracing | 1 week | None |
| Watch API filtering | 1 week | None |

### Phase 4: Polish (Weeks 13-16)

| Task | Effort | Dependencies |
|------|--------|--------------|
| Lease keepalive protocol | 1 week | None |
| Range-based sharding option | 2 weeks | Sharding metrics |
| Load-based redistribution | 2 weeks | Per-shard metrics, splitting |
| Hot shard detection + alerts | 1 week | Per-shard metrics |

---

## Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Byzantine test coverage | <5 tests | 50+ tests | Test count |
| BUGGIFY injection points | 8 (tester) | 50+ (production) | Code audit |
| Benchmark availability | None | YCSB A/B/C/D/F | Suite execution |
| Read latency (linearizable) | ~5ms | <2ms | p99 benchmark |
| Sharding flexibility | Static | Dynamic split/merge | Feature flag |
| Observability metrics | 6 | 50+ | Prometheus scrape |

---

## Sources

### FoundationDB

- [BUGGIFY Fault Injection](https://transactional.blog/simulation/buggify)
- [Deterministic Simulation Testing](https://apple.github.io/foundationdb/testing.html)
- [FoundationDB Engineering Philosophy](https://apple.github.io/foundationdb/engineering.html)

### TiKV

- [Load Base Split](https://docs.pingcap.com/tidb/stable/configure-load-base-split/)
- [Data Sharding Deep Dive](https://tikv.github.io/deep-dive-tikv/scalability/data-sharding.html)
- [Region Scalability](https://tikv.github.io/tikv-dev-guide/understanding-tikv/scalability/region.html)

### CockroachDB

- [Follower Reads](https://www.cockroachlabs.com/docs/stable/follower-reads)
- [Bounded Staleness Reads](https://www.cockroachlabs.com/blog/bounded-staleness-reads/)
- [Follow-the-Workload](https://www.cockroachlabs.com/blog/follow-the-workload/)

### etcd

- [API Guarantees](https://etcd.io/docs/v3.5/learning/api_guarantees/)
- [Watch API](https://etcd.io/docs/v3.4/learning/api/)

### MVCC/OCC

- [Tihku MVCC (Rust)](https://github.com/penberg/tihku)
- [TiKV MVCC Module](https://tikv.github.io/doc/tikv/storage/mvcc/index.html)

### Simulation Testing

- [DST Ecosystem Overview](https://databases.systems/posts/open-source-antithesis-p1)
- [Antithesis DST Resources](https://antithesis.com/resources/deterministic_simulation_testing/)
- [RisingWave DST Story](https://www.risingwave.com/blog/applying-deterministic-simulation-the-risingwave-story-part-2-of-2/)
