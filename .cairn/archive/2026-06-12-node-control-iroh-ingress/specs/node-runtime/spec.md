# Node Runtime Delta: Control Iroh Ingress

### Requirement: Ingress envelopes are canonical
r[molten.node_control_ingress.spec.canonical_envelope] Remote-facing node control ingress MUST use canonical Preserves `node-control-ingress-envelope-v1` and `node-control-ingress-receipt-v1` artifacts that bind transport profile, topic, source peer, target node, sequence, operation ref, request ref, embedded control request, peer bootstrap refs, authority refs, policy refs, resource refs, evidence refs, diagnostics, and checks.

#### Scenario: Envelope binds embedded request
- GIVEN a node control request
- WHEN an ingress envelope is built
- THEN the envelope request ref matches the embedded request hash
- AND the operation ref is derived from the target topic, source peer, target node, sequence, request ref, and policy refs.

### Requirement: Ingress gates run before enqueue
r[molten.node_control_ingress.spec.pre_enqueue_gates] Delivery of an ingress envelope MUST fail closed before writing to the durable control inbox unless peer bootstrap, authority, policy, resource, and delivery idempotency checks pass.

#### Scenario: Missing authority denies before enqueue
- GIVEN an ingress envelope with no authority refs
- WHEN it is delivered
- THEN delivery emits a denying ingress receipt
- AND no control inbox request is written.

### Requirement: Ingress uses the durable inbox
r[molten.node_control_ingress.spec.durable_inbox] Passing ingress delivery MUST enqueue the embedded request through the same durable file-backed inbox used by local control submit, and MUST NOT dispatch the request directly.

#### Scenario: Delivered request is later dispatched by the loop
- GIVEN a running node and a passing ingress delivery
- WHEN `molten node run-loop` processes the inbox
- THEN the resulting control receipt is produced by normal dispatch
- AND existing install/run/gate provenance and source gates remain authoritative.

### Requirement: Duplicate remote operations are replay-safe
r[molten.node_control_ingress.spec.duplicate_replay] Ingress delivery MUST use scoped delivery idempotency so duplicate remote operations suppress enqueue side effects and stale, sequence-gap, or conflicting operations deny before enqueue.

#### Scenario: Duplicate ingress suppresses enqueue
- GIVEN an ingress envelope has already been delivered
- WHEN the same envelope is delivered again
- THEN the ingress receipt records duplicate suppression
- AND no second inbox side effect is committed.

### Requirement: Ingress CLI is available
r[molten.node_control_ingress.spec.cli] The CLI MUST expose commands to build, publish, and deliver deterministic local-Iroh node-control ingress envelopes, with optional receipt output files.

#### Scenario: CLI delivers an ingress envelope
- GIVEN a running node and a canonical request file
- WHEN an operator builds, publishes, and delivers a local-Iroh ingress envelope
- THEN the CLI writes parseable ingress receipts
- AND the request becomes visible to the durable control loop.

### Requirement: Ingress tests cover safety paths
r[molten.node_control_ingress.spec.tests] The implementation MUST include coverage for passing ingress, duplicate suppression, missing authority denial, and provenance-gated dispatch after ingress.

#### Scenario: Test suite covers ingress safety
- GIVEN the Molten test suite
- WHEN node control ingress tests run
- THEN ingress pass, duplicate, authority denial, and provenance denial cases are covered with canonical receipts.
