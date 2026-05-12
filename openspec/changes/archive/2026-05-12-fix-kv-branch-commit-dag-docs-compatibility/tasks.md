## Phase 1: Reproduce

- [x] [serial] Capture the current `cargo check -p aspen-docs --features commit-dag-federation` failure transcript.
- [x] [depends:reproduce] Classify the failure as feature wiring, dependency version skew, RNG API mismatch, or branch/DAG API regression.

## Phase 2: Fix and evidence

- [x] [depends:classification] Apply the smallest compatibility fix that keeps branch/DAG reusable graphs clean.
- [x] [depends:fix] Capture passing docs feature evidence and rerun branch/DAG package/feature checks.
- [x] [depends:fix] Rerun `scripts/check-crate-extraction-readiness.rs --candidate-family kv-branch-commit-dag` with fresh evidence.

## Phase 3: Closeout

- [x] [depends:evidence] Update `docs/crate-extraction/kv-branch-commit-dag.md`, inventory, and verification notes.
- [x] [depends:closeout] Run strict OpenSpec validation and `git diff --check`.
