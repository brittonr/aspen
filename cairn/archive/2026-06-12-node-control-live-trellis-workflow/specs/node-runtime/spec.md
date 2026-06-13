# Node Runtime Delta: Live Trellis Workflow Gate

### Requirement: Live workflow evidence uses a finite Trellis protocol
r[molten.node_control_live_trellis_workflow.spec.protocol_shape] Node-control live workflow protocol gating MUST model bundle handoff, apply evidence, and ack evidence as a finite sender/receiver Trellis protocol before emitting completion evidence.

#### Scenario: Completed workflow reaches terminal protocol states
- GIVEN a live workflow bundle, matching gate receipt, apply receipt, reconcile receipt, and ack artifact
- WHEN protocol gating runs
- THEN the emitted protocol session gate receipt records a passing lifecycle
- AND the lifecycle contains sender and receiver terminal states.

### Requirement: Protocol gate binds workflow receipt refs
r[molten.node_control_live_trellis_workflow.spec.workflow_bindings] Protocol gating MUST verify that the gate receipt binds the bundle, the apply receipt binds the bundle and gate receipt, the reconcile receipt binds the apply receipt and bundle, and the ack binds the apply, reconcile, and bundle refs.

#### Scenario: Mismatched gate evidence is denied
- GIVEN an apply receipt whose gate ref does not match the supplied bundle gate receipt
- WHEN protocol gating runs
- THEN the protocol session gate receipt decision is deny
- AND diagnostics identify the mismatched apply gate binding.

### Requirement: Receiver outcome remains part of the gate
r[molten.node_control_live_trellis_workflow.spec.receiver_outcome] Protocol gating MUST require the ack to preserve a passing receiver decision and MUST surface receiver denial diagnostics when the ack records a denied receiver outcome.

#### Scenario: Receiver denial denies workflow protocol gate
- GIVEN a valid ack package that records a receiver control denial
- WHEN protocol gating runs
- THEN the ack remains parseable evidence
- AND the protocol session gate receipt decision is deny with receiver-denial diagnostics.

### Requirement: Expected operation guards are enforced
r[molten.node_control_live_trellis_workflow.spec.expected_guards] Protocol gating MUST enforce supplied expected envelope, operation, and request refs against the ack artifact.

#### Scenario: Wrong expected request is denied
- GIVEN an ack artifact for one receiver request
- WHEN protocol gating is supplied another expected request ref
- THEN the protocol session gate receipt decision is deny
- AND diagnostics identify the expected request mismatch.

### Requirement: Protocol gate receipts are not authority
r[molten.node_control_live_trellis_workflow.spec.non_authority] Protocol session gate receipts for live workflows MUST NOT satisfy authority, peer bootstrap, policy/resource, provenance, sender-import, receiver-ingress, delivery-idempotency, or transport gates by themselves.

#### Scenario: Protocol gate cannot replace grant
- GIVEN a protocol session gate receipt and no admitted authority grant in a live ingress envelope
- WHEN live ingress evaluates authority
- THEN authority admission denies before enqueue
- AND the protocol gate receipt is not treated as a grant.

### Requirement: CLI covers workflow protocol gating
r[molten.node_control_live_trellis_workflow.spec.cli_tests] Automated tests MUST cover the CLI protocol gate path and canonical artifact-kind recognition for the emitted protocol session gate receipt.

#### Scenario: CLI writes protocol gate receipt
- GIVEN CLI-produced bundle, gate, apply, reconcile, and ack artifacts
- WHEN `live-workflow-bundle-protocol-gate` writes a receipt
- THEN the receipt is recognized as `protocol-session-gate-receipt`
- AND its rendered checks declare it is not authority.
