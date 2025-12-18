# Aspen End-to-End Testing Plan

**Created**: 2025-12-17
**Status**: Planning Phase

## Executive Summary

This document outlines a comprehensive end-to-end (E2E) testing strategy for Aspen, building upon the existing sophisticated testing infrastructure. The plan addresses gaps identified in current testing coverage and incorporates industry best practices for distributed systems testing.

## Current Testing Infrastructure Analysis

### Existing Testing Layers (Ordered by Speed/Determinism)

| Layer | Framework | Speed | Determinism | Test Count | Coverage |
|-------|-----------|-------|-------------|------------|----------|
| Property-Based | Bolero | Fast | Deterministic | 13 files | Edge cases |
| In-Memory | AspenRouter | Very Fast | Deterministic | 5+ files | API routing |
| Simulation | Madsim | Fast | Seeded | 15+ files | Distributed bugs |
| Storage | Tokio + Tempfile | Medium | Deterministic | 5+ files | Persistence |
| Integration | Tokio + Iroh | Slow | Non-deterministic | 5+ files | Real behavior |
| VM-Based | Cloud Hypervisor | Very Slow | Non-deterministic | 3 files | Real systems |

### Existing Test Counts

- **Total test files**: 65+
- **Lines of test code**: ~27,207
- **Passing tests**: 350+
- **Property-based tests**: 13 files with Bolero generators
- **Madsim simulation tests**: 15+ files with deterministic replay
- **VM-based tests**: 3 files (basic, partition, failure)

### Key Testing Infrastructure Components

1. **AspenRouter** (`src/testing/router.rs` - 905 LOC)
   - In-memory Raft routing with configurable delays/failures
   - Network partition simulation
   - Wait helpers for metrics-based assertions

2. **AspenRaftTester** (`src/testing/madsim_tester.rs` - 2,131 LOC)
   - High-level API reducing boilerplate by 80%
   - Byzantine failure injection
   - BUGGIFY-style probabilistic faults
   - Liveness testing (TigerBeetle-style two-phase)

3. **Test Doubles**
   - `DeterministicClusterController` (in-memory cluster control)
   - `DeterministicKeyValueStore` (in-memory KV store)
   - Mock Iroh endpoint and gossip implementations

4. **VM Infrastructure** (`src/testing/vm_manager.rs` - 633 LOC)
   - Cloud Hypervisor microVM management
   - Network bridge/TAP device creation
   - Fault injection via iptables

## Identified Gaps and Improvement Opportunities

### Gap 1: HTTP API E2E Testing

**Current State**: Only `scripts/aspen-cluster-smoke.sh` tests the HTTP API
**Gap**: No programmatic HTTP API tests beyond the bash smoke test
**Impact**: Medium - HTTP API is a primary user interface

### Gap 2: Cross-Layer Integration

**Current State**: Tests operate in isolated layers
**Gap**: No tests that span multiple layers (e.g., HTTP API -> Raft -> Storage)
**Impact**: High - Integration bugs between layers are untested

### Gap 3: Long-Running Stability (Soak Tests)

**Current State**: Single soak test (`soak_sustained_write_madsim.rs`)
**Gap**: Limited duration and scenario coverage
**Impact**: Medium - Production stability under sustained load

### Gap 4: Flaky Test Mitigation

**Current State**: Known flaky tests documented in `gossip_e2e_integration.rs`
**Gap**: Tests use fixed `sleep()` instead of condition-based waiting
**Impact**: High - CI reliability issues

### Gap 5: TUI Testing

**Current State**: No TUI tests exist
**Gap**: Terminal UI (`aspen-tui.rs`) is completely untested
**Impact**: Low-Medium - TUI is secondary interface

### Gap 6: Multi-Storage Backend Comparison

**Current State**: Tests use InMemory or SQLite, rarely both
**Gap**: No comparative tests across storage backends
**Impact**: Low - Storage backends are well-tested individually

## Proposed E2E Testing Strategy

### Phase 1: HTTP API E2E Testing Framework

**Objective**: Create programmatic E2E tests for the HTTP REST API

**Implementation**:

```rust
// tests/http_api_e2e.rs
use reqwest::Client;
use aspen::node::{NodeBuilder, NodeId};
use tempfile::TempDir;

struct HttpClusterHarness {
    nodes: Vec<(Node, u16)>,  // Node + HTTP port
    client: Client,
}

impl HttpClusterHarness {
    async fn new(node_count: usize) -> Self { ... }

    async fn init_cluster(&self) -> Result<()> { ... }

    async fn write_via_http(&self, node: usize, key: &str, value: &str) -> Result<()> { ... }

    async fn read_via_http(&self, node: usize, key: &str) -> Result<String> { ... }

    async fn add_learner_via_http(&self, node: usize, learner_id: u64) -> Result<()> { ... }

    async fn change_membership_via_http(&self, node: usize, members: &[u64]) -> Result<()> { ... }

    async fn get_metrics_via_http(&self, node: usize) -> Result<PrometheusMetrics> { ... }

    async fn health_check(&self, node: usize) -> Result<HealthStatus> { ... }
}
```

