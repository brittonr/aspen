## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `complete-kv-branch-commit-dag-readiness`.

## Phase 2: Implementation and evidence

- [x] Inventory current `aspen-commit-dag` and `aspen-kv-branch` dependency graphs and hash/helper ownership. Evidence: `evidence/i5-inventory-source-ownership.txt`, `evidence/i5-package-checks.txt`.
- [x] Add/update downstream and negative fixtures for default/no-default/feature-enabled graphs. Evidence: `fixtures/downstream-branch-dag/`, `evidence/i5-downstream-branch-dag-metadata.json`, `evidence/i5-downstream-branch-dag-check.txt`, `evidence/i5-downstream-branch-dag-forbidden-grep.txt`.
- [x] Run fixture metadata, negative dependency checks, representative jobs/CI/deploy/FUSE/docs/CLI consumers, and update extraction docs. Evidence: `evidence/i5-representative-consumers-check.txt`, `docs/crate-extraction/kv-branch-commit-dag.md`. The docs feature path is recorded as a pre-existing compatibility blocker, so the family remains `workspace-internal`.

## Phase 3: Closeout

- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [x] Sync/archive only after every implementation/evidence task is complete. Evidence: `evidence/i5-final-validation.txt`, `evidence/i5-preflight.txt`.
