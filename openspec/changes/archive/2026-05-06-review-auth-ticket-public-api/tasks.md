## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `review-auth-ticket-public-api`.

## Phase 2: Implementation and evidence

- [x] Inventory portable auth/ticket public types and canonical imports.
- [x] Add/update portable downstream fixture plus negative runtime-verifier boundary fixture.
- [x] Run serialization goldens, malformed rejection tests, fixture metadata, and representative consumers; update extraction manifests.

## Phase 3: Closeout

- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [x] Sync/archive only after every implementation/evidence task is complete.
