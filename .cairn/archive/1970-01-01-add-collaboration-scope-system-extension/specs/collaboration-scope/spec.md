# Collaboration Scope

## ADDED Requirements

### Requirement: Collaboration scope is a versioned system extension

r[molten.collaboration_scope.versioned_extension] Molten MUST implement application collaboration scopes as a separately versioned system extension with explicit fabric-port bindings. It MUST NOT change fabric node-membership semantics or infer collaboration membership from transport, placement, actor, session, UI, or ordinary dataspace facts.

#### Scenario: Valid extension manifest is admitted

r[molten.collaboration_scope.versioned_extension.admitted]
- GIVEN a supported collaboration-scope manifest with exact schema, profile, port, resource, policy, and limit references
- WHEN system-extension admission runs
- THEN Molten MUST admit only the declared collaboration service generation
- AND existing fabric membership MUST remain unchanged.

#### Scenario: Fabric membership is presented as collaboration membership

r[molten.collaboration_scope.versioned_extension.fabric-denied]
- GIVEN a node, peer, transport, placement, service-role, or actor-membership fact without a collaboration membership record
- WHEN scoped resource admission runs
- THEN the request MUST be denied before resource or product effects.

### Requirement: Collaboration records use nominal bounded identities

r[molten.collaboration_scope.nominal_model] Scope, subject, membership, resource-binding, audience, request, operation, policy, decision, snapshot, and receipt references MUST use distinct nominal domains with checked grammar and bounded canonical Preserves projections. Display labels and one domain's reference MUST NOT satisfy another domain.

#### Scenario: Valid nominal records are admitted

r[molten.collaboration_scope.nominal_model.valid]
- GIVEN a bounded scope record uses the correct nominal reference for every field
- WHEN pure record admission runs
- THEN Molten MUST produce the canonical admitted model without changing its wire meaning.

#### Scenario: Node reference is used as a subject

r[molten.collaboration_scope.nominal_model.domain-mismatch]
- GIVEN a node, session, resource, policy, or display-label value is supplied where a collaboration subject is required
- WHEN pure record admission runs
- THEN admission MUST fail with a stable domain-mismatch diagnostic.

### Requirement: Membership currentness is explicit

r[molten.collaboration_scope.membership_currentness] Every scope MUST carry a monotonic membership epoch and current scope revision. Add, remove, expiry, and role-change transitions MUST bind an exact operation, authority decision, active service generation, consistency epoch, and before-state reference. Later admission MUST reject stale membership facts.

#### Scenario: Member removal invalidates a later stale request

r[molten.collaboration_scope.membership_currentness.removed]
- GIVEN a member was removed and a higher membership epoch committed
- WHEN a request presents the earlier epoch
- THEN Molten MUST deny the request without restoring or inferring membership.

#### Scenario: Current member is admitted

r[molten.collaboration_scope.membership_currentness.current]
- GIVEN an active member, current scope revision, current membership epoch, and matching authority facts
- WHEN membership admission runs
- THEN Molten MAY admit that membership fact for the exact requested decision.

### Requirement: Each resource has one current owner scope

r[molten.collaboration_scope.resource_binding] A scoped resource binding MUST identify one current owner scope, owner subject, binding revision, resource policy, required abilities, operation identity, and evidence references. Copying or moving a binding to another scope MUST require an explicit current authorized transition and MUST NOT follow from names, ancestry, membership, read access, or destination visibility.

#### Scenario: Authorized scope move commits

r[molten.collaboration_scope.resource_binding.move]
- GIVEN current source and target scopes, exact move authority, target policy admission, and current consistency evidence
- WHEN the move transition commits
- THEN the resource MUST have one new owner-scope binding
- AND the earlier binding MUST no longer be current.

#### Scenario: Implicit copy is denied

r[molten.collaboration_scope.resource_binding.implicit-copy]
- GIVEN a caller can read a resource or belongs to its current scope but lacks exact move or copy authority
- WHEN it requests a target-scope binding
- THEN Molten MUST deny without creating another current binding.

### Requirement: Effective audience is an authority intersection

r[molten.collaboration_scope.effective_audience] An audience request MUST bind the exact scope revision, membership epoch, resource binding, selected subject set, requested ability, request identity, scope policy, resource policy, optional organization floor, and current decision facts. Admission MUST require every selected subject to pass the effective policy intersection.

#### Scenario: Every selected subject passes

r[molten.collaboration_scope.effective_audience.allowed]
- GIVEN every selected subject has current membership and matching scope, resource, organization-floor, and authority decisions
- WHEN effective-audience admission runs
- THEN Molten MAY emit an allowed snapshot for that exact audience and request.

#### Scenario: One selected subject fails

