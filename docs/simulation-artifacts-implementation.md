# Simulation Artifacts Implementation

## Overview

This document describes the simulation artifact capture system implemented for Aspen's deterministic distributed system testing.

## Motivation

When running deterministic simulations with `madsim`, it's critical to capture complete information about each test run for debugging and regression analysis. The simulation artifact system provides a structured way to:

1. **Debug failures**: Complete event traces and metrics make it easy to understand what went wrong
2. **Reproduce issues**: Deterministic seeds allow exact replay of failed scenarios
3. **Track performance**: Duration and metrics help identify performance regressions
4. **CI visibility**: Artifacts are automatically collected in CI for failed test analysis

## Implementation

### Core Module: `src/simulation.rs`

The simulation module provides:

- `SimulationArtifact`: A complete snapshot of a simulation run
- `SimulationArtifactBuilder`: A fluent builder for constructing artifacts
- JSON serialization for persistence
- Automatic timestamp and duration tracking

### Key Features

**Captured Data:**

- Deterministic seed (for reproducibility)
- Complete event trace (ordered list of operations)
- Transport metrics snapshots
- Test status (passed/failed)
- Error messages (for failures)
- Simulation duration in milliseconds
- Unique run ID and timestamp

**Builder Pattern:**

```rust
let artifact = SimulationArtifactBuilder::new("test_name", seed)
    .start()
    .add_event("operation: description")
    .add_events(trace_vec)
    .with_metrics(metrics_snapshot)
    .build();

artifact.persist("docs/simulations")?;
```

### Integration Points

**Test Integration:**
The `tests/hiqlite_flow.rs` test demonstrates the pattern:

- Create builder at test start
- Add events as operations complete
- Capture metrics from transport layer
- Include leader churn traces
- Persist artifact at test end

**CI Integration:**
The `flake.nix` nextest check includes:

```nix
postInstall = ''
  if [ -d docs/simulations ]; then
    mkdir -p $out/simulations
    cp -r docs/simulations/*.json $out/simulations/ 2>/dev/null || true
  fi
'';
```

This ensures artifacts are collected even when tests fail.

## File Structure

```
docs/simulations/
├── README.md                    # Documentation
└── *.json                       # Artifacts (gitignored)
```

### Artifact Naming Convention

```
{test_name}-seed{seed}-{timestamp}.json
```

Example:

```
hiqlite_flow_simulation_tracks_transport_metrics-seed42-20251202-175136.json
```

## Example Artifact

```json
{
  "run_id": "hiqlite_flow_simulation_tracks_transport_metrics-seed42-20251202-175136",
  "timestamp": "2025-12-02T17:51:36.080290850Z",
  "seed": 42,
  "test_name": "hiqlite_flow_simulation_tracks_transport_metrics",
  "events": [
    "init: cluster with nodes [1, 2, 3]",
    "add-learner: node 4",
    "change-membership: promote node 4 to voter",
    "writes: completed 5 payloads",
    "read: verified payload",
    "transport: iroh cluster online",
    "leader-elected:1",
    "leader-elected:2",
    "leader-elected:3"
  ],
  "metrics": "# TYPE magicsock_recv_datagrams counter\nmagicsock_recv_datagrams{node=\"hiqlite-sim\"} 42\n# EOF\n",
  "status": "Passed",
  "error": null,
  "duration_ms": 64
}
```

## Usage Guidelines

### When to Create Artifacts

Create artifacts for:

- All deterministic `madsim` tests
- Tests that involve multiple nodes or complex coordination
- Tests that include failure injection or network partitions
- Long-running simulation scenarios

### What to Capture

**Event Traces:**

- Cluster membership changes (init, add learner, change membership)
- Write/read operations
- Leader elections and failovers
- Transport state changes (online, offline)
- Failure injections (partition, crash, delay)

**Metrics:**

- Transport counters (packets sent/received)
- Raft metrics (term, commit index, applied index)
- Performance counters (latency, throughput)
- Resource usage (memory, CPU)

### Best Practices

1. **Descriptive events**: Use clear, structured event descriptions
2. **Consistent format**: Follow the pattern `"category: description"`
3. **Meaningful seeds**: Use seeds that have special significance (42, 0, etc.)
4. **Error context**: When tests fail, include detailed error information
5. **Selective commits**: Only commit artifacts that demonstrate specific issues

## Future Enhancements

Potential improvements:

- Automatic seed sweep for discovering edge cases
- Artifact comparison tools for regression detection
- Web UI for browsing artifact history
- Integration with Antithesis-style coverage tracking
- Automatic issue filing for new failure modes

## Related Files

- `src/simulation.rs` - Core implementation
- `tests/hiqlite_flow.rs` - Example usage
- `docs/simulations/README.md` - User documentation
- `flake.nix` - CI integration
- `.gitignore` - Artifact exclusion rules

## Status

✅ Complete - Phase 2 milestone achieved

See `plan.md` for the next steps in the Aspen reboot roadmap.
