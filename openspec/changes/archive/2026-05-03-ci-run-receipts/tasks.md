## Phase 1: Spec

- [x] Define native CI run receipt behavior.

## Phase 2: Implementation

- [x] Add client API request/response models for CI run receipts.
- [x] Add CI handler projection from `PipelineRun` to receipt response.
- [x] Add `aspen-cli ci receipt <run-id> [--json]` output.
- [x] Update operator docs.

## Phase 3: Verification

- [x] Run targeted cargo checks/tests and CLI smoke.
- [x] Archive OpenSpec change, commit, push, and leave repo clean/even.
