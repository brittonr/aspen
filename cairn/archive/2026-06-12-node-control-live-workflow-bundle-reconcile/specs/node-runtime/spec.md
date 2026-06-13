# Node Runtime Delta: Live Workflow Bundle Reconcile UX

### Requirement: Reconcile emits receiver-evidence receipts
r[molten.node_control_live_workflow_bundle_reconcile.spec.reconcile_receipt] Node-control live workflow bundle reconciliation MUST emit a canonical `node-control-live-workflow-bundle-reconcile-receipt-v1` that binds the apply receipt ref, bundle ref, optional send/ingress/queue/control receipt refs, envelope/operation/request refs, diagnostics, and checks.

#### Scenario: Valid receiver evidence reconciles
- GIVEN a passing apply receipt for a live envelope and receiver ingress/queue/control receipts for that envelope
- WHEN the operator reconciles the workflow
- THEN the reconcile receipt decision is pass
- AND the receipt binds the same envelope, operation, and request refs.

### Requirement: Apply and send receipts are bound
r[molten.node_control_live_workflow_bundle_reconcile.spec.apply_send_binding] Reconciliation MUST deny when the apply receipt is non-passing, has no live envelope, references a send receipt that is absent, or is paired with a send receipt for a different envelope.

#### Scenario: Missing send receipt is denied
- GIVEN an apply receipt that references a send receipt
- WHEN reconcile runs without that send receipt
- THEN the reconcile receipt decision is deny
- AND diagnostics identify the missing send receipt.

### Requirement: Receiver ingress evidence is required
r[molten.node_control_live_workflow_bundle_reconcile.spec.receiver_ingress_binding] Reconciliation MUST deny missing receiver ingress receipts and MUST deny receiver ingress receipts whose envelope, operation, or expected refs differ from the apply/send workflow.

#### Scenario: Wrong receiver envelope is denied
- GIVEN a passing apply receipt for one envelope
- WHEN reconcile is supplied a receiver ingress receipt or expected envelope for another envelope
- THEN the reconcile receipt decision is deny
- AND diagnostics identify the envelope mismatch.

### Requirement: Queue and control receipts bind the same request
r[molten.node_control_live_workflow_bundle_reconcile.spec.queue_control_binding] Reconciliation MUST require passing receiver ingress receipts to include durable queue evidence, MUST check supplied queue/control receipts against the same receiver request ref, and MUST propagate receiver control denial diagnostics.

#### Scenario: Receiver control denial propagates
- GIVEN a receiver ingress receipt that enqueued a request and a control receipt denying that request
- WHEN reconcile runs with those receipts
- THEN the reconcile receipt decision is deny
- AND diagnostics include the receiver denial.

### Requirement: Reconcile receipts are not authority
r[molten.node_control_live_workflow_bundle_reconcile.spec.non_authority] Bundle reconcile receipts MUST NOT satisfy authority, provenance, policy/resource, delivery-idempotency, sender-import, or receiver-ingress gates by themselves.

#### Scenario: Reconcile receipt cannot replace grant
- GIVEN a bundle reconcile receipt and no original authority grant ref in the live envelope
- WHEN live ingress evaluates authority
- THEN authority admission denies before enqueue
- AND the reconcile receipt is not treated as a grant.

### Requirement: CLI tests cover reconcile UX
r[molten.node_control_live_workflow_bundle_reconcile.spec.cli_tests] Automated tests MUST cover live workflow bundle reconciliation through the CLI and canonical receipt kind recognition.

#### Scenario: CLI writes reconcile receipt
- GIVEN a CLI-produced apply receipt
- WHEN `live-workflow-bundle-reconcile` writes a receipt
- THEN the receipt kind is recognized
- AND the receipt declares that reconciliation is not authority.

### Requirement: Reconcile prints next steps
r[molten.node_control_live_workflow_bundle_reconcile.spec.next_steps] Reconcile CLI output MUST include deterministic next-step guidance for missing receiver ingress evidence, receiver denial diagnostics, and passing outcomes with or without control receipts.

#### Scenario: Missing ingress guides wait/import
- GIVEN an apply receipt and no receiver ingress receipt
- WHEN reconcile runs
- THEN the CLI output includes a next step to wait for or import receiver ingress evidence.
