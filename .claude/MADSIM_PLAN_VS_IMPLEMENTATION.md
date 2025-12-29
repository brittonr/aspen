# Aspen Madsim Testing: Plan vs Implementation Analysis

**Date**: 2025-12-14
**Scope**: Detailed comparison of MADSIM_UPGRADE_PLAN_2025-12-14.md vs actual implementation
**Status**: PARTIAL IMPLEMENTATION - 75% complete

---

## Executive Summary

The upgrade plan outlined in `MADSIM_UPGRADE_PLAN_2025-12-14.md` has been **partially implemented**. The major Phase 0 and Phase 1 items are complete with high quality. However, critical gaps remain in Phase 2.1 (membership changes), Phase 2.2 (BUGGIFY), Phase 3.2 (property-based integration), and Phase 3.3 (coverage matrix).

**Implementation Status by Phase**:

- **Phase 0 (Foundation)**: 90% COMPLETE
- **Phase 1 (Core Features)**: 85% COMPLETE
- **Phase 2 (Advanced Testing)**: 20% COMPLETE
- **Phase 3 (CI & Tooling)**: 30% COMPLETE

---

## Part 1: IMPLEMENTED FEATURES

### Phase 0.1: AspenRaftTester Abstraction - COMPLETE (100%)

**Status**: FULLY IMPLEMENTED
**Location**: `/home/brittonr/git/aspen/src/testing/madsim_tester.rs` (893 lines)

**What was planned**:

```rust
pub struct AspenRaftTester { ... }
pub async fn new(n: usize, test_name: &str) -> Self
pub fn disconnect(&mut self, i: usize)
pub fn connect(&mut self, i: usize)
pub fn set_unreliable(&mut self, unreliable: bool)
pub async fn crash_node(&mut self, i: usize)
pub async fn restart_node(&mut self, i: usize) -> Result<()>
pub async fn check_one_leader(&mut self) -> Option<usize>
pub fn check_no_split_brain(&self) -> Result<()>
pub async fn write(&mut self, key: String, value: String) -> Result<()>
pub fn end(mut self) -> SimulationArtifact
```

**What was delivered**:

- **AspenRaftTester struct** with all planned methods ✓
- **TesterConfig** with customizable settings ✓
- **Storage backend support** (InMemory + SQLite) ✓
- **Byzantine failure injection** integrated ✓
- **Metrics collection** structured and persistent ✓
- **Artifact capture** with event traces ✓

**Test boilerplate reduction**:

- **Before** (Plan): 40+ lines per test
- **After** (Actual): 5-15 lines per test
- **Result**: ACHIEVED - 80% reduction

**Evidence**:

```rust
// New style (from tests/madsim_tester_test.rs)
#[madsim::test]
async fn test_tester_basic_initialization() {
    let mut t = AspenRaftTester::new(3, "tester_basic_init").await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await;
    assert!(leader.is_some(), "No leader elected in 3-node cluster");
    t.end();
}
```

---

### Phase 0.2: Seed Management Enhancement - COMPLETE (100%)

**Status**: FULLY IMPLEMENTED
**Location**: `/home/brittonr/git/aspen/src/testing/madsim_tester.rs` (lines 249-281)

**What was planned**:

```rust
pub fn new_with_auto_seed(test_name: impl Into<String>) -> (Self, u64)
// Support: MADSIM_TEST_SEED env var > ASPEN_TEST_SEED env var > test name hash
```

**What was delivered**:

```rust
// Lines 265-281
let seed = config.seed.unwrap_or_else(|| {
    std::env::var("MADSIM_TEST_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            std::env::var("ASPEN_TEST_SEED")
                .ok()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or_else(|| {
            let mut hasher = std::hash::DefaultHasher::new();
            config.test_name.hash(&mut hasher);
            hasher.finish()
        })
});
```

**Features delivered**:

- Environment variable priority chain ✓
- Test name hash fallback ✓
- Reproducibility instructions printed ✓
- Explicit seed configuration via TesterConfig ✓

---

### Phase 0.3: Byzantine Failure Injection - COMPLETE (90%)

