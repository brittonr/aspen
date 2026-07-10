# Authority Identity Revocation Delta: Claim-Scoped Capability Authority

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
