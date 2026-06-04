# Node Runtime Delta: Live Iroh Transport

### Requirement: Live transport receipts are canonical
r[molten.node_control_live_iroh.spec.transport_receipts] Live node-control ingress MUST emit canonical `node-control-live-transport-receipt-v1` receipts that bind operation, decision, transport profile, topic, node, delivered-from endpoint, envelope ref, ingress receipt ref, diagnostics, and checks.

#### Scenario: Live receive writes a transport receipt
- GIVEN canonical live node-control ingress bytes
- WHEN the receiver accepts the bytes from Iroh gossip
- THEN it emits a parseable live transport receipt
- AND the receipt references the canonical envelope ref.

### Requirement: Live ingress uses Iroh gossip bytes
r[molten.node_control_live_iroh.spec.gossip_bytes] Live publish MUST broadcast canonical envelope bytes through `iroh-gossip`, and live receive MUST reject non-canonical or mismatched envelope bytes before trust-sensitive side effects.

#### Scenario: Canonical bytes are required
- GIVEN an ingress envelope
- WHEN it is published over live Iroh gossip
- THEN the broadcast payload is canonical Preserves bytes
- AND receive recomputes the envelope ref before storing it.

### Requirement: Live receive feeds the durable ingress path
r[molten.node_control_live_iroh.spec.durable_ingress] Live receive MUST store admitted envelopes in the existing ingress area and call the same ingress delivery function used by local-Iroh delivery.

#### Scenario: Live receive enqueues through ingress gates
- GIVEN a live ingress envelope with peer bootstrap, authority, policy, and resource refs
- WHEN the receiver processes it
- THEN any queued request is produced by normal ingress pre-enqueue gates
- AND dispatch remains the responsibility of `serve` or `run-loop`.

### Requirement: Transport is not authority
r[molten.node_control_live_iroh.spec.transport_not_authority] Live Iroh delivery MUST NOT by itself satisfy authority, policy, resource, provenance, or source-gate requirements.

#### Scenario: Missing evidence still denies
- GIVEN a live-delivered install or run request without admitted provenance
- WHEN the existing control loop dispatches the request
- THEN dispatch denies before operation side effects.

### Requirement: Live loopback coverage exists
r[molten.node_control_live_iroh.spec.loopback_tests] The implementation MUST include a local two-endpoint Iroh gossip loopback that publishes and receives a node-control ingress envelope.

#### Scenario: Loopback exercises real Iroh gossip
- GIVEN two local Iroh endpoints joined to the node-control topic
- WHEN one endpoint broadcasts a live ingress envelope
- THEN the other receives it through `iroh-gossip`
- AND the request is enqueued through the durable ingress path.
