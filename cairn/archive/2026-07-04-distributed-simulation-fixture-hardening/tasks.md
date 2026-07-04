# Tasks: distributed-simulation-fixture-hardening

## Phase 1: Direct simulator fixtures

- [x] [parallel] r[molten.testing.distributed_simulation.direct_fault_fixtures] Add named positive fixtures for delay, drop, reorder, rejoin, crash, restart, and duplicate suppression; assert pass decisions, stable receipt refs, stable final-state refs, committed operation ids, event kinds, and diagnostics.
- [x] [parallel] r[molten.testing.distributed_simulation.direct_fault_fixtures] Add named negative fixtures for stale evidence, corrupted receipts, resource pressure, unauthorized transport evidence, undeclared ambient state, and partitioned quorum; assert deny-before-side-effects, denied operation ids, empty commit sets for denied commands, and fault-specific diagnostics.
- [x] [serial] r[molten.testing.distributed_simulation.fixture_traceability] Add or refresh traceability markers so the direct fixture set contributes both positive and negative evidence for distributed simulation requirements.

## Phase 2: Distributed CI profile wiring

- [x] [parallel] r[molten.testing.distributed_ci.profile_wiring_evidence] Add profile metadata/gate fixtures that derive profile ids, command surfaces, expected artifact kinds, cost class, and release-review status from the configured distributed CI matrix.
- [x] [parallel] r[molten.testing.distributed_ci.profile_wiring_evidence] Add negative profile-wiring fixtures for missing configured profile, mismatched command surface, missing receipt ref, missing variance declaration, unavailable required profile, and retry-only pass.

## Phase 3: Validation and readback

- [x] [serial] r[molten.testing.distributed_simulation.direct_fault_fixtures] Run `cargo test --lib distributed` and record the pass/fail evidence in the implementation notes.
- [x] [serial] r[molten.testing.distributed_ci.profile_wiring_evidence] Run `cargo nextest run --profile deterministic` or explain the blocker if unavailable.
- [x] [serial] r[molten.testing.distributed_simulation.fixture_traceability] Update `docs/distributed-testing.md` or README readback if fixture names or commands change.

## Implementation notes

- Baseline before edits: `nix develop -c cargo test --lib distributed` passed with 11 tests.
- Focused validation after edits: `nix develop -c cargo test --lib distributed` passed with 18 tests.
- Deterministic validation after edits: `nix develop -c cargo nextest run --profile deterministic` passed with 826 tests.
