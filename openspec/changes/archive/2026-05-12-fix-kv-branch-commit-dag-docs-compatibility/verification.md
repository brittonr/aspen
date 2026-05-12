# Verification

## Evidence

- `evidence/docs-feature-reproduce.txt`: `cargo check -p aspen-docs --features commit-dag-federation` passed on current workspace; the prior failure is stale.
- `evidence/i5-downstream-branch-dag-metadata.json`: structured classification and representative consumer metadata.
- `evidence/i5-downstream-branch-dag-forbidden-grep.txt`: cargo-tree boundary transcript for `aspen-commit-dag`, `aspen-kv-branch`, no-default, and `commit-dag` feature graphs.
- `evidence/package-feature-checks.txt`: branch/DAG package and feature compile checks.
- `docs/crate-extraction/kv-branch-commit-dag.md`, `docs/crate-extraction.md`, and `docs/crate-extraction/policy.ncl`: readiness updated to `extraction-ready-in-workspace` while publishable/repo-split remains blocked on license policy.

## Task Coverage

- Task: Capture the current `cargo check -p aspen-docs --features commit-dag-federation` failure transcript.
  - Evidence: `evidence/docs-feature-reproduce.txt`; current transcript is passing, so no current failure was reproduced.
- Task: Classify the failure as feature wiring, dependency version skew, RNG API mismatch, or branch/DAG API regression.
  - Evidence: `evidence/i5-downstream-branch-dag-metadata.json`; classified as stale compatibility blocker, not an active wiring/skew/RNG/API regression.
- Task: Apply the smallest compatibility fix that keeps branch/DAG reusable graphs clean.
  - Evidence: no code fix required after reproduction; docs/policy readiness updated only after current compatibility evidence passed.
- Task: Capture passing docs feature evidence and rerun branch/DAG package/feature checks.
  - Evidence: `evidence/docs-feature-reproduce.txt` and `evidence/package-feature-checks.txt`.
- Task: Rerun `scripts/check-crate-extraction-readiness.rs --candidate-family kv-branch-commit-dag` with fresh evidence.
  - Evidence: `evidence/kv-branch-commit-dag-readiness.md` and `evidence/kv-branch-commit-dag-readiness.json`.
- Task: Update `docs/crate-extraction/kv-branch-commit-dag.md`, inventory, and verification notes.
  - Evidence: changed docs plus this verification index.
- Task: Run strict OpenSpec validation and `git diff --check`.
  - Evidence: command list below.

## Commands

- `cargo check -p aspen-docs --features commit-dag-federation`
- `cargo check -p aspen-commit-dag`
- `cargo check -p aspen-kv-branch`
- `cargo check -p aspen-kv-branch --no-default-features`
- `cargo check -p aspen-kv-branch --features commit-dag`
- `cargo tree -p aspen-commit-dag --edges normal`
- `cargo tree -p aspen-kv-branch --edges normal`
- `cargo tree -p aspen-kv-branch --no-default-features --edges normal`
- `cargo tree -p aspen-kv-branch --features commit-dag --edges normal`
- `nix develop -c cargo -q -Zscript scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family kv-branch-commit-dag --output-json openspec/changes/fix-kv-branch-commit-dag-docs-compatibility/evidence/kv-branch-commit-dag-readiness.json --output-markdown openspec/changes/fix-kv-branch-commit-dag-docs-compatibility/evidence/kv-branch-commit-dag-readiness.md`
- `openspec validate fix-kv-branch-commit-dag-docs-compatibility --strict`
- `openspec validate --all --strict --json`
- `git diff --check`
