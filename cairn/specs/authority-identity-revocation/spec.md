# Authority Identity Revocation Specification

## Purpose

Defines the `authority-identity-revocation` capability.

## Requirements

### Requirement: System MUST Define canonical principal, node, actor, service, session, artifact, and execution identity records
r[molten.authority.identity_model] The system MUST Define canonical principal, node, actor, service, session, artifact, and execution identity records.

### Requirement: System MUST Enforce that identity records alone grant no authority
r[molten.authority.no_identity_authority] The system MUST Enforce that identity records alone grant no authority.

### Requirement: System MUST Define authority context records with capabilities, delegation chains, attenuation, expiry, revocation refs, key refs, policy refs, and evidence refs
r[molten.authority.context_model] The system MUST Define authority context records with capabilities, delegation chains, attenuation, expiry, revocation refs, key refs, policy refs, and evidence refs.

### Requirement: System MUST Document that human-readable names are metadata, not security identity
r[molten.authority.names_metadata] The system MUST Document that human-readable names are metadata, not security identity.

### Requirement: System MUST Require trust-boundary actions to carry and record an authority context before admission
r[molten.authority.admission_gate] The system MUST Require trust-boundary actions to carry and record an authority context before admission.

### Requirement: System MUST Define revocation targets for keys, principals, delegations, capabilities, live refs, handler bindings, sessions, and artifacts
r[molten.authority.revocation_model] The system MUST Define revocation targets for keys, principals, delegations, capabilities, live refs, handler bindings, sessions, and artifacts.

### Requirement: System MUST Retract assertions, subscriptions, live refs, and handler bindings when authority is lost
r[molten.authority.authority_cleanup] The system MUST Retract assertions, subscriptions, live refs, and handler bindings when authority is lost.

### Requirement: System MUST Emit Cairn receipts for admission, denial, revocation, expiry, key rotation, and cleanup
r[molten.authority.receipts] The system MUST Emit Cairn receipts for admission, denial, revocation, expiry, key rotation, and cleanup.

### Requirement: System MUST Gatekeeper resolution returns scoped, attenuated, expiring live refs with evidence refs
r[molten.authority.gatekeeper_resolution] The system MUST Gatekeeper resolution returns scoped, attenuated, expiring live refs with evidence refs.

### Requirement: System MUST Check authority contexts for effect handler binding and effect requests
r[molten.authority.effect_integration] The system MUST Check authority contexts for effect handler binding and effect requests.

### Requirement: System MUST Apply authority contexts to typed storage, remote sync/execution, and catalog visibility
r[molten.authority.storage_remote_catalog] The system MUST Apply authority contexts to typed storage, remote sync/execution, and catalog visibility.

### Requirement: System MUST Ensure replay verifies recorded authority decisions without minting new current authority
r[molten.authority.replay_scope] The system MUST Ensure replay verifies recorded authority decisions without minting new current authority.

### Requirement: System MUST Add tests that revocation retracts dependent assertions and denies future effect requests
r[molten.authority.revocation_tests] The system MUST Add tests that revocation retracts dependent assertions and denies future effect requests.

### Requirement: System MUST Add tests for expiry using admitted logical clock sources
r[molten.authority.expiry_tests] The system MUST Add tests for expiry using admitted logical clock sources.

### Requirement: System MUST Add tests for key rotation preserving historical verification without current authority
r[molten.authority.rotation_tests] The system MUST Add tests for key rotation preserving historical verification without current authority.

### Requirement: System MUST Add Hegel property tests for attenuation monotonicity, no identity-as-authority, and cleanup invariants
r[molten.authority.property_tests] The system MUST Add Hegel property tests for attenuation monotonicity, no identity-as-authority, and cleanup invariants.

### Requirement: Authority grant state admits only current scoped capability
r[molten.authority_peer_state_proof.current_scoped_grant] Molten MUST prove that authority admission accepts a grant only when principal, capability, operation, scope, epoch, attenuation, expiry, key currentness, and revocation evidence all match the requested action.

