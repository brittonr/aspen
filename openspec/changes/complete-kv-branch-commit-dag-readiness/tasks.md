## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `complete-kv-branch-commit-dag-readiness`.

## Phase 2: Implementation and evidence

- [ ] Inventory current `aspen-commit-dag` and `aspen-kv-branch` dependency graphs and hash/helper ownership.
- [ ] Add/update downstream and negative fixtures for default/no-default/feature-enabled graphs.
- [ ] Run fixture metadata, negative dependency checks, representative jobs/CI/deploy/FUSE/docs/CLI consumers, and update extraction docs.

## Phase 3: Closeout

- [ ] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [ ] Sync/archive only after every implementation/evidence task is complete.
