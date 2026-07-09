# Peer Bootstrap Negotiation Delta: Explicit Peer Session Transition Relation

### Requirement: Peer session transition relation is closed
r[molten.peer_session.transition_relation_closed] Molten MUST define a reviewed finite peer-session transition relation over prior state, requested event, target state, and explicit guard facts, and MUST deny any peer-session transition not present in that relation before advancing session state.

#### Scenario: Admitted peer reaches connected through reviewed steps
- GIVEN a peer session has passed discovery, invitation, handshake, negotiation, bootstrap admission, and authority/resource guard checks
- WHEN the connect event is evaluated against the reviewed transition relation
- THEN Molten emits a passing transition receipt
- AND the after-state is connected with the guard evidence refs bound.

#### Scenario: Discovered peer cannot jump to connected
- GIVEN a peer session is only discovered and has no admitted bootstrap or negotiation evidence
- WHEN a connected target state is requested
- THEN the transition decision is deny
- AND the prior session state ref remains unchanged.

### Requirement: Terminal and quarantine states require explicit recovery
r[molten.peer_session.terminal_quarantine_guards] Molten MUST prevent expired, revoked, or quarantined peer sessions from becoming admitted or connected again unless an explicit recovery or re-admission event with current policy, freshness, revocation, bootstrap, and resource evidence passes.

#### Scenario: Revoked peer cannot reconnect from transport observation
- GIVEN a peer session is revoked
- WHEN a live transport neighbor observation or send receipt is supplied as reconnect evidence
- THEN the transition decision is deny
- AND diagnostics state that transport evidence cannot exit the revoked state.

#### Scenario: Quarantined peer recovers with explicit evidence
- GIVEN a peer session is quarantined and current policy permits a recovery workflow
- WHEN recovery evidence, current freshness, revocation checks, bootstrap admission, and resource evidence are supplied
- THEN Molten may emit a passing recovery transition receipt to the reviewed recovery target state.

### Requirement: Peer transition receipts bind state refs
r[molten.peer_session.transition_receipt_binding] Peer-session transition receipts MUST bind the peer/session identity, from-state, requested event, target-state, before-state ref, after-state ref or preserved-state ref, guard evidence refs, decision, diagnostics, and an evidence-only caveat that peer session state does not grant operation authority.

#### Scenario: Denied transition preserves state ref
- GIVEN a peer transition request is denied for a wrong topic or missing guard evidence
- WHEN the transition receipt is emitted
- THEN the receipt binds the original before-state ref as the preserved state
- AND no connected, admitted, or authority-bearing state is minted by the denial.

### Requirement: Peer transition tests cover the relation
r[molten.peer_session.transition_trace_tests] Molten SHOULD include positive and negative peer-session transition tests, including bounded generated traces, that cover reviewed state progression, invalid skips, wrong topics, stale tickets, revoked evidence, quarantine bypass, missing admissions, missing authority, and transport-only evidence.

#### Scenario: Generated peer trace rejects invalid edge
- GIVEN a generated peer-session transition trace includes a state/event pair outside the reviewed relation
- WHEN the trace is evaluated
- THEN the invalid edge emits a deny receipt
- AND all later state assertions derive from the preserved prior state.