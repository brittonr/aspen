# ADR-006: Deterministic Simulation Testing with madsim

**Status:** Accepted

**Date:** 2025-12-03

**Context:** Distributed systems are notoriously difficult to test. Traditional integration tests are flaky, slow, and fail to explore rare edge cases. Aspen requires a testing strategy that provides determinism, reproducibility, and comprehensive coverage of failure scenarios.

## Decision

Use **madsim** (MAterialize Deterministic SIMulator) for deterministic simulation testing of Aspen's distributed protocols, with automatic artifact capture for debugging failed runs.

## Implementation

### Simulation Infrastructure

Located in `src/simulation.rs`:

```rust
pub struct SimulationArtifact {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub seed: u64,                    // Deterministic seed
    pub test_name: String,
    pub events: Vec<String>,           // Event trace
    pub metrics: String,               // Transport/Raft metrics
    pub status: SimulationStatus,      // Passed/Failed
    pub error: Option<String>,
    pub duration_ms: u64,
}
```

### Artifact Builder Pattern

```rust
let mut artifact = SimulationArtifactBuilder::new("test_name", SEED)
    .start();

// Add events as simulation progresses
artifact = artifact.add_event("operation: description");

// Capture final state
let artifact = artifact.build();
artifact.persist("docs/simulations")?;
```

### Example Test

From `tests/hiqlite_flow.rs`:

```rust
#[madsim::test]
async fn hiqlite_flow_simulation_tracks_transport_metrics() {
    const SIMULATION_SEED: u64 = 42;
    let mut artifact_builder = SimulationArtifactBuilder::new(
        "hiqlite_flow_simulation_tracks_transport_metrics",
        SIMULATION_SEED
    ).start();

    // Simulate cluster operations
    artifact_builder = artifact_builder.add_event("init: cluster with nodes [1, 2, 3]");
    // ... perform operations ...

    artifact_builder = artifact_builder.add_event("writes: completed 5 payloads");

    let artifact = artifact_builder.build();
    artifact.persist("docs/simulations")?;
}
```

## Rationale

### Why madsim?

1. **Determinism**: Same seed produces identical execution every time
   - Replay failures with exact same sequence of events
   - Bisect to find regression-introducing commits
   - Seed-based debugging: `SIMULATION_SEED=42 cargo test`

2. **Time control**: Virtual clock enables time travel
   ```rust
   madsim::time::sleep(Duration::from_secs(10)).await;
   // Executes instantly in simulation time
   ```

3. **Comprehensive testing**: Explore many scenarios quickly
   - Run 1000s of simulations with different seeds in CI
   - Find rare race conditions through Monte Carlo testing
   - No need for real networks, real delays, or real failures

4. **Tiger Style alignment**:
   - **Fixed limits**: Simulation duration bounded (no infinite loops)
   - **Explicit types**: Seeds are `u64`, durations are `u64` milliseconds
   - **Fail fast**: Assertions fail immediately, captured in artifacts

### Artifact Collection

Artifacts serve multiple purposes:

1. **Debugging**: Complete record of what happened (seed, events, metrics, error)
2. **Regression testing**: Re-run failed simulations with same seed
3. **CI reporting**: Collect artifacts from failed CI runs
4. **Performance tracking**: Duration and metrics over time

Artifact naming: `{test_name}-seed{seed}-{timestamp}.json`

Example:
```
hiqlite_flow_simulation_tracks_transport_metrics-seed42-20251202-175136.json
```

### AspenRouter for In-Memory Multi-Node Tests

While not yet implemented, madsim enables simulating entire clusters in a single process:

```rust
// Future implementation concept
let mut router = AspenRouter::new();
router.create_node(1);  // Node 1 in virtual network
router.create_node(2);  // Node 2 in virtual network
router.inject_partition([1], [2]);  // Simulate network partition
router.deliver_messages();  // Process queued messages
```

This allows testing:
- Network partitions
- Message reordering
- Packet loss
- Node crashes and recovery

## Alternatives Considered

### 1. Jepsen

**Pros:**
- Industry standard for distributed systems testing
- Rich failure injection (network partitions, crashes, clock skew)
- Linearizability checking

