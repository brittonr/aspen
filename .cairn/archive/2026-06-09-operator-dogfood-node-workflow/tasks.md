## Phase 1: Workflow records

- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Define operator workflow, step, checkpoint, dogfood report, and release gate receipt DTOs.
- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Classify operator workflow/report/gate artifacts in ledger/catalog/MCP views.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.report] Render concise operator summaries from canonical reports.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.report] Document that text logs are non-normative views over receipts.

## Phase 2: Local-node dogfood workflow

- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Create a clean explicit state root and initialize node config/identity.
- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Start node, collect startup/health receipts, and shut down with shutdown receipts.
- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Install artifact, start service, publish remote dataspace assertion, run job DAG sync/admit/execute, query catalog, export repro, and gate evidence.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.report] Emit checkpoints after each step with request/ref/result bindings.

## Phase 3: Gating and release evidence

- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Emit `dogfood-report-v1` with pass/deny/diagnostic decision and all step receipts.
- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.release_gate] Emit `release-gate-receipt-v1` only if mandatory deterministic/recorded steps pass.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.release_gate] Require redaction/encrypted-ref checks before exporting dogfood repro bundles.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.report] Expose dogfood status through read-only catalog/MCP calls.

## Phase 4: Tests and CI hook

- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.no_hidden_bypass] Add `molten dogfood local-node` or equivalent test command.
- [x] [serial] r[molten.operator_dogfood_node_workflow.spec.report] Add integration test covering the local dogfood workflow with fixture artifacts.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.release_gate] Test missing receipt, non-replayable production evidence, redaction leak, stale policy, and dirty state denial.
- [x] [parallel] r[molten.operator_dogfood_node_workflow.spec.release_gate] Add an optional CI profile/app that runs the dogfood workflow after unit/nextest gates.
