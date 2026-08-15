# Tasks: cluster-lifecycle-run-receipt

## Phase 1: Receipt core

- [x] [serial] r[molten.testing.cluster_lifecycle_receipt.run_receipt] Define a pure cluster lifecycle summary and `cluster-lifecycle-run-v1` Preserves receipt builder.
- [x] [serial] r[molten.testing.cluster_lifecycle_receipt.fail_closed_validation] Add validation diagnostics for missing receipts, duplicate nodes, node-order drift, stale manifest refs, and stdout-only evidence.

## Phase 2: CLI/test shell

- [x] [serial] r[molten.testing.cluster_lifecycle_receipt.run_receipt] Wire cluster lifecycle CLI tests to emit and inspect the canonical run receipt.
- [x] [parallel] r[molten.testing.cluster_lifecycle_receipt.fail_closed_validation] Add negative fixtures for missing phase receipts, stale lifecycle state, duplicate summaries, and rendered-output-only success.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.cluster_lifecycle_receipt.run_receipt] Document the receipt fields and evidence-only boundary in distributed testing docs.
- [x] [serial] r[molten.testing.cluster_lifecycle_receipt.run_receipt] Run focused cluster lifecycle tests and update traceability coverage with positive and negative evidence refs.
