## Phase 1: Bootstrap and handshake model

- [x] [serial] r[molten.peer_bootstrap.bootstrap_inputs] Define canonical bootstrap inputs for static peers, invites, endpoint ids, local discovery, catalog records, gatekeeper credentials, and control-plane membership.
- [x] [serial] r[molten.peer_bootstrap.handshake_record] Define handshake records with node ids, versions, artifact/schema/effect/transport support, resources, replay support, requested groups, capabilities, and policy refs.
- [x] [parallel] r[molten.peer_bootstrap.no_transport_authority] Document that Iroh transport identity alone is not Molten authority.
- [x] [parallel] r[molten.peer_bootstrap.receipts] Emit receipts for handshake start, negotiation result, join admission, denial, and disconnect.

## Phase 2: Negotiation and capabilities

- [x] [serial] r[molten.peer_bootstrap.feature_vector] Define feature vectors for runtime version, registry protocol, schema identity, Preserves boundary, handler profiles, transport support, and replay support.
- [x] [serial] r[molten.peer_bootstrap.negotiation_policy] Select the highest mutually admitted feature set and deny unsafe downgrades unless policy explicitly allows them.
- [x] [serial] r[molten.peer_bootstrap.capability_offers] Represent capability offers and requests as scoped, attenuated, expiring, policy-gated records.
- [x] [parallel] r[molten.peer_bootstrap.resource_limits] Include negotiated resource limits and quotas in join agreements.

## Phase 3: Join admission

- [x] [serial] r[molten.peer_bootstrap.topic_doc_join] Gate gossip topic and Iroh docs namespace joins through negotiated agreements and authority checks.
- [x] [parallel] r[molten.peer_bootstrap.remote_sync_join] Use peer agreements to determine remote artifact sync and catalog visibility behavior.
- [x] [parallel] r[molten.peer_bootstrap.protocol_job_join] Gate protocol sessions and job pools through peer agreements.
- [x] [parallel] r[molten.peer_bootstrap.raft_join_plan] Define how future Raft/control-plane membership joins use stronger admission.

## Phase 4: Tests

- [x] [serial] r[molten.peer_bootstrap.loopback_tests] Add loopback handshake tests for compatible feature negotiation and join admission.
- [x] [serial] r[molten.peer_bootstrap.downgrade_tests] Add tests that unsafe downgrade attempts are denied.
- [x] [parallel] r[molten.peer_bootstrap.capability_tests] Add tests that capability offers do not grant authority until accepted and admitted.
- [x] [parallel] r[molten.peer_bootstrap.property_tests] Add Hegel property tests for negotiation determinism, no-ambient-authority, and denied-join safety.
