## Phase 1: Durable audit artifacts

- [x] Write `audit-report.md` in this change with the current scan scope, methodology, known scanner limitations, ranked hotspot table, and an explicit status line (`audit-only`, `remediation in progress`, or `remediation complete`).
- [x] Add a repo-local Tiger Style scanner command that can be rerun by reviewers and emits a machine-readable hotspot inventory.
- [x] Save the initial scanner transcript under `evidence/` and reference it from `audit-report.md`.

## Phase 2: Scanner regression coverage

- [x] Add a committed regression fixture that reproduces the trait-declaration false positive described in the audit notes.
- [x] Verify the scanner ignores declaration-only signatures ending in `;` and only counts functions whose signatures reach a body `{` before a top-level `;`.
- [x] Save the regression run output under `evidence/` so the napkin rule has in-repo support.

## Phase 3: First remediation slice

- [x] Add characterization coverage for `crates/aspen-ci-handler/src/handler/pipeline.rs:handle_trigger_pipeline` before refactoring.
- [x] Refactor `handle_trigger_pipeline` into smaller helpers, moving deterministic planning logic out of the main async shell and adding assertions around parsed IDs, ref resolution, and checkout/cleanup invariants.
- [x] Add characterization coverage for `crates/aspen-auth/src/capability.rs:Capability::authorizes` and `Capability::contains` before refactoring.
- [x] Split `Capability::authorizes` and `Capability::contains` into smaller pure helpers grouped by capability family, preserving behavior and making the decision table easier to test.
- [x] Reduce `usize` leakage in the first verified/public hotspot set:
  - `crates/aspen-ci-core/src/verified/pipeline.rs`
  - `crates/aspen-forge/src/verified/ref_validation.rs`
  - `crates/aspen-cluster/src/gossip/discovery/helpers.rs`
- [x] Re-run the Tiger Style audit and update `audit-report.md` with before/after counts for the first remediation slice.

## Phase 4: Second remediation slice

- [x] Add characterization coverage for `crates/aspen-core-essentials-handler/src/coordination.rs:handle` before extraction.
- [x] Break `crates/aspen-core-essentials-handler/src/coordination.rs:handle` into bounded request-family helpers with shared response builders to reduce repetition and centralize control flow.
- [x] Add characterization coverage for `crates/aspen-forge-handler/src/executor.rs:execute` before extraction.
- [x] Break `crates/aspen-forge-handler/src/executor.rs:execute` into per-request helpers or executor submodules so the top-level dispatcher stays under the Tiger Style function-length target.

## Phase 5: Verification and completion discipline

- [x] Save all command transcripts, diffs, and targeted test output under `evidence/`.
- [x] Add `verification.md` once any task is checked, using verbatim task text and repo-relative evidence paths.
- [x] Run `scripts/openspec-preflight.sh durable-tigerstyle-audit` before requesting done review or checking the final task.
