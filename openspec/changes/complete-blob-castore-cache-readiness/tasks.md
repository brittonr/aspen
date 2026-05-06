## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `complete-blob-castore-cache-readiness`.

## Phase 2: Implementation and evidence

- [ ] Audit current blob/castore/cache manifests and identify remaining workspace-internal reasons.
- [ ] Add/update downstream positive fixtures, negative policy fixtures, and readiness checker mapping for the family.
- [ ] Run fixture builds, metadata capture, negative mutation checks, representative Aspen consumers, and update readiness docs.

## Phase 3: Closeout

- [ ] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [ ] Sync/archive only after every implementation/evidence task is complete.
