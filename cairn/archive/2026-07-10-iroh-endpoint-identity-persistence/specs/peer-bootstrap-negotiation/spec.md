# Peer Bootstrap Negotiation Delta: Iroh Identity Binding

### Requirement: Iroh endpoint identity is transport binding, not authority
r[molten.peer_bootstrap.iroh_identity_transport_boundary] Peer bootstrap handshakes that include Iroh endpoint public identity refs MUST treat those refs as transport binding and diagnostics only; operation authority still requires explicit peer admission, capability, authority, policy, resource, replay/idempotency, and subsystem-specific evidence.

#### Scenario: Matching endpoint identity supports bootstrap binding
- GIVEN a peer handshake includes a node identity ref and matching Iroh endpoint public identity ref
- WHEN peer bootstrap admission evaluates transport binding
- THEN it may bind the endpoint identity into the peer session diagnostics and receipts.

#### Scenario: Endpoint identity alone cannot authorize operation
- GIVEN a peer presents a known Iroh endpoint public identity but lacks operation authority evidence
- WHEN the peer submits a node-control or protocol operation
- THEN admission denies before side effects and diagnostics state that endpoint identity is not operation authority.