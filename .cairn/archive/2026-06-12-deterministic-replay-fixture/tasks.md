# Tasks: deterministic-replay-fixture

- [x] [serial] r[molten.determinism.replay_fixture.identity] Define `deterministic-run-identity-v1` with artifact, dependency closure, initial state, schema, policy, capability/revocation, handler profile, seed/effect-log, and runtime/tool version refs.
- [x] [serial] r[molten.determinism.replay_fixture.record] Emit a bounded `deterministic-fixture-record-v1` binding run identity, ordered turn journals, effect log refs, output refs, and final state ref.
- [x] [serial] r[molten.determinism.replay_fixture.verify] Implement replay verification that compares scheduler/input/effect/action/receipt/output/after-state boundaries in order and emits `deterministic-replay-verify-v1`.
- [x] [serial] r[molten.determinism.replay_fixture.first_divergence] Emit `deterministic-first-divergence-v1` for the first mismatched semantic boundary with safe expected/actual refs and diagnostics.
- [x] [parallel] r[molten.determinism.replay_fixture.no_live_effects] Ensure replay-profile fixture verification injects recorded responses and denies live external clock, random, filesystem, network, environment, process, and storage observations.
- [x] [parallel] r[molten.determinism.replay_fixture.cli] Add `molten test replay-fixture` commands for record, verify, tamper/divergence, and show/readback flows.
- [x] [parallel] r[molten.determinism.replay_fixture.tests] Add tests for pass replay, changed identity, changed effect response, changed policy/receipt boundary, live-effect denial, and canonical readback.
