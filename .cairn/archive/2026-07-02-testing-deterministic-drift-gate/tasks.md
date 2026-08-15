## Phase 1: Drift comparison core

- [x] [serial] r[molten.testing.deterministic_drift.comparison_core] Add a pure comparator for paired workflow evidence summaries and explicit variance declarations.
- [x] [parallel] r[molten.testing.deterministic_drift.variance_declarations] Define canonical allowed-variance declarations with reason classes and fail-closed validation.

## Phase 2: Workflow gate

- [x] [serial] r[molten.testing.deterministic_drift.fresh_rerun_gate] Add an imperative shell that runs selected evidence workflows in fresh isolated state roots and feeds canonical refs to the comparator.
- [x] [parallel] r[molten.testing.deterministic_drift.release_workflows] Cover dogfood local-node, sealed repro verify/unpack, release bundle verify/promote/export verify, and deterministic VM child evidence where available.

## Phase 3: Fixtures, Nix, and docs

- [x] [parallel] r[molten.testing.deterministic_drift.negative_fixtures] Add positive same-input/same-ref fixtures and negative fixtures for injected drift, undeclared volatile fields, ambient state, and unstable rendered output.
- [x] [serial] r[molten.testing.deterministic_drift.gate_surface] Expose the drift gate through an explicit Nix check/app or release-readiness command.
- [x] [parallel] r[molten.testing.deterministic_drift.docs] Document what the drift gate compares, how variance is declared, and why retries are not accepted as drift fixes.