#### Scenario: Revoked delegation denies
- GIVEN an authority context with a delegation ref listed in current revocation evidence
- WHEN a trust-boundary action evaluates that authority context
- THEN the admission decision is `deny`
- AND no side effect is admitted from the revoked delegation.

### Requirement: Authority imports do not mint authority by possession
r[molten.authority_peer_state_proof.import_not_authority] Molten MUST prove that imported grant, key, ticket, or receipt artifacts remain evidence candidates and do not become current authority until the normal authority admission state machine passes for the requested operation.

#### Scenario: Imported grant wrong scope denies
- GIVEN an imported authority grant for one operation scope
- WHEN node control evaluates a different operation scope
- THEN authority admission is `deny`
- AND diagnostics identify the scope mismatch.

### Requirement: Authority replay preserves history without current authority
r[molten.authority_peer_state_proof.replay_no_current_authority] Molten MUST prove that replay verification can validate historical authority decisions without treating old receipts, expired keys, or revoked delegations as current authority.

#### Scenario: Historical receipt cannot authorize new action
- GIVEN a historical passing authority receipt whose key is now revoked
- WHEN a new trust-boundary action attempts to reuse that receipt as current authority
- THEN current admission is `deny`
- AND replay may still validate the historical receipt as evidence-only.

### Requirement: Capability tokens and proofsets are canonical
r[molten.capability_token.record_model] Molten MUST define canonical capability token, capability proofset, and capability admission receipt records that bind issuer, holder, session or actor context, resource, ability, operation, scope, attenuation, caveats, freshness, expiry, revocation, key-currentness, policy refs, resource refs, delegation chain refs, and evidence refs.

#### Scenario: Token admission binds holder and ability
- GIVEN a capability proofset for a peer publishing to one topic
- WHEN Molten emits a capability admission receipt
- THEN the receipt binds the holder, peer/session ref, resource, ability, scope, selected token refs, caveat decisions, revocation/currentness checks, policy/resource refs, decision, and diagnostics
- AND the receipt ref is derived from canonical Preserves bytes.

### Requirement: Capability evidence taxonomy is explicit
r[molten.capability_token.taxonomy] Molten MUST distinguish identity refs, transport receipts, peer sessions, handoff bundles, bootstrap tickets, read tokens, write tokens, promotion tokens, authority tokens, and membership evidence so that only admitted capability tokens or proofsets can authorize privileged actions.

#### Scenario: Peer session is not a token
- GIVEN a peer session records a connected peer and admitted bootstrap evidence
- WHEN the peer requests a side-effecting operation without a matching capability token or authority grant
- THEN authorization denies
- AND diagnostics classify the session as context rather than a capability token.

### Requirement: Capability admission is not bearer-only
r[molten.capability_token.admission_law] Molten MUST validate capability proofsets at use time against exact holder, session or actor context, resource, ability, scope, attenuation, caveats, expiry, revocation, key-currentness, policy, resource, and subsystem constraints.

#### Scenario: Wrong holder denies
- GIVEN a valid token was issued to one peer session
- WHEN another peer session presents the token for the same operation and scope
- THEN capability admission denies
- AND diagnostics identify the holder or session mismatch.

### Requirement: Imported capability artifacts do not mint authority
r[molten.capability_token.import_not_authority] Molten MUST treat imported capability tokens, imported proofsets, handoff-carried tokens, and historical capability receipts as evidence candidates only until the current capability admission law passes for the requested action.

#### Scenario: Imported token wrong scope denies
- GIVEN a state root imports a token for one topic
- WHEN the holder requests a different topic scope
- THEN capability admission denies
- AND the imported artifact remains stored only as evidence.

### Requirement: Basalt/UCAN proof seam is preserved
r[molten.capability_token.basalt_ucan_seam] Molten MUST keep the capability verifier structured so Basalt/UCAN proofs, caveats, revocation evidence, and authority receipts can replace or augment local deterministic fixtures without weakening fail-closed admission.

