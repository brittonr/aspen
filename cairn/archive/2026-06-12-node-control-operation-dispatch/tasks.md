## Phase 1: Operation routing

- [x] [serial] r[molten.node_control_operation.spec.install_dispatch] Route `install` requests to the node artifact registry with ledger-resolved payloads.
- [x] [serial] r[molten.node_control_operation.spec.run_dispatch] Route `run` requests to node-local job execution with matching admission receipts.
- [x] [serial] r[molten.node_control_operation.spec.gate_dispatch] Route `gate` requests through strict Octet source-gate validation.

## Phase 2: Safety and evidence

- [x] [serial] r[molten.node_control_operation.spec.fail_closed_before_effects] Deny side-effecting operations before effects when authority, policy, resource, payload, target, or ledger evidence is missing.
- [x] [parallel] r[molten.node_control_operation.spec.ledger_imports] Import operation subreceipts and final control receipts into the node ledger.

## Phase 3: Coverage

- [x] [parallel] r[molten.node_control_operation.spec.tests] Add tests for passing install/run/gate dispatch and missing evidence denial.
