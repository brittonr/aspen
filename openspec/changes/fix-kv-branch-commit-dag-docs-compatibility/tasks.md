## Phase 1: Reproduce

- [ ] [serial] Capture the current `cargo check -p aspen-docs --features commit-dag-federation` failure transcript.
- [ ] [depends:reproduce] Classify the failure as feature wiring, dependency version skew, RNG API mismatch, or branch/DAG API regression.

## Phase 2: Fix and evidence

- [ ] [depends:classification] Apply the smallest compatibility fix that keeps branch/DAG reusable graphs clean.
- [ ] [depends:fix] Capture passing docs feature evidence and rerun branch/DAG package/feature checks.
- [ ] [depends:fix] Rerun `scripts/check-crate-extraction-readiness.rs --candidate-family kv-branch-commit-dag` with fresh evidence.

## Phase 3: Closeout

- [ ] [depends:evidence] Update `docs/crate-extraction/kv-branch-commit-dag.md`, inventory, and verification notes.
- [ ] [depends:closeout] Run strict OpenSpec validation and `git diff --check`.
