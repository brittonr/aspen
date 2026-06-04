# Node Runtime Delta: Supervised Runner

### Requirement: Service runner receipts are canonical
r[molten.node_control_service.spec.canonical_receipts] The node control supervised runner MUST emit canonical Preserves service lock, service heartbeat, and service run receipt artifacts that bind startup, topic, bounds, ingress receipts, loop receipts, processed requests, diagnostics, and checks.

#### Scenario: Serve emits receipts
- GIVEN a running node with an active startup lock
- WHEN `molten node serve` runs for a bounded tick
- THEN it writes a parseable service run receipt
- AND the receipt references at least one service heartbeat receipt.

### Requirement: Service runner refuses duplicate active instances
r[molten.node_control_service.spec.duplicate_lock] The supervised runner MUST fail closed before ingress delivery or inbox drain if a service lock already exists for the state root.

#### Scenario: Duplicate service lock denies
- GIVEN a state root already has a service lock
- WHEN a second serve attempt starts
- THEN it emits or returns a denial before side effects
- AND no ingress envelope or inbox request is processed by that attempt.

### Requirement: Service runner drives ingress through the durable inbox
r[molten.node_control_service.spec.ingress_to_inbox] Each serve tick MUST scan published local-Iroh ingress envelopes in deterministic order and deliver them through the existing ingress delivery function before inbox drain.

#### Scenario: Published ingress becomes a queued request
- GIVEN a published ingress envelope for the served topic
- WHEN a serve tick runs
- THEN the envelope is delivered through ingress pre-enqueue gates
- AND any admitted request is queued in the durable control inbox before dispatch.

### Requirement: Service runner drains with existing control loop
r[molten.node_control_service.spec.loop_reuse] The supervised runner MUST drain requests only through the existing bounded control loop and MUST NOT bypass operation dispatch, provenance gates, source gates, or shutdown semantics.

#### Scenario: Serve dispatches through loop receipts
- GIVEN an admitted request in the durable inbox
- WHEN serve drains a tick
- THEN the resulting dispatch is represented by normal control loop and control receipts
- AND install/run provenance gates remain authoritative.

### Requirement: Service runner stops on shutdown
r[molten.node_control_service.spec.shutdown_stop] The supervised runner MUST stop after a passing shutdown dispatch removes the active control lock and MUST record the stop in the final service run receipt.

#### Scenario: Shutdown stops serve
- GIVEN a queued shutdown control request
- WHEN serve processes the request
- THEN the active node lock is removed
- AND the service run receipt records `stopped=true`.

### Requirement: Service runner coverage exists
r[molten.node_control_service.spec.tests] The implementation MUST cover duplicate runner denial, ingress-to-dispatch, shutdown stop, and heartbeat continuity.

#### Scenario: Test suite covers service safety
- GIVEN the Molten test suite
- WHEN node-control service tests run
- THEN duplicate lock, ingress delivery, dispatch, shutdown, and heartbeat receipt paths are exercised.