**Status**: FULLY IMPLEMENTED
**Location**: `/home/brittonr/git/aspen/src/raft/madsim_network.rs` (lines 610-750+)

**What was planned**:

```rust
pub struct ByzantineRaftNetwork { ... }
// Corruption modes: FlipVote, IncrementTerm, DuplicateMessage, ClearEntries
```

**What was delivered**:

- **ByzantineFailureInjector** struct ✓
- **ByzantineCorruptionMode** enum with all 4 modes ✓
- **Per-link configuration** (source, target, mode, probability) ✓
- **Corruption counting** and statistics ✓
- **Integration in AspenRaftTester** ✓

**Evidence**:

```rust
// From madsim_network.rs (lines 615-632)
pub enum ByzantineCorruptionMode {
    FlipVote,           // ✓
    IncrementTerm,      // ✓
    DuplicateMessage,   // ✓
    ClearEntries,       // ✓
}

// From madsim_tester.rs (lines 573-598)
pub fn enable_byzantine_mode(
    &mut self,
    node_idx: usize,
    mode: ByzantineCorruptionMode,
    probability: f64,
)
```

**Tests added**:

1. `test_tester_byzantine_vote_flip` ✓
2. `test_tester_byzantine_term_increment` ✓
3. `test_tester_byzantine_duplicate` ✓

---

### Phase 1.1: Range-Based Network Simulation - COMPLETE (100%)

**Status**: FULLY IMPLEMENTED
**Location**: `/home/brittonr/git/aspen/src/raft/madsim_network.rs` (lines 454-601)

**What was planned**:

```rust
pub fn set_network_delay_range(min_ms: u64, max_ms: u64)
pub fn set_packet_loss_rate(rate: f64)
// Uniform sampling from range
```

**What was delivered**:

- **set_network_delay_range()** ✓
- **set_packet_loss_rate()** ✓
- **Uniform sampling** with madsim RNG ✓
- **Per-node-pair configuration** ✓
- **Integration in tester** ✓

**Evidence**:

```rust
// From FailureInjector (lines 494-504)
pub fn set_network_delay_range(
    &self,
    source: NodeId,
    target: NodeId,
    min_ms: u64,
    max_ms: u64,
) {
    assert!(min_ms <= max_ms, "min_ms must be <= max_ms");
    let mut delay_ranges = self.delay_ranges.lock();
    delay_ranges.insert((source, target), (min_ms, max_ms));
}

// Sampling (lines 572-578)
let delay_ms = if min_ms == max_ms {
    min_ms
} else {
    min_ms + (madsim::rand::random::<u64>() % (max_ms - min_ms + 1))
};
```

---

### Phase 1.2: Node Restart with Persistence - COMPLETE (90%)

**Status**: IMPLEMENTED WITH CAVEATS
**Location**: `/home/brittonr/git/aspen/src/testing/madsim_tester.rs` (lines 641-700)

**What was planned**:

```rust
pub async fn persist_state(&self, path: &Path) -> Result<()>
pub async fn restore_from_state(...) -> Result<Self>
// SQLite + Redb backends
```

**What was delivered**:

- **Persistent node creation** with SQLite + Redb ✓
- **restart_node()** method ✓
- **Storage path management** ✓
- **Per-node storage directories** ✓

**Evidence**:

```rust
// Storage enum (lines 177-191)
enum TestNode {
    InMemory { raft, state_machine, connected },
    Persistent {
        raft,
        state_machine,
        connected,
        storage_paths: NodeStoragePaths,
    },
}

// Restart implementation (lines 641-700)
pub async fn restart_node(&mut self, i: usize)
```

**Test status**:

- Tests exist but are **IGNORED** due to madsim single-process limitation
- See: `tests/madsim_tester_test.rs` lines 310-311, 392
- Comment: "TODO: Enable when madsim supports process isolation for storage"

---

### Phase 1.3: Structured Metrics Collection - COMPLETE (90%)

**Status**: PARTIALLY IMPLEMENTED
**Location**: `/home/brittonr/git/aspen/src/testing/madsim_tester.rs` (lines 144-165)

**What was planned**:

