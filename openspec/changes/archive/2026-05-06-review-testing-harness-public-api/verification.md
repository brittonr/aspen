# Verification: review-testing-harness-public-api

## Implementation Evidence

- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/testing-harness.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.lock`
- Changed file: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml`
- Changed file: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/src/lib.rs`
- Changed file: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/Cargo.lock`
- Changed file: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/Cargo.toml`
- Changed file: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/src/lib.rs`
- Changed file: `openspec/changes/review-testing-harness-public-api/specs/testing-harness-extraction/spec.md`
- Changed file: `openspec/changes/review-testing-harness-public-api/proposal.md`
- Changed file: `openspec/changes/review-testing-harness-public-api/design.md`
- Changed file: `openspec/changes/review-testing-harness-public-api/tasks.md`
- Changed file: `openspec/changes/review-testing-harness-public-api/verification.md`

- Public API ownership, canonical reusable root, adapter ownership, allowed Tokio utility scope, and readiness state are recorded in `docs/crate-extraction/testing-harness.md`.
- The extraction inventory and Nickel policy promote the testing harness family to `extraction-ready-in-workspace` while keeping publishable/repo-split states blocked on license/publication policy.
- `fixtures/testing-harness-core-smoke` is an independent downstream fixture that imports `aspen-testing-core` directly and exercises reusable deterministic helpers.
- `fixtures/testing-harness-bootstrap-negative` proves madsim/network/patchbay adapter crates are unavailable without explicit adapter dependencies.
- Evidence files under `evidence/` cover compatibility checks, downstream metadata, dependency graph/forbidden runtime boundary, rerun smoke output, and readiness checker output.

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `review-testing-harness-public-api`.
  - Evidence: `openspec/changes/review-testing-harness-public-api/proposal.md`
  - Evidence: `openspec/changes/review-testing-harness-public-api/design.md`
  - Evidence: `openspec/changes/review-testing-harness-public-api/specs/testing-harness-extraction/spec.md`
  - Evidence: `openspec/changes/review-testing-harness-public-api/tasks.md`
- [x] Inventory `aspen-testing-core` APIs and classify adapter-owned helpers.
  - Evidence: `docs/crate-extraction/testing-harness.md`
  - Evidence: `docs/crate-extraction.md`
  - Evidence: `docs/crate-extraction/policy.ncl`
- [x] Add/update reusable smoke fixture, negative adapter-boundary fixture, and extraction metadata.
  - Evidence: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml`
  - Evidence: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/src/lib.rs`
  - Evidence: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/Cargo.toml`
  - Evidence: `openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/src/lib.rs`
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-downstream-metadata.json`
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-forbidden-boundary.txt`
- [x] Run fixture builds, metadata, negative checks, patchbay/harness compatibility, and update readiness docs.
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-compatibility.txt`
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-core-smoke-rerun.txt`
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.md`
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.json`
- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
  - Evidence: `openspec/changes/review-testing-harness-public-api/verification.md`
  - Evidence: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.md`
- [x] Sync/archive only after every implementation/evidence task is complete.
  - Evidence: `openspec/changes/review-testing-harness-public-api/tasks.md`
  - Evidence: `openspec/changes/review-testing-harness-public-api/verification.md`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| fixture | `cargo test --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml` | pass | `evidence/testing-harness-core-smoke-rerun.txt` | Exercises direct downstream imports from reusable testing core without cluster/bootstrap adapters. | Add a published-crate fixture after license policy is decided. |
| negative | `cargo check --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/Cargo.toml` | expected fail | `evidence/testing-harness-compatibility.txt` | Proves madsim/network/patchbay adapter crates are unavailable without explicit dependencies. | Add compile-fail UI tests if the adapter boundary becomes source-level policy. |
| graph | `cargo tree --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml -e normal` plus forbidden scan | pass | `evidence/testing-harness-forbidden-boundary.txt` | Covers root Aspen, cluster runtime, RPC handlers, transport/Raft runtime, Iroh runtime, patchbay, madsim/turmoil, and adapter exclusions. | Full workspace dependency audit is broader than this API-review drain. |
| compatibility | `cargo check -p aspen-testing-core` and adapter package checks | pass | `evidence/testing-harness-compatibility.txt` | Confirms reusable root and explicit adapters compile after the readiness decision. | Run patchbay profile suites before changing adapter code. |
| readiness | `scripts/check-crate-extraction-readiness.rs --candidate-family testing-harness ...` | pass | `evidence/testing-harness-readiness.md` | Covers policy/inventory/docs readiness contract for the virtual testing-harness family. | Full publication package verification remains blocked on license policy. |
| format | `git diff --check` | pass | closeout transcript | Markdown/TOML/Rust fixture diffs are whitespace-clean. | Repo-wide lint is outside this docs/evidence slice. |

## Verification Commands

- Artifact: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-compatibility.txt`
- Artifact: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-core-smoke-rerun.txt`
- Artifact: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-forbidden-boundary.txt`
- Artifact: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-downstream-metadata.json`
- Artifact: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.md`
- Artifact: `openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.json`

```bash
cargo test --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml
cargo metadata --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml --format-version 1 --no-deps > openspec/changes/review-testing-harness-public-api/evidence/testing-harness-downstream-metadata.json
cargo check --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-bootstrap-negative/Cargo.toml # expected failure: unresolved adapter crates
cargo check -p aspen-testing-core
cargo check -p aspen-testing-fixtures -p aspen-testing-madsim -p aspen-testing-network -p aspen-testing-patchbay
cargo tree --manifest-path openspec/changes/review-testing-harness-public-api/fixtures/testing-harness-core-smoke/Cargo.toml -e normal
scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family testing-harness --output-json openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.json --output-markdown openspec/changes/review-testing-harness-public-api/evidence/testing-harness-readiness.md
openspec validate review-testing-harness-public-api --strict
scripts/openspec-preflight.sh review-testing-harness-public-api
git diff --check
```
