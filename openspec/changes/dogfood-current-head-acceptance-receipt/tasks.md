## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `dogfood-current-head-acceptance-receipt`.

## Phase 2: Implementation and evidence

- [x] Confirm clean current head and identify the exact dogfood command and receipt paths.
- [x] Run or deliberately gate the full dogfood loop; capture success/failure receipt, local readback, cluster readback if applicable, and diagnostics.
- [x] Update operator docs/evidence with commit-bound receipt, redaction notes, and acceptance/failure triage.

## Phase 3: Closeout

- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
- [x] Sync/archive only after every implementation/evidence task is complete.