#### Scenario: Missing future UCAN caveat evidence denies
- GIVEN a future UCAN-backed proofset requires caveat evidence
- WHEN the caveat evidence is missing or stale
- THEN capability admission denies
- AND no local fallback grants ambient authority.

### Requirement: UCAN verification receipts define current capability authority
r[molten.capability_token.ucan_verified_authority] Molten MUST treat non-empty UCAN proofsets as current capability authority only after UCAN verification receipts bind the compact token refs, proof refs, verification key evidence, caveat decisions, revocation facts, replay facts, derived grant refs, requested holder/session/context, resource, ability, scope, and request ref.

#### Scenario: Verified UCAN proofset admits requested grant
- GIVEN a UCAN proofset whose leaf token, proof chain, caveats, revocation facts, replay facts, holder, session, context, resource, ability, and scope all match a requested action
- WHEN capability admission evaluates the proofset
- THEN it emits a passing UCAN verification receipt and derived grant refs bound to that request
- AND only those derived grant refs are eligible for Basalt enforcement.

#### Scenario: Missing proof receipt denies authority
- GIVEN a non-empty UCAN proofset that references proof material
- WHEN the matching proof traversal or verification receipt is missing, stale, malformed, or bound to another request
- THEN capability admission denies before any derived grant can authorize the action.

### Requirement: Local capability fixtures are not parallel production authority
r[molten.capability_token.fixture_not_parallel_authority] Local deterministic capability token fixtures MAY support harness-only tests, but Molten MUST NOT treat local fixture grants, imported token records, or historical admission receipts as a production authority path parallel to current verified UCAN/Basalt admission.

#### Scenario: Fixture fallback cannot satisfy UCAN-backed request
- GIVEN an operation declares that its authority source is a UCAN proofset
- WHEN UCAN verification fails or is missing
- THEN Molten denies the request even if a local fixture grant with matching resource and ability exists.

#### Scenario: Historical receipt remains evidence only
- GIVEN a historical passing capability admission receipt whose token is now expired or revoked
- WHEN a new request attempts to reuse that receipt as current authority
- THEN Molten denies current admission and may use the old receipt only for replay or audit evidence.

### Requirement: Capability diagnostics are actionable
r[molten.capability_token.diagnostics] Molten SHOULD emit diagnostics that identify missing or mismatched holder, session, resource, ability, scope, caveat, expiry, revocation, policy, resource, issuer, delegation chain, token kind, and subsystem boundary evidence.

#### Scenario: Wrong ability diagnostic names token ability
- GIVEN a token admits read projection but the request attempts publish
- WHEN capability admission evaluates the proofset
- THEN the decision is deny
- AND diagnostics identify the admitted read ability and requested publish ability.

### Requirement: Capability token tests cover authority boundaries
r[molten.capability_token.positive_negative_tests] Molten SHOULD include positive token admission fixtures and negative tests for bearer-only use, wrong holder, wrong session, wrong operation, over-broad scope, expired tokens, revoked issuers or delegations, caveat failure, missing policy/resource, token import as authority, and handoff/session/transport-as-token attempts.

#### Scenario: Bearer-only fixture denies
- GIVEN a request presents a token value without matching holder/session/context evidence
- WHEN capability admission evaluates the request
- THEN the decision is deny
- AND diagnostics state that bearer-only use is not admitted.

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

### Requirement: Claim authority uses existing capability admission
r[molten.claim_authority.capability_profile] Molten MUST represent external claim authority as a capability-token profile over holder, session, context, resource, ability, scope, attenuation, caveats, expiry, revocation refs, policy refs, resource refs, delegation refs, and evidence refs rather than as a new trust primitive.

#### Scenario: Friend cluster can attest a narrow claim
- GIVEN a peer session for an external cluster
- AND a capability proofset whose holder/session/context match that cluster
- AND the token ability is `claim:attest`
- AND the token resource ref names the subject selector for the claim domain
- AND the token scope matches the requested claim kind
- WHEN UCAN verification, Basalt enforcement, and capability admission all pass
- THEN Molten may emit a passing claim admission receipt for that exact claim.

