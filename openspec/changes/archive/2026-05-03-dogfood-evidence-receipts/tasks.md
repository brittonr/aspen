## Phase 1: Receipt model foundation

- [x] Create OpenSpec proposal, design, tasks, and dogfood evidence delta spec.
- [x] Add `aspen-dogfood` receipt model with schema constant, bounded validation, stage receipts, artifact receipts, and canonical JSON helpers.
- [x] Add serialization, validation, and failure/status consistency tests for the receipt model.

## Phase 2: Live dogfood receipt writing

- [x] Add receipt path/run id configuration derived from the cluster/run directory.
- [x] Record start, push, build, deploy, verify, and stop stage receipts during `full` runs.
- [x] Save success and failure receipts before exit and print the receipt path in the final summary.

## Phase 3: Verification and evidence

- [x] Run `cargo nextest run -p aspen-dogfood` and save transcript under this change's `evidence/` directory.
- [x] Run `openspec validate dogfood-evidence-receipts --json` and helper verification, then save transcripts under `evidence/`.
- [x] Run `git diff --check` and save transcript under `evidence/`.
