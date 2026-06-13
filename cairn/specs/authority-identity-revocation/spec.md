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
