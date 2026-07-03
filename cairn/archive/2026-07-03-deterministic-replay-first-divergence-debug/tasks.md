# Tasks: deterministic-replay-first-divergence-debug

## Phase 1: Replay core

- [x] [serial] r[molten.determinism.replay_first_divergence.verify_receipt] Add replay verify receipt fields that bind expected and actual comparison refs plus divergence kind and first-divergence ref.
- [x] [serial] r[molten.determinism.replay_first_divergence.debug_record] Add canonical first-divergence records for semantic mismatch classes.
- [x] [parallel] r[molten.determinism.replay_first_divergence.recorded_effects_only] Ensure missing recorded effect responses deny as live-effect replay attempts.

## Phase 2: CLI and fixtures

- [x] [parallel] r[molten.determinism.replay_first_divergence.cli_fixture] Extend replay-fixture CLI support for tampered fixture variants and receipt output.
- [x] [parallel] r[molten.determinism.replay_first_divergence.debug_record] Store manifest-backed first-divergence debug artifacts where replay bundles need partial debug fetch support.

## Phase 3: Tests and evidence

- [x] [serial] r[molten.determinism.replay_first_divergence.tests] Add positive unchanged replay verification tests.
- [x] [serial] r[molten.determinism.replay_first_divergence.tests] Add negative tamper matrix tests for every supported divergence kind.
- [x] [serial] r[molten.determinism.replay_first_divergence.tests] Add CLI harness coverage proving tampered fixtures deny and receipt files bind first-divergence refs.
- [x] [serial] r[molten.determinism.replay_first_divergence.tests] Run `cargo test replay -- --nocapture`, `cargo test`, and the clippy gate after `clippy-gate-cleanup` is complete.
