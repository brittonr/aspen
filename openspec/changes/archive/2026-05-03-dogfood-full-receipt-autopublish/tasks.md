## Phase 1: Spec

- [x] Define automatic full-run in-cluster receipt publication behavior.
- [x] Define explicit leave-running operator inspection behavior.

## Phase 2: Implementation

- [x] Add `full --leave-running` CLI args and auto-publish final success receipts before stop.
- [x] Update operator docs for normal auto-publication and leave-running readback.

## Phase 3: Verification

- [x] Run focused `aspen-dogfood` checks and CLI smoke tests.
- [x] Archive the OpenSpec change, commit, push, and leave the repo clean/even.
