# Node Runtime Delta: Live Workflow Bundle Apply UX

### Requirement: Bundle apply emits orchestration receipts
r[molten.node_control_live_workflow_bundle_apply.spec.apply_receipt] Node-control live workflow bundle apply MUST emit a canonical `node-control-live-workflow-bundle-apply-receipt-v1` that binds the state root, bundle ref, recomputed verify receipt ref, optional gate receipt ref, import receipt ref, imported refs, mode, optional live-send refs, expected bindings, diagnostics, and checks.

#### Scenario: Valid bundle apply imports members
- GIVEN a valid live workflow bundle and a current matching gate receipt
- WHEN the operator applies the bundle with the gate receipt required
- THEN the apply receipt decision is pass
- AND the underlying ticket, peer admission, authority grant, bundle, and supporting receipts are imported into the sender state root.

### Requirement: Apply may require current gate receipts
r[molten.node_control_live_workflow_bundle_apply.spec.gate_required] Bundle apply MUST deny when `--require-gate-receipt` is used without a supplied, parseable, passing, current gate receipt for the same bundle and expected bindings.

#### Scenario: Required gate receipt is missing or stale
- GIVEN a valid bundle
- WHEN apply requires a gate receipt but the receipt is absent or was produced for different expected arguments
- THEN the apply receipt decision is deny
- AND no bundle members are imported.

### Requirement: Apply imports only after validation
r[molten.node_control_live_workflow_bundle_apply.spec.import_after_validation] Bundle apply MUST validate the bundle and gate receipt before invoking bundle import, and MUST preserve the standalone import path's fail-closed checks.

#### Scenario: Invalid gate prevents import
- GIVEN a bundle and a non-passing gate receipt
- WHEN apply runs with the gate required
- THEN diagnostics identify the gate failure
- AND the sender ledger does not contain the bundle member refs.

### Requirement: Apply is dry-run by default
r[molten.node_control_live_workflow_bundle_apply.spec.dry_run_default] Bundle apply MUST NOT publish over live Iroh unless `--send` is explicitly supplied.

#### Scenario: Request preflight runs without send
- GIVEN a request and a valid applied bundle
- WHEN apply runs without `--send`
- THEN it records envelope and operation refs for preflight diagnostics
- AND no live-send receipt is emitted.

### Requirement: Explicit send uses live-send path
r[molten.node_control_live_workflow_bundle_apply.spec.send_explicit] Bundle apply with `--send` MUST delegate to the existing bounded live-send implementation and record the nested send receipt ref and value when one is produced.

#### Scenario: Explicit send records nested receipt
- GIVEN a request and a valid applied bundle
- WHEN apply runs with `--send`
- THEN the receipt mode is `send`
- AND the apply receipt includes the live-send receipt ref.

### Requirement: Apply receipts are not authority
r[molten.node_control_live_workflow_bundle_apply.spec.non_authority] Bundle apply receipts MUST NOT satisfy authority, provenance, policy/resource, delivery-idempotency, sender-import, or receiver-ingress gates by themselves.

#### Scenario: Apply receipt cannot replace grant
- GIVEN a bundle apply receipt and no original authority grant ref in the live envelope
- WHEN live ingress evaluates authority
- THEN authority admission denies before enqueue
- AND the apply receipt is not treated as a grant.

### Requirement: CLI tests cover apply UX
r[molten.node_control_live_workflow_bundle_apply.spec.cli_tests] Automated tests MUST cover live workflow bundle apply through the CLI and canonical receipt kind recognition.

#### Scenario: CLI writes apply receipt
- GIVEN a CLI-exported live workflow bundle and matching gate receipt
- WHEN `live-workflow-bundle-apply` writes a receipt
- THEN the receipt kind is recognized
- AND the receipt declares that apply is not authority.
