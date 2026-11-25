# Fabric simulation deltas

## ADDED Requirements

### Requirement: Simulation workload refs, failure fingerprints, differential evaluation, and replay comparisons are collision-free and complete
r[molten.fabric_simulation.collision_free_workload_and_complete_comparison] `LiveWorkload::workload_ref` and `failure_fingerprint` MUST use collision-free canonical framing with domain separation (and deduplicated invariants); `evaluate_live_differential` MUST validate `simulated_class` against `Unsupported` and `declared.permitted`; and `compare_replay` MUST compare full `selected` and `eligible` choice metadata.

#### Scenario: Embedded NUL bytes in ordering_key cannot collide workload_ref
- GIVEN a 1-step workload whose `ordering_key` attempts to mimic a 2-step serialized boundary with `\0` bytes
- WHEN `LiveWorkload::workload_ref` or `valid_ordering_key` runs
- THEN validation MUST reject control bytes and distinct step lists MUST never produce equal `workload_ref`s.

#### Scenario: Unsupported simulated_class fails differential check
- GIVEN `simulated_class == OutcomeClass::Unsupported` while `live_class` is permitted
- WHEN `evaluate_live_differential` evaluates the step
- THEN it MUST report `Incomplete` or `Failure` rather than `PermittedDivergence`.