**Test Scenarios**:

1. Full cluster lifecycle via HTTP (init, add learner, membership change, shutdown)
2. KV operations through different nodes (write to leader, read from follower)
3. Error handling (invalid requests, not leader, not initialized)
4. Metrics endpoint validation
5. Health check endpoint under various cluster states

### Phase 2: Cross-Layer Integration Tests

**Objective**: Test complete request flows from HTTP API through Raft to storage

**Implementation**:

```rust
// tests/cross_layer_e2e.rs
#[tokio::test]
async fn test_http_to_storage_roundtrip() {
    let harness = CrossLayerHarness::new(3, StorageBackend::Sqlite).await;

    // 1. Write via HTTP API
    harness.http_write("key1", "value1").await.unwrap();

    // 2. Verify data reached SQLite state machine
    let sqlite_value = harness.read_from_sqlite_directly("key1").await.unwrap();
    assert_eq!(sqlite_value, "value1");

    // 3. Read via KeyValueStore trait (bypassing HTTP)
    let trait_value = harness.trait_read("key1").await.unwrap();
    assert_eq!(trait_value, "value1");

    // 4. Verify Raft log entry was created
    let log_entries = harness.get_raft_log_entries().await.unwrap();
    assert!(log_entries.iter().any(|e| e.contains("key1")));
}
```

**Test Scenarios**:

1. Write flow: HTTP -> RaftNode -> Raft consensus -> SQLite state machine
2. Read flow: HTTP -> RaftNode -> ReadIndex -> SQLite query
3. Snapshot flow: Trigger snapshot -> Verify SQLite state captured
4. Recovery flow: Kill node -> Restart -> Verify state from SQLite

### Phase 3: Robust Waiting Patterns (Fix Flaky Tests)

**Objective**: Replace fixed `sleep()` with condition-based waiting

**Implementation**:

```rust
// tests/support/wait_helpers.rs
use tokio::time::{timeout, Duration};

/// Wait for a condition with exponential backoff.
pub async fn wait_for<F, Fut>(
    condition: F,
    description: &str,
    max_wait: Duration,
) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    let mut backoff = Duration::from_millis(10);

    while start.elapsed() < max_wait {
        if condition().await {
            return Ok(());
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_millis(500));
    }

    anyhow::bail!("Timeout waiting for: {}", description)
}

/// Wait for leader election to complete.
pub async fn wait_for_leader(
    node: &RaftNode,
    max_wait: Duration,
) -> Result<u64> {
    wait_for(
        || async {
            node.get_leader().await.ok().flatten().is_some()
        },
        "leader election",
        max_wait,
    ).await?;

    node.get_leader().await?.ok_or_else(|| anyhow::anyhow!("No leader"))
}

/// Wait for data replication to all nodes.
pub async fn wait_for_replication(
    nodes: &[RaftNode],
    key: &str,
    expected_value: &str,
    max_wait: Duration,
) -> Result<()> {
    for (i, node) in nodes.iter().enumerate() {
        wait_for(
            || async {
                node.read(ReadRequest { key: key.to_string() })
                    .await
                    .map(|r| r.value == expected_value)
                    .unwrap_or(false)
            },
            &format!("replication to node {}", i),
            max_wait,
        ).await?;
    }
    Ok(())
}
```

**Refactoring Targets**:

1. `gossip_e2e_integration.rs`: Replace `sleep(Duration::from_secs(10))` with `wait_for_leader()`
2. `raft_node_direct_api_test.rs`: Replace `sleep(Duration::from_millis(500))` with condition waits
3. All tests using arbitrary sleep durations

### Phase 4: Enhanced Soak Testing

**Objective**: Comprehensive long-running stability tests

**Implementation**:

```rust
// tests/soak/comprehensive_soak.rs
#[tokio::test]
#[ignore] // Run explicitly with: cargo nextest run --ignored
async fn soak_test_24h_operations() {
    let config = SoakTestConfig {
        duration: Duration::from_hours(24),
        node_count: 5,
        write_rate_per_sec: 100,
        read_rate_per_sec: 500,
        fault_injection_rate: Duration::from_secs(60),
        metrics_collection_interval: Duration::from_secs(10),
    };

    let harness = SoakTestHarness::new(config).await;

    // Continuous operation loop
    harness.run_with_faults(|harness| async {
        // Periodically inject faults
        harness.inject_random_fault().await;

        // Verify invariants after each fault
        harness.verify_cluster_invariants().await?;

        Ok(())
    }).await.unwrap();

    // Generate comprehensive report
    harness.generate_report().await;
}
```

