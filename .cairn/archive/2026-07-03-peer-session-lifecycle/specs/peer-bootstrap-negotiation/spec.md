## ADDED Requirements

### Requirement: Peer session lifecycle records are canonical
r[molten.peer_session.lifecycle_model] Molten MUST define canonical peer profile and peer session records that bind peer identity, endpoint expectations, negotiated joins, admitted scopes, resource bounds, freshness, revocation state, lifecycle state, and evidence refs.

#### Scenario: Session records bind admitted evidence
- GIVEN a peer has a matching profile, live ticket, peer admission receipt, negotiated agreement, and resource evidence
- WHEN Molten builds the peer session record
- THEN the session binds the profile ref, identity refs, ticket refs, admission refs, admitted scopes, resource refs, freshness data, and lifecycle state
- AND the session ref is derived from canonical Preserves bytes.

### Requirement: Peer lifecycle transitions are explicit
r[molten.peer_session.lifecycle_transitions] Molten MUST advance peer sessions only through explicit lifecycle transition receipts and MUST deny invalid skips, stale evidence, revoked evidence, and quarantine bypasses.

#### Scenario: Quarantined peer cannot reconnect implicitly
- GIVEN a peer session is marked quarantined by a canonical transition receipt
- WHEN a live transport neighbor observation or send receipt is observed for that peer
- THEN the session remains quarantined
- AND reconnect requires an explicit admitted transition out of quarantine.

### Requirement: Peer sessions do not grant authority
r[molten.peer_session.authority_boundary] Molten MUST NOT treat a peer profile, peer agreement, peer session, live endpoint, topic membership, or transport observation as authority for side-effecting operations.

#### Scenario: Connected peer lacks operation authority
- GIVEN a peer session is connected and has a valid live ticket admission
- WHEN the peer submits a node-control operation without a matching authority grant
- THEN ingress denies before enqueue or side effects
- AND diagnostics state that the peer session is not operation authority.

### Requirement: Peer diagnostics explain missing gates
r[molten.peer_session.diagnostics] Molten SHOULD produce peer diagnostics that separately report transport reachability, bootstrap admission, capability admission, authority grant, policy/resource admission, replay/idempotency status, and the next missing operator step.

#### Scenario: Missing authority is actionable
- GIVEN a peer has a valid session and bootstrap admission but no imported authority grant for the requested operation
- WHEN `molten peer diagnose` evaluates the peer
- THEN the diagnostic identifies bootstrap as present and authority as missing
- AND it names the import or grant step required before live send can pass.

### Requirement: Peer session tests cover positive and negative evidence
r[molten.peer_session.positive_negative_tests] Molten SHOULD cover peer sessions with positive lifecycle tests and negative tests for stale tickets, wrong topics, missing admissions, missing authority, revoked profiles, unsafe static config, and transport-only evidence.

#### Scenario: Transport-only negative fixture denies
- GIVEN a fixture contains live neighbor or send transport evidence but no peer admission receipt
- WHEN peer session validation evaluates bootstrap state
- THEN validation denies admission
- AND the diagnostic says transport evidence is not bootstrap authority.
