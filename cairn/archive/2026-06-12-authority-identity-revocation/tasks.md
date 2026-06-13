## Phase 1: Identity and authority records

- [x] [serial] r[molten.authority.identity_model] Define canonical principal, node, actor, service, session, artifact, and execution identity records.
- [x] [serial] r[molten.authority.no_identity_authority] Enforce that identity records alone grant no authority.
- [x] [serial] r[molten.authority.context_model] Define authority context records with capabilities, delegation chains, attenuation, expiry, revocation refs, key refs, policy refs, and evidence refs.
- [x] [parallel] r[molten.authority.names_metadata] Document that human-readable names are metadata, not security identity.

## Phase 2: Admission and revocation

- [x] [serial] r[molten.authority.admission_gate] Require trust-boundary actions to carry and record an authority context before admission.
- [x] [serial] r[molten.authority.revocation_model] Define revocation targets for keys, principals, delegations, capabilities, live refs, handler bindings, sessions, and artifacts.
- [x] [serial] r[molten.authority.authority_cleanup] Retract assertions, subscriptions, live refs, and handler bindings when authority is lost.
- [x] [parallel] r[molten.authority.receipts] Emit Cairn receipts for admission, denial, revocation, expiry, key rotation, and cleanup.

## Phase 3: Integration

- [x] [serial] r[molten.authority.gatekeeper_resolution] Gatekeeper resolution returns scoped, attenuated, expiring live refs with evidence refs.
- [x] [serial] r[molten.authority.effect_integration] Check authority contexts for effect handler binding and effect requests.
- [x] [parallel] r[molten.authority.storage_remote_catalog] Apply authority contexts to typed storage, remote sync/execution, and catalog visibility.
- [x] [parallel] r[molten.authority.replay_scope] Ensure replay verifies recorded authority decisions without minting new current authority.

## Phase 4: Tests

- [x] [serial] r[molten.authority.revocation_tests] Add tests that revocation retracts dependent assertions and denies future effect requests.
- [x] [serial] r[molten.authority.expiry_tests] Add tests for expiry using admitted logical clock sources.
- [x] [parallel] r[molten.authority.rotation_tests] Add tests for key rotation preserving historical verification without current authority.
- [x] [parallel] r[molten.authority.property_tests] Add Hegel property tests for attenuation monotonicity, no identity-as-authority, and cleanup invariants.