r[molten.collaboration_scope.effective_audience.partial-denial]
- GIVEN one selected subject is absent, removed, expired, denied, revoked, or outside the organization floor
- WHEN effective-audience admission runs
- THEN the whole proposed audience request MUST be denied
- AND Molten MUST NOT silently reduce or widen the audience.

#### Scenario: Narrow scope weakens the organization floor

r[molten.collaboration_scope.effective_audience.floor]
- GIVEN a narrower scope policy allows an action that the current organization floor denies
- WHEN effective policy is computed
- THEN the organization-floor denial MUST prevail.

### Requirement: Sharing authority is non-transitive

r[molten.collaboration_scope.nontransitive_sharing] Resource read, use, execution, prior delivery, shared membership, and scope ancestry MUST NOT imply `resource/share`, copy, move, delegation, or target-scope binding authority. Resharing MUST use a separate exact current request and decision.

#### Scenario: Exact share ability is allowed

r[molten.collaboration_scope.nontransitive_sharing.allowed]
- GIVEN a caller has a current exact `resource/share` decision for the resource, source scope, target scope, audience, and request
- WHEN share admission runs
- THEN Molten MAY produce the declared share transition plan.

#### Scenario: Reader attempts transitive share

r[molten.collaboration_scope.nontransitive_sharing.reader-denied]
- GIVEN a caller has read or use access but no current exact `resource/share` decision
- WHEN it requests resharing or a target binding
- THEN Molten MUST deny before scope mutation or product delivery.

### Requirement: Scope snapshots bind current authority facts

r[molten.collaboration_scope.authority_snapshot] A scope admission snapshot MUST bind the scope revision, membership epoch, resource binding, audience, requested ability, request, policy, consent when required, Basalt decision, UCAN verification, revocation observation, consistency epoch, logical currentness, outcome, and BLAKE3 state and receipt identities. The snapshot MUST remain evidence for one request and MUST NOT become a bearer capability or future authority.

#### Scenario: Current snapshot supports one consumer request

r[molten.collaboration_scope.authority_snapshot.current]
- GIVEN a consumer presents a snapshot whose complete binding matches current scope and authority facts
- WHEN the consumer validates the exact protected request before its effect
- THEN the snapshot MAY support that one consumer admission decision.

#### Scenario: Prior allowed snapshot is replayed

r[molten.collaboration_scope.authority_snapshot.replay]
- GIVEN an earlier snapshot was allowed before membership, policy, consent, revocation, or consistency state changed
- WHEN it is replayed for a later effect
- THEN currentness validation MUST deny it as fresh authority.

### Requirement: Scope projections are bounded and content free

r[molten.collaboration_scope.safe_projection] Operator and consumer projections MUST contain only bounded nominal references, epochs, counts, lifecycle, requested ability, outcome, safe diagnostic classes, and evidence identities. They MUST NOT contain raw tokens, keys, identity documents, policy bodies, member labels, messages, memories, prompts, files, credentials, resource bytes, or unrestricted diagnostics.

#### Scenario: Safe status is rendered

r[molten.collaboration_scope.safe_projection.safe]
- GIVEN a valid scope state and admission result
- WHEN operator or consumer projection runs
- THEN the projection MUST expose only the admitted metadata fields and explicit non-claims.

#### Scenario: Projection candidate contains payload data

r[molten.collaboration_scope.safe_projection.leak]
- GIVEN a projection candidate contains prohibited identity, policy, credential, content, token, or diagnostic material
- WHEN projection validation runs
- THEN validation MUST reject the candidate before persistence or publication.

### Requirement: Collaboration scope validation includes failures

r[molten.collaboration_scope.validation] Adoption MUST pair positive scope, membership, binding, move, audience, organization-floor, share, snapshot, restart, and recovery fixtures with negative domain, stale epoch, removed member, partial audience, wrong policy, weakened floor, transitive share, implicit copy, replay, revocation, expiry, partition, conflict, uncertain durability, stale consumer, and payload-leak fixtures.

#### Scenario: Complete fixture corpus passes

r[molten.collaboration_scope.validation.complete]
- GIVEN the reviewed positive and negative unit, property, simulation, and multiprocess corpus
- WHEN maintained collaboration-scope checks run
- THEN allowed cases MUST preserve the declared invariants
- AND denied cases MUST preserve prior state and produce no protected product effect.

#### Scenario: Evidence overclaims revocation or confidentiality

r[molten.collaboration_scope.validation.overclaim]
- GIVEN local tests and one passing scope receipt
- WHEN evidence claims global identity truth, revocation freshness, prior-disclosure erasure, transport confidentiality, external-effect safety, or release eligibility
- THEN claim validation MUST fail unless separate evidence establishes that claim.
