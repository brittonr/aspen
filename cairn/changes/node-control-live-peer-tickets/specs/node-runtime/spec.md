# Node Runtime Delta: Live Peer Tickets

### Requirement: Live tickets are canonical bootstrap artifacts
r[molten.node_control_live_peer_tickets.spec.ticket_artifacts] Node-control live endpoint tickets MUST be represented by canonical `node-control-live-ticket-v1` artifacts that bind node id, node identity ref, logical endpoint id, live endpoint id, topic, exported addresses, policy refs, evidence refs, and non-authority checks.

#### Scenario: Ticket is exportable
- GIVEN an initialized node
- WHEN a live ticket is exported for a node-control topic
- THEN the ticket has a stable artifact ref
- AND it is imported into the node ledger as bootstrap evidence only.

### Requirement: Peer admissions are canonical receipts
r[molten.node_control_live_peer_tickets.spec.peer_admission_artifacts] Node-control live peer admissions MUST be represented by canonical `node-control-live-peer-admission-v1` receipts that bind peer id, ticket ref, node id, topic, sequence/expiry, policy refs, evidence refs, diagnostics, and non-authority checks.

#### Scenario: Peer is admitted from a ticket
- GIVEN a live ticket matching the local node
- WHEN an operator admits a peer against the ticket
- THEN a pass admission receipt is written
- AND the admission receipt is imported into the node ledger.

### Requirement: Ticket CLI is available
r[molten.node_control_live_peer_tickets.spec.ticket_cli] The CLI MUST expose ticket export for offline bootstrap and bound listener ticket output for live serve sessions.

#### Scenario: Serve writes bound ticket
- GIVEN a running node
- WHEN `molten node serve --live-iroh --live-ticket-out` starts
- THEN it writes a parseable live ticket
- AND the listener receipt still records bounded listener evidence.

### Requirement: Peer admit CLI is available
r[molten.node_control_live_peer_tickets.spec.peer_admit_cli] The CLI MUST expose peer admission from a ticket without granting operation authority.

#### Scenario: Peer admission helper writes receipt
- GIVEN a live ticket file
- WHEN `molten node live-peer-admit` is run for a peer
- THEN it emits a canonical admission receipt
- AND the receipt states authority is still required.

### Requirement: Live ingress resolves ticket admissions before enqueue
r[molten.node_control_live_peer_tickets.spec.live_pre_enqueue_gate] Live node-control ingress MUST resolve peer bootstrap refs to admitted peer ticket receipts before delivery idempotency or queue side effects.

#### Scenario: Admitted peer may reach authority gate
- GIVEN a live envelope from a peer with an admitted ticket receipt
- WHEN ingress delivery runs
- THEN peer bootstrap passes
- AND the envelope may proceed to authority delegation and idempotency checks.

### Requirement: Peer ticket checks fail closed
r[molten.node_control_live_peer_tickets.spec.fail_closed] Live peer bootstrap MUST deny before side effects when the admission is unknown, not a peer admission, denied, bound to the wrong peer/node/topic, not yet valid, or expired.

#### Scenario: Wrong peer is denied
- GIVEN a live envelope from `peer:other`
- AND the bootstrap ref admits `peer:operator`
- WHEN ingress delivery runs
- THEN no request is enqueued
- AND the ingress receipt contains wrong-peer diagnostics.

### Requirement: Tickets are not authority
r[molten.node_control_live_peer_tickets.spec.transport_non_authority] Live tickets, peer admissions, Iroh endpoint ids, and neighbor observations MUST NOT satisfy node-control operation authority or payload provenance.

#### Scenario: Ticket does not grant operation authority
- GIVEN a live envelope with an admitted peer ticket but no admitted authority grant
- WHEN ingress delivery runs
- THEN peer bootstrap may pass
- BUT authority delegation denies before enqueue.
