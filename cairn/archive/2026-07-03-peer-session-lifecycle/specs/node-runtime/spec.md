## ADDED Requirements

### Requirement: Node state stores peer session read model
r[molten.peer_session.node_state_table] Molten MUST store a bounded node-local peer read model that indexes peer sessions by peer id, node id, profile ref, ticket ref, admission ref, and admitted scope while preserving canonical receipts as the authority source.

#### Scenario: Status reads peer table without granting trust
- GIVEN a node state root contains a peer session read-model entry
- WHEN an operator runs peer status
- THEN the status output reports lifecycle state, refs, scopes, freshness, and diagnostics
- AND the read-model entry alone cannot satisfy authority, policy, provenance, source-gate, resource, retention, or execution gates.

### Requirement: Static peer config is contract-bound
r[molten.peer_session.nickel_config] Molten MUST validate static peer profile configuration through typed Nickel contracts before exporting runtime-consumed peer config, and runtime node operations MUST consume checked exports rather than evaluating Nickel live.

#### Scenario: Invalid peer config fails before runtime
- GIVEN a static peer profile uses an unsupported transport, malformed evidence ref, unsafe endpoint pattern, or contradictory resource bound
- WHEN the profile is exported through Nickel
- THEN export fails before node startup or peer connection can rely on that profile.

### Requirement: Live tickets bind into peer sessions
r[molten.peer_session.live_ticket_session_binding] Molten MUST bind existing live tickets, peer admissions, and imported authority grants into peer session records without changing their canonical receipt semantics or making imports authoritative by themselves.

#### Scenario: Ticket import updates session readback only
- GIVEN a sender imports a receiver live ticket and matching peer admission receipt
- WHEN the peer session read model updates
- THEN the session records the ticket and admission refs as bootstrap evidence
- AND operation authority remains absent until a matching authority grant is imported.

### Requirement: Peer lifecycle CLI wraps existing gates
r[molten.peer_session.peer_cli] Molten SHOULD expose peer invite, connect, status, revoke, and diagnose commands as thin shells over canonical peer-session, ticket, admission, authority, policy, resource, and replay evidence.

#### Scenario: Diagnose reports next missing live-send step
- GIVEN a sender has a receiver ticket but no matching peer admission in its state root
- WHEN `molten peer diagnose` runs for that peer and scope
- THEN it reports the missing peer admission import
- AND it does not attempt a live send or mutate authority state.

### Requirement: Peer lifecycle validation is reproducible
r[molten.peer_session.validation] Molten SHOULD validate peer-session lifecycle work with focused positive and negative tests, Nickel peer config fixtures, formatting, peer-related cargo tests, and Cairn validation before the change is archived.

#### Scenario: Validation catches stale ticket regression
- GIVEN a regression accepts an expired or wrong-topic peer ticket as an admitted session
- WHEN focused peer-session validation runs
- THEN the negative fixture fails
- AND the change cannot be marked complete until the denial is restored.
