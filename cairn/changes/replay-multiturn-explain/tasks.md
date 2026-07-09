# Tasks: replay-multiturn-explain

## Phase A: Pure comparison core

- [ ] [serial] r[molten.determinism.multiturn_replay.core] Define replay summary DTOs and pure comparison output types for run identity, turn refs, boundary refs, effect refs, output refs, and final-state refs.
- [ ] [serial] r[molten.determinism.multiturn_replay.core] Implement ordered multi-turn comparison that returns pass or the first semantic divergence without performing I/O.
- [ ] [parallel] r[molten.determinism.multiturn_replay.first_divergence_path] Extend first-divergence DTOs with path metadata and redaction status.

## Phase B: CLI and manifest-backed debug

- [ ] [serial] r[molten.determinism.multiturn_replay.explain_cli] Add replay compare CLI shell that reads artifacts, invokes the pure comparator, and writes canonical verify receipts.
- [ ] [serial] r[molten.determinism.multiturn_replay.explain_cli] Add replay explain CLI shell that writes explain receipts before rendering summaries.
- [ ] [parallel] r[molten.determinism.multiturn_replay.merkle_prefix] Add manifest-backed prefix comparison and partial-fetch receipt binding for large traces.

## Phase C: Tests and docs

- [ ] [serial] r[molten.determinism.multiturn_replay.tests] Add positive tests for stable multi-turn replay across fresh and replay runs.
- [ ] [serial] r[molten.determinism.multiturn_replay.tests] Add negative tamper tests for every supported semantic boundary and path metadata field.
- [ ] [serial] r[molten.determinism.multiturn_replay.tests] Add explain CLI malformed-input and redaction-safe rendering tests.
- [ ] [serial] r[molten.determinism.multiturn_replay.tests] Document compare/explain commands and evidence-only caveats.
