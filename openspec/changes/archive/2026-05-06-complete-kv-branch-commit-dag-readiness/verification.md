# Verification

## Implementation Evidence

- Changed file: `docs/crate-extraction/kv-branch-commit-dag.md`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/design.md`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/specs/kv-branch-commit-dag-extraction/spec.md`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/verification.md`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/fixtures/downstream-branch-dag/Cargo.toml`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/fixtures/downstream-branch-dag/Cargo.lock`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/fixtures/downstream-branch-dag/src/lib.rs`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-inventory-source-ownership.txt`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-package-checks.txt`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-metadata.json`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-check.txt`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-forbidden-grep.txt`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-representative-consumers-check.txt`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-final-validation.txt`
- Changed file: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-preflight.txt`

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `complete-kv-branch-commit-dag-readiness`.
  - Evidence: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/proposal.md`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/design.md`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/specs/kv-branch-commit-dag-extraction/spec.md`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/tasks.md`
- [x] Inventory current `aspen-commit-dag` and `aspen-kv-branch` dependency graphs and hash/helper ownership. Evidence: `evidence/i5-inventory-source-ownership.txt`, `evidence/i5-package-checks.txt`.
  - Evidence: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-inventory-source-ownership.txt`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-package-checks.txt`
- [x] Add/update downstream and negative fixtures for default/no-default/feature-enabled graphs. Evidence: `fixtures/downstream-branch-dag/`, `evidence/i5-downstream-branch-dag-metadata.json`, `evidence/i5-downstream-branch-dag-check.txt`, `evidence/i5-downstream-branch-dag-forbidden-grep.txt`.
  - Evidence: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/fixtures/downstream-branch-dag/Cargo.toml`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/fixtures/downstream-branch-dag/src/lib.rs`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-metadata.json`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-check.txt`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-forbidden-grep.txt`
- [x] Run fixture metadata, negative dependency checks, representative jobs/CI/deploy/FUSE/docs/CLI consumers, and update extraction docs. Evidence: `evidence/i5-representative-consumers-check.txt`, `docs/crate-extraction/kv-branch-commit-dag.md`. The docs feature path is recorded as a pre-existing compatibility blocker, so the family remains `workspace-internal`.
  - Evidence: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-representative-consumers-check.txt`, `docs/crate-extraction/kv-branch-commit-dag.md`
- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
  - Evidence: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-final-validation.txt`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-preflight.txt`
- [x] Sync/archive only after every implementation/evidence task is complete. Evidence: `evidence/i5-final-validation.txt`, `evidence/i5-preflight.txt`.
  - Evidence: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-final-validation.txt`, `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-preflight.txt`

## Verification Commands

- Command: `cargo check -p aspen-commit-dag`; `cargo check -p aspen-kv-branch --no-default-features`; `cargo check -p aspen-kv-branch --features commit-dag`
- Artifact: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-package-checks.txt`
- Command: `cargo check` and `cargo test` for `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/fixtures/downstream-branch-dag`
- Artifact: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-check.txt`
- Command: `cargo tree` default/no-default/feature graph scans plus forbidden dependency grep
- Artifact: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-downstream-branch-dag-forbidden-grep.txt`
- Command: representative consumer `cargo check` rails and documented docs-feature blocker capture
- Artifact: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-representative-consumers-check.txt`
- Command: `openspec validate complete-kv-branch-commit-dag-readiness --strict`; helper verify; `git diff --check`; `scripts/openspec-preflight.sh complete-kv-branch-commit-dag-readiness`
- Artifact: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-final-validation.txt`
- Artifact: `openspec/changes/archive/2026-05-06-complete-kv-branch-commit-dag-readiness/evidence/i5-preflight.txt`

## Drain Verification Matrix

| Rail | Command / oracle | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| Source ownership | Scan `crates/aspen-commit-dag` and `crates/aspen-kv-branch` for live `aspen_raft::verified` imports and `aspen-raft` dependency declarations | PASS | `evidence/i5-inventory-source-ownership.txt` | Proves the reusable branch/DAG source seam does not import the Raft helper surface outside historical Verus comments. | Re-run after any branch/DAG hash-helper source move. |
| Leaf package checks | `cargo check -p aspen-commit-dag`; `cargo check -p aspen-kv-branch --no-default-features`; `cargo check -p aspen-kv-branch --features commit-dag` | PASS | `evidence/i5-package-checks.txt` | Proves the default/minimal/feature-enabled leaf graphs compile. | Promote to nextest after source changes. |
| Downstream fixture | `cargo check` and `cargo test` for `fixtures/downstream-branch-dag` | PASS | `evidence/i5-downstream-branch-dag-check.txt`; `fixtures/downstream-branch-dag/` | Proves an external consumer can use branch overlay plus direct commit hash/DAG APIs. | Keep fixture current with any public API change. |
| Negative graph checks | `cargo tree` for `aspen-commit-dag` and `aspen-kv-branch` default/no-default/commit-dag graphs, then forbidden dependency scan | PASS | `evidence/i5-downstream-branch-dag-forbidden-grep.txt` | Proves forbidden app/runtime shells (`aspen-raft`, root app, CI/forge/CLI/FUSE/docs shells) are absent from the reusable leaf graphs. | Add this family to any future centralized extraction-readiness checker before raising readiness. |
| Representative consumers | `cargo check` for jobs, CI shell executor, deploy, FUSE, CLI feature paths; attempted docs feature path | PASS with documented blocker | `evidence/i5-representative-consumers-check.txt`; `docs/crate-extraction/kv-branch-commit-dag.md` | Proves current feature consumers except docs compile; docs failure is pre-existing `iroh-blobs`/`iroh-docs` and RNG API skew outside branch/DAG leaf graph. | Fix `aspen-docs --features commit-dag-federation` before promoting beyond `workspace-internal`. |

## Notes

- The extraction doc intentionally keeps the family at `workspace-internal`; this slice captures current evidence and the concrete docs blocker rather than falsely promoting readiness.
- No secrets or operator credentials are present in the captured transcripts.
