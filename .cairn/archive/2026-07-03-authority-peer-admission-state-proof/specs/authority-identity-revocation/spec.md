## ADDED Requirements

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
