# Node Runtime Delta: Live Workflow Bundle Gate UX

### Requirement: Bundle gate emits review receipts
r[molten.node_control_live_workflow_bundle_gate.spec.gate_receipt] Node-control live workflow bundle gating MUST emit a canonical `node-control-live-workflow-bundle-gate-receipt-v1` that binds the bundle ref, recomputed verify receipt ref, optional supplied verify receipt ref, expected bindings, diagnostics, and checks without importing bundle members.

#### Scenario: Valid bundle gate passes
- GIVEN a valid live workflow bundle and a current matching verify receipt
- WHEN the operator gates the bundle with that receipt required
- THEN the gate receipt decision is pass
- AND no sender state root mutation is required.

### Requirement: Gate detects stale verify receipts
r[molten.node_control_live_workflow_bundle_gate.spec.stale_verify_deny] Bundle gating MUST deny when a supplied verify receipt is malformed, stale, or bound to different expected arguments than the current gate invocation.

#### Scenario: Verify receipt was for a different topic
- GIVEN a valid bundle and a verify receipt produced with a wrong expected topic
- WHEN the operator gates the same bundle with the correct expected topic and requires the verify receipt
- THEN the gate receipt decision is deny
- AND diagnostics identify the recomputed verify mismatch.

### Requirement: Gate may require verify receipts
r[molten.node_control_live_workflow_bundle_gate.spec.required_verify_receipt] Bundle gating MUST fail closed when `--require-verify-receipt` is used without a supplied verify receipt.

#### Scenario: Required verify receipt is missing
- GIVEN a valid bundle
- WHEN the operator gates it with verify receipts required but does not supply one
- THEN the gate receipt decision is deny
- AND diagnostics explain that a current verify receipt is required.

### Requirement: Gate receipts are not authority
r[molten.node_control_live_workflow_bundle_gate.spec.non_authority] Bundle gate receipts MUST NOT satisfy authority, provenance, policy/resource, delivery-idempotency, sender-import, or receiver-ingress gates by themselves.

#### Scenario: Gate receipt cannot replace grant
- GIVEN a bundle gate receipt and no original authority grant ref in the live envelope
- WHEN live ingress evaluates authority
- THEN authority admission denies before enqueue
- AND the gate receipt is not treated as a grant.

### Requirement: CLI tests cover gate UX
r[molten.node_control_live_workflow_bundle_gate.spec.cli_tests] Automated tests MUST cover live workflow bundle gating through the CLI and canonical receipt kind recognition.

#### Scenario: CLI writes gate receipt
- GIVEN a CLI-exported live workflow bundle and matching verify receipt
- WHEN `live-workflow-bundle-gate` writes a receipt
- THEN the receipt kind is recognized
- AND the receipt declares that gating is not authority.

### Requirement: Gate prints next steps
r[molten.node_control_live_workflow_bundle_gate.spec.next_steps] Bundle gate CLI output MUST include deterministic next-step guidance for passing imports, stale or missing verify receipts, malformed bundles, and missing ticket/grant evidence.

#### Scenario: Passing gate guides import
- GIVEN a valid bundle gate decision
- WHEN the CLI prints the gate result
- THEN it includes a next step to run `live-workflow-bundle-import`.