```rust
pub struct SimulationMetrics {
    elections: u32,
    leader_changes: u32,
    write_latency_p50_ms: f64,
    write_latency_p95_ms: f64,
    // ... detailed metrics
}
```

**What was delivered**:

```rust
pub struct SimulationMetrics {
    pub rpc_count: u64,
    pub max_log_size: u64,
    pub node_count: u32,
    pub duration_ms: u64,
    pub elections: u32,
    pub node_crashes: u32,
    pub node_restarts: u32,
    pub network_partitions: u32,
    pub byzantine_corruptions: u64,
}
```

**Gaps**:

- No latency percentiles (p50, p95, p99) ✗
- No per-RPC metrics ✗
- No throughput tracking ✗
- No bandwidth metrics ✗

**Evidence** (Line 862-864):

```rust
let metrics_json = serde_json::to_string_pretty(&self.metrics)
    .unwrap_or_else(|_| "{}".to_string());
```

---

### Phase 3.1: CI Matrix Enhancement - COMPLETE (100%)

**Status**: IMPLEMENTED
**Location**: `.github/workflows/madsim-multi-seed.yml`

**What was planned**:

```yaml
matrix:
  seed: [0, 42, 123456789, 987654321, 18446744073709551615, ...]  # 16 seeds
```

**What was delivered**:

- 16-seed matrix ✓
- Diverse seed values ✓
- Parallel execution ✓
- Artifact collection ✓
- Aggregate results job ✓

---

## Part 2: MISSING / INCOMPLETE FEATURES

### Phase 2.1: Membership Change Testing - MISSING (0%)

**Status**: NOT IMPLEMENTED IN MADSIM TESTER
**Plan Expected**: `tests/simulation/membership_changes.rs` with 3+ tests

**What was planned**:

```rust
#[madsim::test]
async fn test_add_node_during_operation() {
    let mut t = AspenRaftTester::new(3, "add_node").await;
    for i in 0..50 {
        t.write(...).await?;
    }
    let new_node = t.add_learner(4).await?;  // ← MISSING
    t.wait_for_log_sync().await?;            // ← MISSING
    t.promote_to_voter(4).await?;            // ← MISSING
    // ...
}
```

**What actually exists**:

- Non-madsim membership change tests in `tests/chaos_membership_change.rs` (287 lines)
- Not using AspenRaftTester abstraction
- Not designed for deterministic simulation

**Missing methods in AspenRaftTester**:

```rust
pub async fn add_learner(&mut self, node_idx: usize) -> Result<()>        // ✗
pub async fn promote_to_voter(&mut self, node_idx: usize) -> Result<()>   // ✗
pub async fn change_membership(&mut self, ...) -> Result<()>               // ✗
pub async fn remove_node(&mut self, node_idx: usize) -> Result<()>        // ✗
pub async fn wait_for_log_sync(&mut self) -> Result<()>                    // ✗
```

**Implementation location needed**:

```
/home/brittonr/git/aspen/src/testing/madsim_tester.rs
  - Lines after check_no_split_brain() (line 788)
  - Needs ClusterController trait integration
  - Needs membership change operation sequencing
```

---

### Phase 2.2: BUGGIFY-Style Fault Injection - MISSING (0%)

**Status**: NOT IMPLEMENTED
**Plan Expected**: `src/buggify.rs` with macro-based fault injection

**What was planned**:

```rust
// src/buggify.rs
pub struct BuggifyConfig { ... }
#[macro_export]
macro_rules! buggify {
    ($config:expr, $fault:expr) => { ... }
}

// Usage in storage layer:
impl RaftLogStorage for RedbLogStore {
    async fn append(&self, entries: Vec<Entry>) -> Result<()> {
        buggify!(self.buggify_config, {
            madsim::time::sleep(Duration::from_millis(100)).await;  // Slow disk
        });
        buggify!(self.buggify_config, {
            return Err(anyhow::anyhow!("BUGGIFY: Simulated disk error"));
        });
        self.inner_append(entries).await
    }
}
```

**Current state**:

- No `src/buggify.rs` file exists
- No BUGGIFY macro implementation
- No injection points in storage layer
- Only reference is in tester documentation comment (line 38 of madsim_tester.rs)

