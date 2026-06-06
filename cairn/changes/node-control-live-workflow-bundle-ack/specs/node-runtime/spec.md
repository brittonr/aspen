# Node Runtime Delta: Live Workflow Bundle Ack UX

### Requirement: Ack artifacts carry receiver evidence
r[molten.node_control_live_workflow_bundle_ack.spec.ack_artifact] Node-control live workflow bundle acknowledgement MUST emit canonical `node-control-live-workflow-bundle-ack-v1` artifacts and `node-control-live-workflow-bundle-ack-export-receipt-v1` / `node-control-live-workflow-bundle-ack-import-receipt-v1` receipts that bind apply, optional send, ingress, queue, optional control, reconcile, bundle, envelope, operation, request, receiver decision, diagnostics, and checks.

#### Scenario: Ack artifact records receiver outcome
- GIVEN an apply receipt and matching receiver ingress, queue, control, and reconcile receipts
- WHEN ack export runs
- THEN the ack artifact binds the same bundle, envelope, operation, and request refs
- AND the export receipt names the ack artifact ref.

### Requirement: Ack export validates reconcile and receiver evidence
r[molten.node_control_live_workflow_bundle_ack.spec.ack_export_binding] Ack export MUST recompute reconciliation from the supplied member receipts, MUST deny stale or mismatched reconcile receipts, MUST require receiver ingress evidence, and MUST require durable queue evidence when receiver ingress passed or named a queue receipt.

#### Scenario: Missing receiver ingress is denied
- GIVEN an apply receipt and reconcile receipt without receiver ingress evidence
- WHEN ack export runs
- THEN the export receipt decision is deny
- AND diagnostics identify the missing receiver ingress receipt.

### Requirement: Ack import validates expected bindings
r[molten.node_control_live_workflow_bundle_ack.spec.ack_import_binding] Ack import MUST parse embedded member receipts, recompute reconciliation, deny stale or mismatched reconcile evidence, enforce supplied expected bundle/envelope/operation/request refs, and only materialize ack/member receipts into the sender ledger after binding checks pass.

#### Scenario: Wrong expected envelope is denied
- GIVEN an ack bundle for one live envelope
- WHEN ack import is supplied another expected envelope ref
- THEN the import receipt decision is deny
- AND the sender ledger is not populated with ack member evidence.

### Requirement: Receiver denials remain portable evidence
r[molten.node_control_live_workflow_bundle_ack.spec.receiver_denial] Ack export/import MUST preserve receiver denial diagnostics and MUST NOT treat a receiver control denial as an invalid ack package when all member refs bind correctly.

#### Scenario: Denied receiver control imports
- GIVEN receiver ingress and queue receipts plus a denying control receipt for the same request
- WHEN ack export and ack import run
- THEN the ack package and import receipt can pass
- AND the receiver decision and denial diagnostics remain recorded.

### Requirement: Ack evidence is not authority
r[molten.node_control_live_workflow_bundle_ack.spec.non_authority] Ack artifacts and ack import/export receipts MUST NOT satisfy authority, provenance, policy/resource, delivery-idempotency, sender-import, receiver-ingress, or control-dispatch gates by themselves.

#### Scenario: Ack receipt cannot replace grant
- GIVEN an ack import/export receipt and no original authority grant ref in the live envelope
- WHEN live ingress evaluates authority
- THEN authority admission denies before enqueue
- AND the ack receipt is not treated as a grant.

### Requirement: CLI tests cover ack UX
r[molten.node_control_live_workflow_bundle_ack.spec.cli_tests] Automated tests MUST cover live workflow bundle ack export/import through the CLI and canonical artifact-kind recognition.

#### Scenario: CLI writes ack artifacts and receipts
- GIVEN CLI-produced apply and reconcile receipts
- WHEN ack export/import write artifacts and receipts
- THEN the ack artifact and ack receipts are recognized by ledger kind
- AND the ack artifact declares it is not authority.

### Requirement: Ack prints next steps
r[molten.node_control_live_workflow_bundle_ack.spec.next_steps] Ack CLI output MUST include deterministic next-step guidance for incomplete receiver evidence, importing exported ack bundles, and imported receiver denial outcomes.

#### Scenario: Incomplete ack guides evidence collection
- GIVEN an ack export missing receiver ingress evidence
- WHEN the CLI runs
- THEN output includes a next step to collect receiver ingress and queue evidence.
