# Simulation Artifacts

This directory contains artifacts from deterministic simulation runs using `madsim`. Each artifact captures the complete state of a simulation run including:

- Deterministic seed used
- Event trace showing the sequence of operations
- Transport metrics snapshots
- Test outcome (passed/failed)
- Simulation duration
- Error details (if failed)

## Artifact Format

Each artifact is stored as a JSON file with the naming pattern:

```
{test_name}-seed{seed}-{timestamp}.json
```

Example:

```
madsim_multi_node_test-seed42-20251202-175136.json
```

## Artifact Schema

```json
{
  "run_id": "unique-identifier",
  "timestamp": "ISO-8601-timestamp",
  "seed": 42,
  "test_name": "name-of-test",
  "events": [
    "event1",
    "event2"
  ],
  "metrics": "prometheus-style-metrics",
  "status": "Passed|Failed",
  "error": "optional-error-message",
  "duration_ms": 123
}
```

## Purpose

These artifacts serve multiple purposes:

1. **Debugging**: When a simulation fails, the artifact provides a complete record of what happened
2. **Regression Testing**: Failed simulations can be re-run with the same seed to reproduce issues
3. **CI Reporting**: CI systems can collect and publish these artifacts for visibility
4. **Performance Tracking**: Duration and metrics help track simulation performance over time

## Usage in Tests

To generate artifacts in your simulation tests:

```rust
use aspen::simulation::SimulationArtifactBuilder;

#[madsim::test]
async fn my_simulation() {
    const SEED: u64 = 42;
    let mut artifact = SimulationArtifactBuilder::new("my_simulation", SEED)
        .start();

    // Add events as the simulation progresses
    artifact = artifact.add_event("operation: description");

    // Capture metrics
    let metrics = get_metrics();
    artifact = artifact.with_metrics(metrics);

    // Build and persist
    let artifact = artifact.build();
    if let Ok(path) = artifact.persist("docs/simulations") {
        eprintln!("Artifact saved to: {}", path.display());
    }
}
```

## CI Integration

The flake checks will automatically collect artifacts from failed simulation runs and publish them as build outputs. This ensures that debugging information is available even in CI environments.

## Gitignore

Simulation artifacts are gitignored by default to avoid committing large numbers of test runs. Only explicitly curated artifacts that demonstrate specific failure modes should be committed to the repository.