**Cons:**
- Requires real nodes (VMs/containers)
- Slow (minutes per test)
- Not deterministic (can't replay with same seed)
- Complex setup (Docker, SSH, Clojure)

**Rejected:** Non-determinism and operational complexity make it unsuitable for fast iteration.

### 2. FoundationDB Simulation

**Pros:**
- Battle-tested (FoundationDB uses it extensively)
- Deterministic and fast
- Comprehensive fault injection

**Cons:**
- Not a standalone tool (deeply integrated with FDB codebase)
- C++ implementation (not compatible with Rust async)
- Would require building similar infrastructure from scratch

**Rejected:** madsim provides similar capabilities as a Rust library without reinventing the wheel.

### 3. Custom Chaos Testing Framework

**Pros:**
- Tailored to Aspen's exact needs
- Full control over failure injection

**Cons:**
- High development cost
- Easy to miss edge cases
- Non-deterministic without significant engineering effort

**Rejected:** Violates "zero technical debt" principle. madsim is proven and maintained.

### 4. Traditional Integration Tests

**Pros:**
- Simple to understand
- Tests real network behavior

**Cons:**
- Flaky (timing-dependent)
- Slow (real network delays)
- Limited failure coverage
- Hard to debug failures

**Rejected:** Insufficient for distributed systems. Complement with madsim, don't replace it.

## Consequences

### Positive

1. **Fast feedback**: 1000s of simulations in seconds
2. **Reproducible**: Same seed = same result (debug with confidence)
3. **Comprehensive**: Explore edge cases impossible with real networks
4. **Debuggable**: Artifacts capture complete execution trace
5. **CI-friendly**: Deterministic tests don't flake

### Negative

1. **Iroh incompatibility**: Real Iroh transport can't run in madsim (uses real syscalls)
   - **Mitigation**: Test protocol logic with mocked transport, integration tests for real Iroh

2. **Learning curve**: Madsim APIs differ from tokio APIs
   - **Mitigation**: Keep abstraction layer thin, document differences

3. **Coverage gaps**: Simulations can't catch all bugs (e.g., hardware-specific issues)
   - **Mitigation**: Combine with integration tests and production monitoring

### Testing Strategy

As documented in `docs/simulations/README.md`:

| Test Type | Environment | Purpose |
|-----------|-------------|---------|
| **Madsim simulations** | Virtual network | Protocol correctness, edge cases |
| **Integration tests** | Real Iroh | Network behavior, discovery |
| **Smoke tests** | Scripts | End-to-end workflows |
| **Production** | Real cluster | Real-world validation |

## Implementation Notes

### Current Limitations

From `tests/hiqlite_flow.rs`:

```rust
// Note: Iroh transport testing is incompatible with madsim (see plan.md Phase 3).
// This simulation tests the deterministic Hiqlite backend logic only.
```

Madsim uses virtual syscalls (`madsim::net::*`) incompatible with Iroh's real syscalls. We test:
- **Madsim**: Control plane logic, Raft state machine, KV operations
- **Integration tests**: Real Iroh networking, discovery, RPC

### Artifact Persistence

Artifacts are gitignored but collected by CI:

```rust
if let Ok(path) = artifact.persist("docs/simulations") {
    eprintln!("Simulation artifact persisted to: {}", path.display());
}
```

CI can upload artifacts from `docs/simulations/*.json` for failed builds.

### Seed-Based Debugging

When a simulation fails:

1. Note the seed from the failure message or artifact
2. Re-run with the same seed:
   ```bash
   SIMULATION_SEED=42 cargo test test_name
   ```
3. Add logging/breakpoints to debug
4. Fix bug
5. Verify fix with same seed

### Future Enhancements

Planned for `AspenRouter`:

1. **Network partition injection**: Split cluster into isolated groups
2. **Message reordering**: Test out-of-order delivery
3. **Clock skew**: Simulate clock drift between nodes
4. **Crash recovery**: Kill nodes and restart with persisted state
5. **Performance regression**: Track simulation duration over commits

## References

- Simulation infrastructure: `src/simulation.rs`
- Example test: `tests/hiqlite_flow.rs`
- Artifact documentation: `docs/simulations/README.md`
- madsim crate: https://docs.rs/madsim
- Tiger Style: `tigerstyle.md`
