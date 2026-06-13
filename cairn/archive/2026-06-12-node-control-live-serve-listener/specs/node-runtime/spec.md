# Node Runtime Delta: Live Serve Listener

### Requirement: Live listener receipts are canonical
r[molten.node_control_live_listener.spec.listener_receipts] Live serve listener mode MUST emit canonical `node-control-live-listener-receipt-v1` receipts that bind startup, node, logical endpoint, bound Iroh endpoint, topic, event bound, observed events, transport receipts, neighbor events, service run receipt, diagnostics, and checks.

#### Scenario: Listener emits receipt
- GIVEN a running node
- WHEN `molten node serve --live-iroh` runs with a bounded event limit
- THEN it emits a parseable live listener receipt
- AND the receipt binds the service run receipt used for dispatch drain.

### Requirement: Live listener feeds live receive before drain
r[molten.node_control_live_listener.spec.receive_before_drain] Live serve listener mode MUST process live Iroh gossip events through the existing live receive function before invoking the supervised control drain.

#### Scenario: Live event is received before dispatch
- GIVEN a live Iroh node-control envelope published to the subscribed topic
- WHEN the listener processes the event
- THEN the envelope is stored through live receive
- AND the queued request is drained only by the supervised control loop.

### Requirement: Listener records neighbor/session evidence
r[molten.node_control_live_listener.spec.session_evidence] Live serve listener receipts MUST record bounded neighbor/session observations without treating them as authority.

#### Scenario: Neighbor evidence is non-authority
- GIVEN a listener observes an Iroh neighbor event
- WHEN it writes the listener receipt
- THEN the neighbor observation is included as diagnostics/evidence
- AND authority, policy, resource, and provenance gates are still required separately.

### Requirement: Listener remains bounded
r[molten.node_control_live_listener.spec.bounded_listener] Live serve listener mode MUST be bounded by explicit event and timeout limits for deterministic tests and replayable operation.

#### Scenario: Empty listener exits on bound
- GIVEN no live Iroh event arrives
- WHEN the listener reaches its event or timeout bound
- THEN it emits a listener receipt
- AND drains the existing control inbox according to the configured request bound.

### Requirement: Live listener loopback coverage exists
r[molten.node_control_live_listener.spec.loopback_tests] The implementation MUST include local two-endpoint Iroh loopback coverage that publishes to a listener and dispatches through serve.

#### Scenario: Loopback listener dispatches request
- GIVEN two local Iroh endpoints joined to the node-control topic
- WHEN one endpoint publishes a live node-control envelope
- THEN the listener endpoint receives it
- AND the queued request is dispatched through the supervised control loop.
