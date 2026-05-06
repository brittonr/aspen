## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `complete-blob-castore-cache-readiness`.

## Phase 2: Implementation and evidence

- [x] Audit current blob/castore/cache manifests and identify remaining workspace-internal reasons.
- [x] Add/update downstream positive fixtures, negative policy fixtures, and readiness checker mapping for the family.
- [x] Run fixture builds, metadata capture, negative mutation checks, representative Aspen consumers, and update readiness docs.

## Phase 3: Closeout

- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [x] Sync/archive only after every implementation/evidence task is complete.
