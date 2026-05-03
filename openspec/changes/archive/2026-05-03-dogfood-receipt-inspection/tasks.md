## Phase 1: Specification

- [x] Define receipt inspection proposal, design, and dogfood-evidence delta spec.

## Phase 2: Implementation

- [x] Add receipt directory discovery, loading, sorting, and summary helpers.
- [x] Add `aspen-dogfood receipts list` with empty-directory behavior and invalid-file warnings.
- [x] Add `aspen-dogfood receipts show <run-id-or-path>` with text and canonical JSON modes.
- [x] Add focused unit tests for receipt listing/show helper behavior.

## Phase 3: Verification

- [x] Run `openspec validate dogfood-receipt-inspection --json`.
- [x] Run `cargo nextest run -p aspen-dogfood`.
- [x] Run `git diff --check`, archive completed OpenSpec change, commit, and push.
