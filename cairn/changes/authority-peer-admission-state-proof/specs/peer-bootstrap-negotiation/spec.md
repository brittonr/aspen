## ADDED Requirements

### Requirement: Live peer admission binds ticket scope
r[molten.peer_admission_state_proof.ticket_scope] Molten MUST prove that live peer admission accepts tickets only when node id, peer id, topic, endpoint, freshness, and policy evidence match the receiver and requested operation scope.

#### Scenario: Ticket for wrong topic denies
- GIVEN a live peer ticket issued for one topic
- WHEN the receiver imports or admits it for another topic
- THEN peer admission decision is `deny`
- AND the ticket cannot satisfy node-control ingress admission.

### Requirement: Transport identity is not bootstrap authority
r[molten.peer_admission_state_proof.transport_not_bootstrap] Molten MUST prove that observed transport identity, neighbor records, listener receipts, and live send receipts cannot replace explicit peer admission or bootstrap tickets.

#### Scenario: Neighbor observation cannot bootstrap
- GIVEN a live transport neighbor observation and no peer admission receipt
- WHEN node-control ingress evaluates bootstrap evidence
- THEN admission is denied before enqueue
- AND diagnostics state that transport evidence is not bootstrap authority.
