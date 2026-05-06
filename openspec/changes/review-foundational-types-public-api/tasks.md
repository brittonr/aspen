## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `review-foundational-types-public-api`.

## Phase 2: Implementation and evidence

- [x] Inventory public APIs and compatibility shims for all foundational crates.
- [x] Add or update downstream fixture and negative boundary checks for the reviewed surface.
- [x] Run no-std, extraction-readiness, and representative consumer checks; update manifests/docs with raise/no-raise evidence.

## Phase 3: Closeout

- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [x] Sync/archive only after every implementation/evidence task is complete.