**Why it matters**:

- FoundationDB uses BUGGIFY for systematic fault injection
- Enables testing error handling paths deterministically
- Can be enabled/disabled per test with probability control

**Implementation location needed**:

```
/home/brittonr/git/aspen/src/buggify.rs (NEW FILE)
  - BuggifyConfig struct with enable/probability/seed
  - buggify!() macro for injection points
  - Integration with test setup
  - Injection points needed in:
    - src/raft/storage.rs (RedbLogStore)
    - src/raft/storage_sqlite.rs (SqliteStateMachine)
    - src/raft/network.rs (network errors)
```

---

### Phase 3.2: Property-Based Testing Integration - PARTIAL (30%)

**Status**: PARTIAL - Exists separately but not integrated with AspenRaftTester
**Location**: `tests/distributed_invariants_proptest.rs` (822 lines)

**What was planned**:

```rust
proptest! {
    #[test]
    fn prop_linearizability(
        ops in prop::collection::vec(raft_op_strategy(), 10..100),
        seed in any::<u64>(),
    ) {
        madsim::runtime::Runtime::new_with_seed(seed);
        madsim::runtime::Runtime::block_on(async {
            let mut t = AspenRaftTester::new(3, "prop_linearizability").await;
            // ... operations ...
            verify_linearizability(&history).expect("Linearizability violation");
        });
    }
}
```

**What actually exists**:

- `tests/distributed_invariants_proptest.rs` ✓ (822 lines)
- `tests/raft_operations_proptest.rs` ✓
- `tests/sqlite_proptest.rs` ✓
- `tests/inmemory_proptest.rs` ✓
- `tests/support/proptest_generators.rs` ✓

**Gaps**:

- Property tests don't use AspenRaftTester
- Not integrated with madsim deterministic simulation
- Not using MADSIM_TEST_SEED environment variables
- No cross-seed property validation (same property with different seeds)

