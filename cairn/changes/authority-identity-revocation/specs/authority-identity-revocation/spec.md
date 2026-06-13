# authority identity revocation Delta Spec

## ADDED Requirements

### Requirement: Define canonical principal, node, actor, service, session, artifact, and execution identity records
r[molten.authority.identity_model] Define canonical principal, node, actor, service, session, artifact, and execution identity records.

### Requirement: Enforce that identity records alone grant no authority
r[molten.authority.no_identity_authority] Enforce that identity records alone grant no authority.

### Requirement: Define authority context records with capabilities, delegation chains, attenuation, expiry, revocation refs, key refs, policy refs, and evidence refs
r[molten.authority.context_model] Define authority context records with capabilities, delegation chains, attenuation, expiry, revocation refs, key refs, policy refs, and evidence refs.

### Requirement: Document that human-readable names are metadata, not security identity
r[molten.authority.names_metadata] Document that human-readable names are metadata, not security identity.

### Requirement: Require trust-boundary actions to carry and record an authority context before admission
r[molten.authority.admission_gate] Require trust-boundary actions to carry and record an authority context before admission.

### Requirement: Define revocation targets for keys, principals, delegations, capabilities, live refs, handler bindings, sessions, and artifacts
r[molten.authority.revocation_model] Define revocation targets for keys, principals, delegations, capabilities, live refs, handler bindings, sessions, and artifacts.

### Requirement: Retract assertions, subscriptions, live refs, and handler bindings when authority is lost
r[molten.authority.authority_cleanup] Retract assertions, subscriptions, live refs, and handler bindings when authority is lost.

### Requirement: Emit Cairn receipts for admission, denial, revocation, expiry, key rotation, and cleanup
r[molten.authority.receipts] Emit Cairn receipts for admission, denial, revocation, expiry, key rotation, and cleanup.

### Requirement: Gatekeeper resolution returns scoped, attenuated, expiring live refs with evidence refs
r[molten.authority.gatekeeper_resolution] Gatekeeper resolution returns scoped, attenuated, expiring live refs with evidence refs.

### Requirement: Check authority contexts for effect handler binding and effect requests
r[molten.authority.effect_integration] Check authority contexts for effect handler binding and effect requests.

### Requirement: Apply authority contexts to typed storage, remote sync/execution, and catalog visibility
r[molten.authority.storage_remote_catalog] Apply authority contexts to typed storage, remote sync/execution, and catalog visibility.

### Requirement: Ensure replay verifies recorded authority decisions without minting new current authority
r[molten.authority.replay_scope] Ensure replay verifies recorded authority decisions without minting new current authority.

### Requirement: Add tests that revocation retracts dependent assertions and denies future effect requests
r[molten.authority.revocation_tests] Add tests that revocation retracts dependent assertions and denies future effect requests.

### Requirement: Add tests for expiry using admitted logical clock sources
r[molten.authority.expiry_tests] Add tests for expiry using admitted logical clock sources.

### Requirement: Add tests for key rotation preserving historical verification without current authority
r[molten.authority.rotation_tests] Add tests for key rotation preserving historical verification without current authority.

### Requirement: Add Hegel property tests for attenuation monotonicity, no identity-as-authority, and cleanup invariants
r[molten.authority.property_tests] Add Hegel property tests for attenuation monotonicity, no identity-as-authority, and cleanup invariants.