**Soak Test Scenarios**:

1. **Sustained Write Load**: 100 writes/sec for 24h
2. **Membership Churn**: Add/remove nodes every 5 minutes
3. **Leader Failover**: Kill leader every 10 minutes
4. **Network Degradation**: Inject latency/packet loss periodically
5. **Storage Pressure**: Fill storage to 90% capacity

### Phase 5: Chaos Engineering Framework

**Objective**: Systematic fault injection for resilience validation

**Implementation**:

```rust
// tests/chaos/chaos_framework.rs
pub enum ChaosFault {
    // Node faults
    NodeCrash { node_id: u64 },
    NodeSlowdown { node_id: u64, factor: u32 },

    // Network faults
    NetworkPartition { isolated: Vec<u64> },
    NetworkLatency { from: u64, to: u64, latency_ms: u64 },
    PacketLoss { node_id: u64, loss_percent: u8 },

    // Storage faults
    StorageSlowdown { node_id: u64, latency_ms: u64 },
    StorageCorruption { node_id: u64, corruption_type: CorruptionType },

    // Clock faults
    ClockSkew { node_id: u64, skew_ms: i64 },
}

pub struct ChaosRunner {
    faults: Vec<(Duration, ChaosFault)>,
}

impl ChaosRunner {
    pub fn random_schedule(duration: Duration, fault_rate: Duration) -> Self { ... }

    pub async fn run(&self, harness: &TestHarness) -> ChaosReport { ... }
}
```

**Chaos Scenarios**:

1. **Jepsen-style**: Network partition during writes
2. **Byzantine**: Message corruption between specific nodes
3. **Cascading Failure**: Multiple node crashes in sequence
4. **Split Brain**: Symmetric network partition
5. **Slow Node**: Single node with 10x latency

### Phase 6: TUI Testing (Optional)

**Objective**: Automated testing for terminal UI

**Implementation**:

```rust
// tests/tui_e2e.rs
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_tui_cluster_view() {
    let backend = TestBackend::new(80, 24);
    let mut app = App::new_for_testing(backend);

    // Simulate cluster connection
    app.connect_to_cluster(&["127.0.0.1:21001"]).await.unwrap();

    // Render frame
    app.draw().unwrap();

    // Assert UI state
    let buffer = app.backend().buffer();
    assert!(buffer.contains("Cluster: healthy"));
    assert!(buffer.contains("Leader: node-1"));
}
```

## Implementation Priority

### High Priority (Address Immediately)

1. **Phase 3**: Fix flaky tests with condition-based waiting
2. **Phase 1**: HTTP API E2E framework
3. **Phase 2**: Cross-layer integration tests

### Medium Priority (Next Sprint)

4. **Phase 5**: Chaos engineering framework
5. **Phase 4**: Enhanced soak testing

### Low Priority (Future Consideration)

6. **Phase 6**: TUI testing

## Test Execution Strategy

### CI Integration

```yaml
# .github/workflows/test.yml
jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop -c cargo nextest run --workspace

  property-tests:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop -c cargo nextest run --workspace -p bolero

  madsim-tests:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop -c cargo nextest run madsim_ --test-threads=1

  http-e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop -c cargo nextest run http_e2e_

  soak-tests:
    runs-on: ubuntu-latest
    timeout-minutes: 480  # 8 hours
    if: github.event_name == 'schedule'  # Nightly only
    steps:
      - run: nix develop -c cargo nextest run --ignored soak_
```

### Local Development

```bash
# Quick feedback loop (< 1 min)
nix develop -c cargo nextest run --lib

# Full test suite (< 10 min)
nix develop -c cargo nextest run

# E2E tests only (< 5 min)
nix develop -c cargo nextest run e2e_

# Madsim simulation (< 5 min)
MADSIM_TEST_SEED=42 nix develop -c cargo nextest run madsim_

# Soak test (hours)
nix develop -c cargo nextest run --ignored soak_test_24h
```

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Test count | 350+ | 500+ |
| E2E test coverage | ~10% | 60% |
| Flaky test rate | ~5% | <1% |
| CI pass rate | ~95% | >99% |
| Time to feedback | ~5min | <3min |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| E2E tests too slow | Medium | High | Parallelize, use test harness caching |
| Flaky tests return | Medium | Medium | Strict review of sleep usage |
| VM tests fail in CI | High | Low | Make VM tests optional/nightly |
| Test maintenance burden | Medium | Medium | Shared test infrastructure, good helpers |

## References

- [FoundationDB Testing](https://apple.github.io/foundationdb/testing.html)
- [TigerBeetle VOPR](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md)
- [RisingWave DST](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/)
- [S2 DST Blog](https://s2.dev/blog/dst) - Turmoil + MadSim hybrid approach
- [Jepsen Testing](https://jepsen.io/) - Distributed systems correctness testing