#### Scenario: Missing UCAN or Basalt evidence denies
- GIVEN an external cluster presents a claim and matching peer session evidence
- WHEN the UCAN verification receipt, Basalt enforcement receipt, or capability admission receipt is missing or denied
- THEN claim authority admission is `deny`
- AND diagnostics identify the missing capability proof path.

### Requirement: Authority claim records are canonical evidence
r[molten.claim_authority.claim_records] Molten MUST define canonical `authority-claim-v1` and `authority-claim-admission-v1` records that bind issuer, holder, peer/session context, subject selector, claim kind, claim value, policy/resource refs, freshness, revocation, capability admission refs, UCAN verification refs, Basalt enforcement refs, decision, diagnostics, and checks.

#### Scenario: Claim admission binds proof receipts
- GIVEN an external claim has a matching capability admission receipt and UCAN/Basalt proof receipts
- WHEN Molten admits the claim
- THEN the `authority-claim-admission-v1` receipt binds the claim ref, requested resource/ability/scope, admitted token refs, UCAN verification refs, Basalt enforcement refs, policy/resource refs, and decision
- AND the receipt ref is derived from canonical Preserves bytes.

#### Scenario: Historical claim receipt is evidence-only
- GIVEN a historical passing claim admission whose token or issuer is now expired or revoked
- WHEN a new claim-use decision attempts to reuse that historical receipt as current authority
- THEN current admission denies unless fresh capability/UCAN/Basalt evidence passes for the new request.

### Requirement: External claim authority is not parallel trust
r[molten.claim_authority.no_parallel_trust] Molten MUST NOT treat transport identity, connected peer sessions, handoff bundles, import receipts, ledger possession, local fixture grants, catalog discovery, or human-readable friend labels as external claim authority without current capability admission and UCAN/Basalt proof evidence.

#### Scenario: Peer session alone cannot attest
- GIVEN a connected peer session for a friendly cluster
- AND no admitted `claim:attest` capability proofset for the requested subject selector and claim kind
- WHEN the cluster presents an authority claim
- THEN claim admission is `deny`
- AND diagnostics state that the peer session is context, not claim authority.

#### Scenario: Local fixture fallback cannot satisfy production claim
- GIVEN an operation declares that claim authority must come from UCAN/Basalt-backed evidence
- WHEN UCAN or Basalt proof verification fails
- THEN Molten denies the claim even if a local deterministic fixture grant has matching resource and scope.

### Requirement: Downstream claim use is exact and explicit
r[molten.claim_authority.downstream_consumption] Subsystem gates that consume admitted external claims MUST check the claim admission decision, subject selector, claim kind, freshness, policy/resource refs, and subsystem-specific caveats before using the claim; claim admission MUST NOT grant unrelated authority by itself.

#### Scenario: Class claim does not grant execution
- GIVEN a passing claim admission says an artifact belongs to a trusted input class
- WHEN a peer requests job execution for that artifact
- THEN execution still requires the normal execution, provenance, policy, resource, and source-gate evidence
- AND the class claim can only satisfy the exact class-membership predicate selected by the execution policy.

#### Scenario: Wrong claim kind denies use
- GIVEN a passing claim admission for `class-membership`
- WHEN a release gate requires `release-channel-attestation`
- THEN downstream claim use denies with a claim-kind mismatch diagnostic.

### Requirement: Claim authority tests cover proof boundaries
r[molten.claim_authority.positive_negative_tests] Molten SHOULD include positive tests for admitted external claims and negative tests for missing proof, wrong holder, wrong session, wrong context, wrong selector, wrong claim kind, revoked issuer or delegation, stale proof, over-broad wildcard, transport-only evidence, registry-only discovery, and local-fixture fallback attempts.

#### Scenario: Transport-only claim fixture denies
- GIVEN a fixture contains live neighbor, listener, or send receipts from an external cluster
- AND no admitted `claim:attest` capability proofset for the claim
- WHEN claim admission evaluates the fixture
- THEN the decision is `deny`
- AND diagnostics say transport evidence is not claim authority.
