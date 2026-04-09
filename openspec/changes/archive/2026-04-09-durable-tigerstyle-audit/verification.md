# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/tigerstyle-audit.py`
- Changed file: `tools/tigerstyle/fixtures/trait_declaration_false_positive.rs`
- Changed file: `tools/tigerstyle/test_tigerstyle_audit.py`
- Changed file: `openspec/changes/durable-tigerstyle-audit/audit-report.md`
- Changed file: `crates/aspen-ci-handler/src/handler/pipeline.rs`
- Changed file: `crates/aspen-auth/src/capability.rs`
- Changed file: `crates/aspen-auth/src/tests.rs`
- Changed file: `crates/aspen-ci-core/src/verified/pipeline.rs`
- Changed file: `crates/aspen-forge/src/verified/ref_validation.rs`
- Changed file: `crates/aspen-cluster/src/gossip/discovery/helpers.rs`
- Changed file: `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs`
- Changed file: `crates/aspen-cluster/src/gossip/discovery/mod.rs`
- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-core-essentials-handler/src/coordination.rs`
- Changed file: `crates/aspen-forge-handler/Cargo.toml`
- Changed file: `crates/aspen-forge-handler/src/executor.rs`
- Changed file: `openspec/changes/durable-tigerstyle-audit/verification.md`
- Changed file: `openspec/changes/durable-tigerstyle-audit/tasks.md`

## Task Coverage

- [x] Write `audit-report.md` in this change with the current scan scope, methodology, known scanner limitations, ranked hotspot table, and an explicit status line (`audit-only`, `remediation in progress`, or `remediation complete`).
  - Evidence: `openspec/changes/durable-tigerstyle-audit/audit-report.md`, `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.json`

- [x] Add a repo-local Tiger Style scanner command that can be rerun by reviewers and emits a machine-readable hotspot inventory.
  - Evidence: `scripts/tigerstyle-audit.py`, `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.json`, `openspec/changes/durable-tigerstyle-audit/evidence/phase1-audit-diff.patch`

- [x] Save the initial scanner transcript under `evidence/` and reference it from `audit-report.md`.
  - Evidence: `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.json`, `openspec/changes/durable-tigerstyle-audit/audit-report.md`

- [x] Add a committed regression fixture that reproduces the trait-declaration false positive described in the audit notes.
  - Evidence: `tools/tigerstyle/fixtures/trait_declaration_false_positive.rs`, `tools/tigerstyle/test_tigerstyle_audit.py`, `openspec/changes/durable-tigerstyle-audit/evidence/phase1-audit-diff.patch`

- [x] Verify the scanner ignores declaration-only signatures ending in `;` and only counts functions whose signatures reach a body `{` before a top-level `;`.
  - Evidence: `tools/tigerstyle/test_tigerstyle_audit.py`, `openspec/changes/durable-tigerstyle-audit/evidence/scanner-regression.txt`

- [x] Save the regression run output under `evidence/` so the napkin rule has in-repo support.
  - Evidence: `openspec/changes/durable-tigerstyle-audit/evidence/scanner-regression.txt`, `openspec/changes/durable-tigerstyle-audit/audit-report.md`

- [x] Add characterization coverage for `crates/aspen-ci-handler/src/handler/pipeline.rs:handle_trigger_pipeline` before refactoring.
  - Evidence: `crates/aspen-ci-handler/src/handler/pipeline.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/ci-handler-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`

- [x] Refactor `handle_trigger_pipeline` into smaller helpers, moving deterministic planning logic out of the main async shell and adding assertions around parsed IDs, ref resolution, and checkout/cleanup invariants.
  - Evidence: `crates/aspen-ci-handler/src/handler/pipeline.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/ci-handler-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`

- [x] Add characterization coverage for `crates/aspen-auth/src/capability.rs:Capability::authorizes` and `Capability::contains` before refactoring.
  - Evidence: `crates/aspen-auth/src/tests.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/auth-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`

- [x] Split `Capability::authorizes` and `Capability::contains` into smaller pure helpers grouped by capability family, preserving behavior and making the decision table easier to test.
  - Evidence: `crates/aspen-auth/src/capability.rs`, `crates/aspen-auth/src/tests.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/auth-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`

- [x] Reduce `usize` leakage in the first verified/public hotspot set:
  - Evidence: `crates/aspen-ci-core/src/verified/pipeline.rs`, `crates/aspen-forge/src/verified/ref_validation.rs`, `crates/aspen-cluster/src/gossip/discovery/helpers.rs`, `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs`, `crates/aspen-cluster/src/gossip/discovery/mod.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/ci-core-verified-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/forge-ref-validation-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/cluster-discovery-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.json`, `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`

- [x] Re-run the Tiger Style audit and update `audit-report.md` with before/after counts for the first remediation slice.
  - Evidence: `openspec/changes/durable-tigerstyle-audit/audit-report.md`, `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.json`

- [x] Add characterization coverage for `crates/aspen-core-essentials-handler/src/coordination.rs:handle` before extraction.
  - Evidence: `crates/aspen-core-essentials-handler/src/coordination.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/core-essentials-coordination-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/phase4-dispatcher-diff.patch`, `openspec/changes/durable-tigerstyle-audit/evidence/review-fixes-diff.patch`

- [x] Break `crates/aspen-core-essentials-handler/src/coordination.rs:handle` into bounded request-family helpers with shared response builders to reduce repetition and centralize control flow.
  - Evidence: `crates/aspen-core-essentials-handler/src/coordination.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/core-essentials-coordination-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/phase4-dispatcher-diff.patch`

- [x] Add characterization coverage for `crates/aspen-forge-handler/src/executor.rs:execute` before extraction.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/forge-handler-executor-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/phase4-dispatcher-diff.patch`, `openspec/changes/durable-tigerstyle-audit/evidence/review-fixes-diff.patch`

- [x] Break `crates/aspen-forge-handler/src/executor.rs:execute` into per-request helpers or executor submodules so the top-level dispatcher stays under the Tiger Style function-length target.
  - Evidence: `crates/aspen-forge-handler/src/executor.rs`, `openspec/changes/durable-tigerstyle-audit/evidence/forge-handler-executor-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-phase4-scan.json`, `openspec/changes/durable-tigerstyle-audit/evidence/phase4-dispatcher-diff.patch`, `openspec/changes/durable-tigerstyle-audit/evidence/review-fixes-diff.patch`

- [x] Save all command transcripts, diffs, and targeted test output under `evidence/`.
  - Evidence: `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.json`, `openspec/changes/durable-tigerstyle-audit/evidence/scanner-regression.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/ci-handler-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/auth-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/ci-core-verified-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/forge-ref-validation-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/cluster-discovery-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.json`, `openspec/changes/durable-tigerstyle-audit/evidence/core-essentials-coordination-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/forge-handler-executor-tests.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-phase4-scan.txt`, `openspec/changes/durable-tigerstyle-audit/evidence/post-phase4-scan.json`, `openspec/changes/durable-tigerstyle-audit/evidence/phase1-audit-diff.patch`, `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`, `openspec/changes/durable-tigerstyle-audit/evidence/phase4-dispatcher-diff.patch`, `openspec/changes/durable-tigerstyle-audit/evidence/review-fixes-diff.patch`

- [x] Add `verification.md` once any task is checked, using verbatim task text and repo-relative evidence paths.
  - Evidence: `openspec/changes/durable-tigerstyle-audit/verification.md`

- [x] Run `scripts/openspec-preflight.sh durable-tigerstyle-audit` before requesting done review or checking the final task.
  - Evidence: `openspec/changes/durable-tigerstyle-audit/evidence/openspec-preflight.txt`

## Review Scope Snapshot

### `git diff -- scripts/tigerstyle-audit.py tools/tigerstyle/fixtures/trait_declaration_false_positive.rs tools/tigerstyle/test_tigerstyle_audit.py openspec/changes/durable-tigerstyle-audit/audit-report.md`

- Status: captured
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/phase1-audit-diff.patch`

### `git diff -- crates/aspen-ci-handler/src/handler/pipeline.rs crates/aspen-auth/src/capability.rs crates/aspen-auth/src/tests.rs crates/aspen-ci-core/src/verified/pipeline.rs crates/aspen-forge/src/verified/ref_validation.rs crates/aspen-cluster/src/gossip/discovery/helpers.rs crates/aspen-cluster/src/gossip/discovery/lifecycle.rs crates/aspen-cluster/src/gossip/discovery/mod.rs openspec/changes/durable-tigerstyle-audit/audit-report.md`

- Status: captured
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/remediation-slice-diff.patch`

### `git diff -- crates/aspen-core-essentials-handler/src/coordination.rs crates/aspen-forge-handler/src/executor.rs openspec/changes/durable-tigerstyle-audit/audit-report.md`

- Status: captured
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/phase4-dispatcher-diff.patch`

### `git diff -- crates/aspen-core-essentials-handler/src/coordination.rs crates/aspen-forge-handler/Cargo.toml crates/aspen-forge-handler/src/executor.rs openspec/changes/durable-tigerstyle-audit/audit-report.md`

- Status: captured
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/review-fixes-diff.patch`

## Verification Commands

### `./scripts/tigerstyle-audit.py crates`

- Status: pass with 5 known parse errors captured for follow-up
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.txt`

### `./scripts/tigerstyle-audit.py --pretty --json crates`

- Status: pass with 5 known parse errors captured for follow-up
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.json`

### `python3 tools/tigerstyle/test_tigerstyle_audit.py`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/scanner-regression.txt`

### `./scripts/tigerstyle-audit.py --pretty --json --line-limit 5 tools/tigerstyle/fixtures/trait_declaration_false_positive.rs`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/scanner-regression.txt`

### `cargo test -p aspen-auth`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/auth-tests.txt`

### `cargo test -p aspen-ci-handler --features forge,blob pipeline::tests -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/ci-handler-tests.txt`

### `cargo test -p aspen-ci-core verified::pipeline`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/ci-core-verified-tests.txt`

### `cargo test -p aspen-forge ref_validation`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/forge-ref-validation-tests.txt`

### `cargo test -p aspen-cluster calculate_backoff_duration -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/cluster-discovery-tests.txt`

### `./scripts/tigerstyle-audit.py crates`

- Status: pass with 5 known parse errors captured for follow-up
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.txt`

### `./scripts/tigerstyle-audit.py --pretty --json crates`

- Status: pass with 5 known parse errors captured for follow-up
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.json`

### `cargo test -p aspen-core-essentials-handler coordination`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/core-essentials-coordination-tests.txt`

### `cargo test -p aspen-forge-handler executor`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/forge-handler-executor-tests.txt`

### `./scripts/tigerstyle-audit.py crates`

- Status: pass with 5 known parse errors captured for follow-up
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/post-phase4-scan.txt`

### `./scripts/tigerstyle-audit.py --pretty --json crates`

- Status: pass with 5 known parse errors captured for follow-up
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/post-phase4-scan.json`

### `scripts/openspec-preflight.sh durable-tigerstyle-audit`

- Status: pass
- Artifact: `openspec/changes/durable-tigerstyle-audit/evidence/openspec-preflight.txt`

## Notes

- All tasks in this change now have repo-backed evidence.
- This turn completes the implementation and verification scope of the change. OpenSpec archive/commit workflow was not performed here because it is a separate step and was not part of the checked task list.
