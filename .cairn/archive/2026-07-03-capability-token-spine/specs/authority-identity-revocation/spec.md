## ADDED Requirements

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
