# Tasks: executor-resource-gate-receipts

- [x] [serial] r[molten.evidence.executor_resource_gate_receipts.aggregate_ref] Compute a canonical aggregate ref for Steel/Wasm execution receipt events during gate checks.
- [x] [serial] r[molten.evidence.executor_resource_gate_receipts.artifact_ref] Add the aggregate execution receipt ref to gate receipt artifact refs.
- [x] [serial] r[molten.evidence.executor_resource_gate_receipts.checks] Add explicit pass checks for Steel resources, Wasm ABI byte bounds, guest memory bounds, and output-ref binding.
- [x] [parallel] r[molten.evidence.executor_resource_gate_receipts.validation] Require the new artifact refs and checks when parsing gate receipts.
- [x] [parallel] r[molten.evidence.executor_resource_gate_receipts.tests] Add tests proving gate receipts expose the new executor resource checks and reject missing checks.
