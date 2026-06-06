# Node Runtime Delta: Live Send UX

### Requirement: Live send receipts are canonical
r[molten.node_control_live_send_ux.spec.send_receipt] Node-control live send attempts MUST be represented by canonical `node-control-live-send-receipt-v1` receipts that bind decision, transport profile, topic, from peer, destination node, receiver ticket ref, envelope ref, optional transport receipt ref, diagnostics, and checks.

#### Scenario: Send emits receipt
- GIVEN a node-control request and receiver live ticket
- WHEN live send runs
- THEN a canonical send receipt is emitted
- AND the receipt has a stable artifact ref.

### Requirement: Live workflow receipts are canonical
r[molten.node_control_live_send_ux.spec.workflow_receipt] Node-control live operator workflows MUST be representable by canonical `node-control-live-workflow-receipt-v1` receipts that bind receiver ticket, peer admission, authority grant, send receipt, receive receipts, optional listener receipt, service-run receipt, diagnostics, and consistency checks.

#### Scenario: Workflow receipt binds runbook evidence
- GIVEN ticket, admission, authority grant, send, receive, listener, and service-run receipts from a live workflow
- WHEN a workflow bundle is created
- THEN a canonical workflow receipt is emitted
- AND it has a stable artifact ref.

### Requirement: Receiver endpoint evidence is bound
r[molten.node_control_live_send_ux.spec.ticket_endpoint_binding] Live send receipts MUST bind the receiver ticket endpoint id and endpoint address evidence used to join the live transport.

#### Scenario: Bound ticket addresses are recorded
- GIVEN a bound live ticket with endpoint addresses
- WHEN live send publishes an envelope
- THEN the send receipt records the receiver endpoint id
- AND it records the receiver address list.

### Requirement: Live send CLI is available
r[molten.node_control_live_send_ux.spec.live_send_cli] The CLI MUST expose `molten node control-ingress-live-send` with request, ticket, peer/evidence refs, optional state root import, transport receipt output, and send receipt output.

#### Scenario: CLI writes send receipt
- GIVEN a request and ticket file
- WHEN `control-ingress-live-send --receipt-out` runs
- THEN the receipt file contains a canonical live send receipt
- AND optional state-root import records the send evidence in the node ledger.

### Requirement: Live workflow bundle CLI is available
r[molten.node_control_live_send_ux.spec.workflow_cli] The CLI MUST expose `molten node live-workflow-bundle` to tie ticket, peer admission, authority grant, send, receive/listener, and service-run evidence into a workflow receipt.

#### Scenario: CLI writes workflow receipt
- GIVEN live workflow evidence files
- WHEN `live-workflow-bundle --receipt-out` runs
- THEN the receipt file contains a canonical workflow receipt
- AND diagnostics explain any missing receive/listener evidence.

### Requirement: Live send uses real Iroh gossip
r[molten.node_control_live_send_ux.spec.real_gossip_send] Live send MUST join the receiver's real Iroh gossip topic from ticket endpoint/address evidence and publish canonical live ingress envelope bytes.

#### Scenario: Sender reaches bounded listener
- GIVEN a receiver listener with a bound live ticket
- AND a peer with admitted bootstrap and authority evidence
- WHEN live send publishes a status request
- THEN the listener records a live transport receive receipt
- AND the durable service loop processes the request.

### Requirement: Live send is not authority
r[molten.node_control_live_send_ux.spec.transport_non_authority] Live send receipts, live tickets, endpoint ids, endpoint addresses, and neighbor observations MUST NOT satisfy peer bootstrap, operation authority, policy/resource, idempotency, or payload provenance requirements.

#### Scenario: Send evidence does not authorize enqueue
- GIVEN a live send receipt and ticket evidence
- AND an envelope lacking admitted authority
- WHEN the receiver delivery gate runs
- THEN transport evidence does not satisfy authority
- AND enqueue is denied before side effects.

### Requirement: Live send fails closed
r[molten.node_control_live_send_ux.spec.fail_closed] Live send MUST fail closed with diagnostics when the receiver ticket is malformed, lacks endpoint addresses, has unsupported endpoint address forms, or cannot join the bounded topic.

#### Scenario: Offline ticket has no addresses
- GIVEN an offline exported live ticket without endpoint addresses
- WHEN live send runs
- THEN a deny send receipt is emitted
- AND no transport publish receipt is emitted.