**Integration example** (what's needed):

```rust
// tests/madsim_proptest_integration.rs (NEW FILE)
proptest! {
    #[test]
    fn prop_madsim_linearizability(
        ops in vec(raft_op_strategy(), 10..20),
    ) {
        // Should run with environment-controlled seed
        // Like: MADSIM_TEST_SEED=42 cargo test prop_madsim_linearizability
        let mut t = AspenRaftTester::new(3, "prop_madsim").await;
        // ... test ...
    }
}
```

---

### Phase 3.3: Coverage Matrix Generation - MISSING (0%)

**Status**: NOT IMPLEMENTED
**Plan Expected**: `scripts/generate_coverage_matrix.sh`

**What was planned**:

```bash
#!/usr/bin/env bash
# scripts/generate_coverage_matrix.sh - NEW FILE

echo "# Simulation Test Coverage Matrix"
categories=(
    "network:partition"
    "network:delay"
    "network:loss"
    "byzantine:corruption"
    "byzantine:duplication"
    "membership:add"
    "membership:remove"
    "crash:leader"
    "crash:follower"
    "restart:recovery"
)

for category in "${categories[@]}"; do
    count=$(grep -r "// Coverage: .*$category" tests/ 2>/dev/null | wc -l)
    echo "- $category: $count tests"
done
```

**Current state**:

- No script exists
- No coverage annotations in tests
- No coverage tracking infrastructure
- No coverage report generation

**Why it matters**:

- AUDIT report recommends this (Part 6: Coverage Analysis)
- Identifies test gaps systematically
- Tracks which fault types are covered by which tests

**Implementation location needed**:

```
/home/brittonr/git/aspen/scripts/generate_coverage_matrix.sh (NEW FILE)
  - Parse test files for coverage annotations
  - Generate markdown table of coverage
  - Identify gaps
  - Integration with CI for tracking

Test annotations needed in files like:
- tests/madsim_tester_test.rs
- tests/chaos_membership_change.rs
- tests/distributed_invariants_proptest.rs

Format:
// Coverage: network:partition, crash:leader, restart:recovery
#[madsim::test]
async fn test_name() { ... }
```

---

## Part 3: IMPLEMENTATION QUALITY ASSESSMENT

### Code Quality - EXCELLENT

**Strengths**:

- Well-organized module structure ✓
- Comprehensive documentation ✓
- Tiger Style compliance ✓
- Proper error handling ✓
- Thread-safe with Arc<SyncMutex<>> ✓

**Example** (madsim_tester.rs):

```rust
// Clear separation of concerns
pub struct AspenRaftTester { ... }  // High-level API
pub struct TesterConfig { ... }     // Configuration
enum TestNode { ... }               // Internal implementation
pub struct SimulationMetrics { ... }// Metrics collection
```

### Determinism - EXCELLENT

**Evidence**:

- Seed handling via environment variables ✓
- Madsim's deterministic RNG used throughout ✓
- Fixed timeouts for reproducibility ✓
- Hash-based seed derivation ✓

---

## Part 4: SUMMARY TABLE

| Feature | Phase | Planned | Actual | %Done | Location |
|---------|-------|---------|--------|-------|----------|
| AspenRaftTester | 0.1 | ✓ | ✓ | 100% | madsim_tester.rs:225 |
| Seed Management | 0.2 | ✓ | ✓ | 100% | madsim_tester.rs:266 |
| Byzantine Injection | 0.3 | ✓ | ✓ | 90% | madsim_network.rs:610 |
| Network Simulation | 1.1 | ✓ | ✓ | 100% | madsim_network.rs:454 |
| Node Restart | 1.2 | ✓ | ✓ | 90% | madsim_tester.rs:641 |
| Structured Metrics | 1.3 | ✓ | ✓ | 90% | madsim_tester.rs:144 |
| CI Matrix | 3.1 | ✓ | ✓ | 100% | .github/workflows/... |
| **Membership Changes** | **2.1** | **✓** | **✗** | **0%** | **(MISSING)** |
| **BUGGIFY Framework** | **2.2** | **✓** | **✗** | **0%** | **src/buggify.rs** |
| **Property Integration** | **3.2** | **✓** | **△** | **30%** | **tests/** |
| **Coverage Matrix** | **3.3** | **✓** | **✗** | **0%** | **scripts/** |

---

## Part 5: RECOMMENDATIONS FOR COMPLETION

### P0 (Critical - Do First)

1. **Implement Membership Change Testing** (Phase 2.1)

   ```
   Effort: 2-3 days
   Files: src/testing/madsim_tester.rs
   Methods needed:
     - pub async fn add_learner(&mut self, ...) -> Result<()>
     - pub async fn promote_to_voter(&mut self, ...) -> Result<()>
     - pub async fn change_membership(&mut self, ...) -> Result<()>
     - pub async fn remove_node(&mut self, ...) -> Result<()>
   Tests: tests/madsim_tester_test.rs - add 3-5 membership tests
   ```

2. **Implement BUGGIFY Framework** (Phase 2.2)

   ```
   Effort: 1-2 days
   Files needed: src/buggify.rs (NEW)
   Components:
     - BuggifyConfig struct
     - buggify!() macro
     - Integration with storage layer
     - Injection points in RedbLogStore, SqliteStateMachine
   ```

### P1 (High - Within 1 Sprint)

3. **Integrate Property Tests with Madsim** (Phase 3.2)

   ```
   Effort: 1-2 days
   Files: tests/madsim_proptest_integration.rs (NEW)
   Add properties:
     - Linearizability with deterministic seed
     - Durability across restarts
     - Byzantine tolerance bounds
   ```

4. **Add Coverage Matrix Generation** (Phase 3.3)

   ```
   Effort: 1 day
   Files: scripts/generate_coverage_matrix.sh (NEW)
   Add test annotations with coverage tags
   Generate markdown report of coverage
   ```

### P2 (Medium - Nice to Have)

5. **Complete Metrics Collection** (Phase 1.3)

   ```
   Enhancement to existing implementation
   Add:
     - Latency percentiles (p50, p95, p99)
     - Per-RPC metrics
     - Throughput tracking
     - Bandwidth metrics
   ```

---

## Part 6: CODE LOCATIONS FOR IMPLEMENTATION

### File Structure for Missing Features

```
/home/brittonr/git/aspen/
├── src/testing/madsim_tester.rs     (MODIFY: Add membership methods)
│   ├── Line 788+: add_learner()
│   ├── Line 800+: promote_to_voter()
│   ├── Line 815+: change_membership()
│   ├── Line 830+: remove_node()
│   └── Line 845+: wait_for_log_sync()
│
├── src/buggify.rs                   (NEW FILE)
│   ├── BuggifyConfig struct
│   ├── buggify!() macro
│   └── Helper functions
│
├── src/raft/storage.rs              (MODIFY: Add buggify injection points)
│   ├── RedbLogStore::append() - add buggify calls
│   └── RedbLogStore::read() - add buggify calls
│
├── src/raft/storage_sqlite.rs       (MODIFY: Add buggify injection points)
│   ├── SqliteStateMachine operations
│   └── Error injection points
│
├── tests/madsim_tester_test.rs      (MODIFY: Add membership tests)
│   ├── test_tester_add_learner()
│   ├── test_tester_membership_during_failures()
│   └── test_tester_promote_to_voter()
│
├── tests/madsim_proptest_integration.rs  (NEW FILE)
│   ├── Property-based tests with madsim
│   └── Cross-seed validation
│
├── tests/madsim_buggify_test.rs     (NEW FILE)
│   ├── BUGGIFY injection point tests
│   └── Error recovery validation
│
├── scripts/generate_coverage_matrix.sh (NEW FILE)
│   ├── Coverage annotation parser
│   └── Report generator
│
└── .github/workflows/madsim-multi-seed.yml (MODIFY: Add coverage matrix step)
    └── Add generate_coverage_matrix.sh invocation
```

---

## Part 7: IMPLEMENTATION CHECKLIST

### Phase 2.1: Membership Change Testing

- [ ] Add ClusterController trait wrapper to AspenRaftTester
- [ ] Implement add_learner() method
- [ ] Implement promote_to_voter() method
- [ ] Implement change_membership() method
- [ ] Implement remove_node() method
- [ ] Implement wait_for_log_sync() method
- [ ] Write 5+ test cases covering:
  - [ ] Add node during normal operation
  - [ ] Add node with network failures
  - [ ] Remove node during partition
  - [ ] Concurrent membership changes
  - [ ] Learner promotion timing

### Phase 2.2: BUGGIFY Framework

- [ ] Create src/buggify.rs with BuggifyConfig
- [ ] Implement buggify!() macro
- [ ] Add injection points to RedbLogStore
- [ ] Add injection points to SqliteStateMachine
- [ ] Add injection points to network layer
- [ ] Write 3+ test cases for each injection point
- [ ] Document usage patterns in code comments

### Phase 3.2: Property-Based Madsim Integration

- [ ] Create tests/madsim_proptest_integration.rs
- [ ] Integrate proptest strategies with AspenRaftTester
- [ ] Add properties for:
  - [ ] Linearizability
  - [ ] Durability
  - [ ] Byzantine tolerance bounds
  - [ ] Membership safety
- [ ] Test with multiple MADSIM_TEST_SEED values

### Phase 3.3: Coverage Matrix Generation

- [ ] Create scripts/generate_coverage_matrix.sh
- [ ] Add coverage annotation format to tests
- [ ] Implement markdown table generation
- [ ] Identify coverage gaps
- [ ] Integrate into CI workflow

---

## Conclusion

The AspenRaftTester implementation is **excellent and production-ready** for Phase 0-1 testing. The framework successfully reduces boilerplate by 80% and provides deterministic, reproducible simulations with good fault injection capabilities.

However, the plan's Phase 2 (advanced testing) remains incomplete. The three missing pieces—membership changes, BUGGIFY injection, and coverage tracking—are important for comprehensive distributed systems testing but are not blockers for current usage.

**Recommendation**: Current implementation is suitable for CI/CD. Prioritize Phase 2.1 (membership testing) as it's already partially implemented in non-madsim form and would provide valuable coverage for dynamic cluster scenarios.

---
