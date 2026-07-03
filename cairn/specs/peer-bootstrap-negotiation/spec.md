# Peer Bootstrap Negotiation Specification

## Purpose

Defines the `peer-bootstrap-negotiation` capability.

## Requirements

### Requirement: System MUST Define canonical bootstrap inputs for static peers, invites, endpoint ids, local discovery, catalog records, gatekeeper credentials, and control-plane membership
r[molten.peer_bootstrap.bootstrap_inputs] The system MUST Define canonical bootstrap inputs for static peers, invites, endpoint ids, local discovery, catalog records, gatekeeper credentials, and control-plane membership.

### Requirement: System MUST Define handshake records with node ids, versions, artifact/schema/effect/transport support, resources, replay support, requested groups, capabilities, and policy refs
r[molten.peer_bootstrap.handshake_record] The system MUST Define handshake records with node ids, versions, artifact/schema/effect/transport support, resources, replay support, requested groups, capabilities, and policy refs.

### Requirement: System MUST Document that Iroh transport identity alone is not Molten authority
r[molten.peer_bootstrap.no_transport_authority] The system MUST Document that Iroh transport identity alone is not Molten authority.

### Requirement: System MUST Emit receipts for handshake start, negotiation result, join admission, denial, and disconnect
r[molten.peer_bootstrap.receipts] The system MUST Emit receipts for handshake start, negotiation result, join admission, denial, and disconnect.

### Requirement: System MUST Define feature vectors for runtime version, registry protocol, schema identity, Preserves boundary, handler profiles, transport support, and replay support
r[molten.peer_bootstrap.feature_vector] The system MUST Define feature vectors for runtime version, registry protocol, schema identity, Preserves boundary, handler profiles, transport support, and replay support.

### Requirement: System MUST Select the highest mutually admitted feature set and deny unsafe downgrades unless policy explicitly allows them
r[molten.peer_bootstrap.negotiation_policy] The system MUST Select the highest mutually admitted feature set and deny unsafe downgrades unless policy explicitly allows them.

### Requirement: System MUST Represent capability offers and requests as scoped, attenuated, expiring, policy-gated records
r[molten.peer_bootstrap.capability_offers] The system MUST Represent capability offers and requests as scoped, attenuated, expiring, policy-gated records.

### Requirement: System MUST Include negotiated resource limits and quotas in join agreements
r[molten.peer_bootstrap.resource_limits] The system MUST Include negotiated resource limits and quotas in join agreements.

### Requirement: System MUST Gate gossip topic and Iroh docs namespace joins through negotiated agreements and authority checks
r[molten.peer_bootstrap.topic_doc_join] The system MUST Gate gossip topic and Iroh docs namespace joins through negotiated agreements and authority checks.

### Requirement: System MUST Use peer agreements to determine remote artifact sync and catalog visibility behavior
r[molten.peer_bootstrap.remote_sync_join] The system MUST Use peer agreements to determine remote artifact sync and catalog visibility behavior.

### Requirement: System MUST Gate protocol sessions and job pools through peer agreements
r[molten.peer_bootstrap.protocol_job_join] The system MUST Gate protocol sessions and job pools through peer agreements.

### Requirement: System MUST Define how future Raft/control-plane membership joins use stronger admission
r[molten.peer_bootstrap.raft_join_plan] The system MUST Define how future Raft/control-plane membership joins use stronger admission.

### Requirement: System MUST Add loopback handshake tests for compatible feature negotiation and join admission
r[molten.peer_bootstrap.loopback_tests] The system MUST Add loopback handshake tests for compatible feature negotiation and join admission.

### Requirement: System MUST Add tests that unsafe downgrade attempts are denied
r[molten.peer_bootstrap.downgrade_tests] The system MUST Add tests that unsafe downgrade attempts are denied.

### Requirement: System MUST Add tests that capability offers do not grant authority until accepted and admitted
r[molten.peer_bootstrap.capability_tests] The system MUST Add tests that capability offers do not grant authority until accepted and admitted.

### Requirement: System MUST Add Hegel property tests for negotiation determinism, no-ambient-authority, and denied-join safety
r[molten.peer_bootstrap.property_tests] The system MUST Add Hegel property tests for negotiation determinism, no-ambient-authority, and denied-join safety.

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
