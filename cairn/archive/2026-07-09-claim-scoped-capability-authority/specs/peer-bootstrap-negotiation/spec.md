# Peer Bootstrap Negotiation Delta: Claim Authority Diagnostics

### Requirement: Peer claim authority is separate from peer admission
r[molten.claim_authority.peer_diagnostics] Peer diagnostics SHOULD report external claim authority as a separate gate from transport reachability, peer bootstrap admission, peer session lifecycle, handoff import, authority grants, policy/resource admission, provenance, replay/idempotency, and execution readiness.

#### Scenario: Friendly peer lacks claim authority
- GIVEN a peer session is connected and bootstrap-admitted
- AND no admitted `claim:attest` capability proofset exists for the requested claim domain
- WHEN `molten peer diagnose` or equivalent readback evaluates the peer
- THEN diagnostics report bootstrap/session as present and claim authority as missing
- AND the next step names the needed capability/UCAN/Basalt proof or claim admission.

### Requirement: Peer sessions can be claim context but not claim authority
r[molten.claim_authority.peer_session_context] Peer sessions, live tickets, peer admissions, and handoff bundles MAY be referenced as holder/session/context evidence for claim capability requests, but MUST NOT satisfy claim authority without matching capability admission and UCAN/Basalt proof receipts.

#### Scenario: Session-bound proof admits claim context
- GIVEN a capability proofset holder and session match a connected peer session
- AND UCAN/Basalt/capability admission passes for `claim:attest` on the requested selector and scope
- WHEN an external claim is admitted
- THEN the claim admission may bind the peer session ref as context evidence.

#### Scenario: Handoff import alone cannot attest
- GIVEN a peer handoff bundle imports a ticket and peer admission for an external cluster
- WHEN the peer presents a claim without matching claim authority proof
- THEN claim admission denies
- AND diagnostics state that handoff evidence is not a claim-attestation capability.

### Requirement: Peer claim diagnostics have positive and negative tests
r[molten.claim_authority.peer_diagnostic_tests] Peer diagnostic tests SHOULD include positive claim-authority readback and negative cases for missing proof, stale session, revoked peer profile, handoff-only evidence, transport-only evidence, wrong selector, and wrong claim kind.

#### Scenario: Transport-only diagnostic names missing proof
- GIVEN only live transport evidence exists for a peer
- WHEN peer diagnostics evaluate external claim authority
- THEN diagnostics classify transport as reachable or observed only
- AND report claim authority denied until capability/UCAN/Basalt evidence passes.
