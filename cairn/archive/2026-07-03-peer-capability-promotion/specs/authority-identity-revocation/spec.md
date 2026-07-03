## ADDED Requirements

### Requirement: Peer capability promotion records are canonical
r[molten.peer_promotion.record_model] Molten MUST define canonical peer capability promotion request, promotion grant, promotion receipt, and demotion receipt records that bind target peer/session, current role evidence, requested role delta, issuer, approvals, attenuation, scope, expiry, revocation, policy refs, resource refs, and supporting evidence refs.

#### Scenario: Promotion receipt binds role delta
- GIVEN a peer requests promotion from subscriber to scoped publisher for one topic
- WHEN promotion preflight emits a receipt
- THEN the receipt binds the target peer/session ref, current role refs, requested role delta, promotion grant refs, policy/resource refs, decision, and diagnostics
- AND the receipt ref is derived from canonical Preserves bytes.

### Requirement: Promotion validates role deltas
r[molten.peer_promotion.role_delta] Molten MUST validate promotions as explicit role/capability deltas from current admitted capabilities to requested capabilities, preserving attenuation, scope, expiry, and revocation constraints.

#### Scenario: Over-broad target role denies
- GIVEN a promotion grant allows publishing to one topic
- WHEN a peer requests promotion to publish to all topics
- THEN promotion validation denies the over-broad delta
- AND diagnostics name the allowed and requested scopes.

### Requirement: Promotion authority is separate from target capability
r[molten.peer_promotion.authority_separation] Molten MUST require explicit promotion authority for role upgrades and MUST NOT satisfy promotion authority from transport identity, connected peer sessions, handoff bundles, subscription receipts, import receipts, or possession of the target capability alone.

#### Scenario: Handoff cannot promote peer
- GIVEN a peer handoff bundle imports a ticket and peer admission
- WHEN the peer requests promotion to node-control operator without a promotion grant
- THEN promotion denies
- AND diagnostics state that handoff evidence is not promotion authority.

### Requirement: Promotion approval policy is explicit
r[molten.peer_promotion.approval_policy] Molten SHOULD support policy-selected approval evidence for high-risk peer promotions, including operator review refs or multi-approval refs when the target role grants mutation, relay, import, execution, retention, or authority-delegation capabilities.

#### Scenario: High-risk promotion missing approval denies
- GIVEN policy requires approval refs for promoting a peer to a job-execution role
- WHEN the promotion request lacks the required approval evidence
- THEN preflight denies
- AND diagnostics name the missing approval class.

### Requirement: Self-escalation and transitive escalation deny
r[molten.peer_promotion.no_self_escalation] Molten MUST deny self-promotion, transitive escalation, stale grants, revoked issuers, and promotions that exceed the issuer's attenuated promotion authority unless explicit current policy admits that exact transition.

#### Scenario: Subscriber self-promotion denies
- GIVEN a subscriber holds no explicit self-promotion grant
- WHEN it requests promotion to publisher for its subscribed topic
- THEN promotion denies
- AND no write capability is added to the peer session.

### Requirement: Demotion cleans dependent authority surfaces
r[molten.peer_promotion.demotion_cleanup] Molten MUST provide demotion or revocation receipts that narrow peer capabilities and trigger cleanup or retraction of dependent subscriptions, live refs, handler bindings, queued jobs, and session read-model state.

#### Scenario: Demotion retracts subscriber projection
- GIVEN a peer is demoted from subscriber to no-read role
- WHEN the demotion receipt applies
- THEN active subscription projections for that peer are retracted or marked denied
- AND historical projection receipts remain evidence-only.

### Requirement: Peer promotion tests cover privilege boundaries
r[molten.peer_promotion.positive_negative_tests] Molten SHOULD include positive scoped promotion/demotion tests and negative tests for self-promotion, missing promotion authority, stale grant, revoked issuer, over-broad target, transitive escalation, subscriber write-upgrade, handoff-as-promotion, and Raft membership promotion.

#### Scenario: Stale promotion grant denies
- GIVEN a promotion grant has expired or is revoked
- WHEN promotion preflight validates it
- THEN the decision is deny
- AND diagnostics identify expiry or revocation as the cause.
