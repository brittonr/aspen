# Node Runtime Delta: Live Workflow Bundle Verify UX

### Requirement: Bundle verification emits non-mutating receipts
r[molten.node_control_live_workflow_bundle_verify.spec.verify_receipt] Node-control live workflow bundle verification MUST emit a canonical `node-control-live-workflow-bundle-verify-receipt-v1` that binds the bundle ref, parsed member refs, expected bindings, diagnostics, and checks without importing bundle members.

#### Scenario: Valid bundle verifies offline
- GIVEN a valid live workflow bundle with matching ticket, peer admission, authority grant, and supporting receipts
- WHEN the operator verifies the bundle
- THEN the verify receipt decision is pass
- AND no sender state root mutation is required.

### Requirement: Verification rejects malformed bundles
r[molten.node_control_live_workflow_bundle_verify.spec.malformed_deny] Bundle verification MUST fail closed on missing fields, malformed member records, member ref mismatches, or unsupported supporting receipt kinds.

#### Scenario: Missing member fields deny verification
- GIVEN a bundle-shaped value with missing member fields
- WHEN the operator verifies the bundle
- THEN the verify receipt decision is deny
- AND diagnostics explain the parse or member binding failure.

### Requirement: Verification checks expected bindings
r[molten.node_control_live_workflow_bundle_verify.spec.expected_bindings] Bundle verification MUST validate expected node, topic, endpoint, peer, operation, scope, freshness, and revocation bounds using the same binding rules as bundle import.

#### Scenario: Wrong expected topic denies verification
- GIVEN a valid bundle for topic `node-control`
- WHEN verification is run with a different expected topic
- THEN the verify receipt decision is deny
- AND diagnostics identify the topic mismatch.

### Requirement: Verify receipts are not authority
r[molten.node_control_live_workflow_bundle_verify.spec.non_authority] Bundle verify receipts MUST NOT satisfy authority, provenance, policy/resource, delivery-idempotency, sender-import, or receiver-ingress gates by themselves.

#### Scenario: Verify receipt cannot replace grant
- GIVEN a bundle verify receipt and no original authority grant ref in the live envelope
- WHEN live ingress evaluates authority
- THEN authority admission denies before enqueue
- AND the verify receipt is not treated as a grant.

### Requirement: CLI tests cover verify UX
r[molten.node_control_live_workflow_bundle_verify.spec.cli_tests] Automated tests MUST cover live workflow bundle verification through the CLI and canonical receipt kind recognition.

#### Scenario: CLI writes verify receipt
- GIVEN a CLI-exported live workflow bundle
- WHEN `live-workflow-bundle-verify` writes a receipt
- THEN the receipt kind is recognized
- AND the receipt declares that verification is not authority.
