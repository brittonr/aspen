# Tasks: cluster-deterministic-drift-gate

## Phase 1: Drift core inputs

- [x] [parallel] r[molten.testing.cluster_drift.lifecycle_rerun_gate] Define cluster lifecycle evidence summaries for manifest refs, node receipt refs, control refs, decisions, diagnostics, and allowed variance refs.
- [x] [parallel] r[molten.testing.cluster_drift.ambient_state_negatives] Add comparator diagnostics for undeclared child ref drift, ambient state drift, runtime path drift, ordering drift, unstable map ordering, retry-only success, and rendered-output-only changes.

## Phase 2: Gate shell

- [x] [serial] r[molten.testing.cluster_drift.lifecycle_rerun_gate] Add a shell command or Nix check that runs cluster lifecycle workflows in two fresh roots and feeds summaries to the pure comparator.
- [x] [serial] r[molten.testing.cluster_drift.lifecycle_rerun_gate] Support explicit variance declarations for temporary roots, runtime paths, store paths, diagnostic logs, and rendered output.

## Phase 3: Fixtures and validation

- [x] [parallel] r[molten.testing.cluster_drift.lifecycle_rerun_gate] Add positive drift fixtures for lifecycle roundtrip, already-running start, malformed-manifest denial, and manifest closure validation.
- [x] [parallel] r[molten.testing.cluster_drift.ambient_state_negatives] Add negative fixtures for changed child refs, undeclared volatile fields, ambient state, unstable ordering, retry-only success, and rendered-output-only success.
- [x] [serial] r[molten.testing.cluster_drift.lifecycle_rerun_gate] Run focused drift tests and update traceability coverage.
