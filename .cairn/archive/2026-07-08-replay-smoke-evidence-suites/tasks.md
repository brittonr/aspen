# Tasks: replay-smoke-evidence-suites

## Replay smoke core

- [x] [serial] r[molten.testing.replay_smoke.all_evidence_suites] Define replay smoke eligibility for evidence-bearing harness suites and non-replayable diagnostic suites.
- [x] [serial] r[molten.testing.replay_smoke.fresh_rerun] Implement a pure comparison helper for fresh run, replay run, and second fresh run canonical refs.

## Suite integration

- [x] [parallel] r[molten.testing.replay_smoke.all_evidence_suites] Apply the smoke helper to representative harness report, CLI report, distributed simulation, and dogfood diagnostic surfaces.
- [x] [parallel] r[molten.testing.replay_smoke.non_replayable_excluded] Ensure exploratory, live-only, VM-unavailable, and diagnostic-only runs are excluded from deterministic pass gates with explicit diagnostics.

## Tests and validation

- [x] [parallel] r[molten.testing.replay_smoke.fresh_rerun] Add positive tests for stable run/replay/fresh-rerun refs and negative tests for missing effect log, changed effect response, ambient-state marker, and non-replayable pass misuse.
- [x] [serial] r[molten.testing.replay_smoke.all_evidence_suites] Add replay-smoke coverage to the evidence matrix or traceability output.
- [x] [serial] r[molten.testing.replay_smoke.non_replayable_excluded] Run focused replay smoke tests and Cairn validation; record unsupported surfaces as explicit follow-ups.
