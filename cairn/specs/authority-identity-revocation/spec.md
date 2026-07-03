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
