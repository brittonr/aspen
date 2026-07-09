# Tasks: replay-coverage-readiness

## Phase A: Matrix core

- [ ] [serial] r[molten.determinism.replay_coverage.matrix] Define replay coverage row DTOs and pure matrix validation.
- [ ] [serial] r[molten.determinism.replay_coverage.matrix] Validate unique subsystem/workflow rows, required positive evidence, required negative evidence, caveats, and stale refs.
- [ ] [parallel] r[molten.determinism.replay_coverage.non_replayable_exclusions] Classify diagnostic-only and non-replayable rows so rendered pass status cannot count as deterministic replay evidence.

## Phase B: Subsystem evidence and readiness shell

- [ ] [serial] r[molten.determinism.replay_coverage.subsystem_smoke] Add replay smoke rows for harness report replay, node-control workflow bundle, job worker scheduling, coordination duplicate operations, remote dataspace delivery logs, vat replay, retention remote-clearance, and dogfood release replay evidence.
- [ ] [parallel] r[molten.determinism.replay_coverage.release_readiness_summary] Emit canonical replay readiness summaries while preserving evidence-only caveats.
- [ ] [parallel] r[molten.catalog.replay_coverage.matrix_search] Add catalog classifications for replay coverage matrices.
- [ ] [parallel] r[molten.catalog.replay_coverage.readonly] Add read-only replay coverage MCP/catalog readback tests or fixtures.

## Phase C: Tests and docs

- [ ] [serial] r[molten.determinism.replay_coverage.tests] Add positive complete-matrix tests.
- [ ] [serial] r[molten.determinism.replay_coverage.tests] Add negative tests for missing positive evidence, missing negative evidence, stale refs, duplicate rows, and diagnostic-only exclusion.
- [ ] [serial] r[molten.determinism.replay_coverage.tests] Document replay coverage/readiness output and evidence-only limits.
