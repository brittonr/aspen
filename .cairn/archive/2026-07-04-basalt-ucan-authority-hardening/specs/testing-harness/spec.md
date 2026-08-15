# Testing Harness Delta: Basalt/UCAN authority hardening

### Requirement: Non-empty harness UCAN proofsets require verification receipts
r[molten.testing.capability_context.ucan_proofset_validation] Harness reports MAY include non-empty UCAN proofsets only when matching UCAN verification receipts are embedded or referenced, and validation MUST fail closed unless each proofset ref, token ref, proof ref, derived grant ref, caveat decision, revocation fact, replay fact, and request ref matches the embedded suite and observation evidence.

#### Scenario: Verified proofset is accepted
- GIVEN a harness suite whose capability gate contains a non-empty UCAN proofset and matching verification receipts
- WHEN report validation recomputes the capability gate
- THEN validation accepts the proofset only if every receipt and derived grant ref binds to the suite capability context and observed request.

#### Scenario: Stale proofset is rejected
- GIVEN a report whose UCAN verification receipt was produced for a different request, suite, holder, session, resource, ability, or proofset ref
- WHEN report validation runs
- THEN validation rejects the report before pass evidence can be emitted.

### Requirement: Harness admission binds Basalt enforcement receipts
r[molten.testing.capability_context.basalt_enforcement_receipts] Harness admission for Basalt-governed actions MUST call Basalt enforcement over the selected contract, requested resource, requested ability, and verified UCAN-derived grants, and MUST bind the Basalt enforcement receipt into observation, capability gate, and pass gate evidence.

#### Scenario: Basalt enforcement receipt admits action
- GIVEN UCAN verification derives a grant for the requested resource and ability
- AND the selected Basalt policy contract permits that resource and ability
- WHEN the harness admits the action
- THEN the observation carries an authority receipt that binds the UCAN verification receipt, derived grant refs, Basalt policy refs, Basalt enforcement receipt, and request ref.

#### Scenario: Mismatched Basalt policy denies action
- GIVEN UCAN verification derives a matching grant
- BUT the Basalt contract policy does not permit the requested resource or ability
- WHEN the harness attempts to admit the action
- THEN admission denies and the report contains deny evidence instead of committed side-effect evidence.

### Requirement: UCAN/Basalt fixtures cover positive and negative authority cases
r[molten.testing.capability_context.ucan_negative_fixtures] Harness tests SHOULD include positive UCAN/Basalt admission fixtures and negative fixtures for invalid signatures, unknown keys, wrong holder, wrong audience, wrong session, wrong context, wrong resource, wrong ability, expired or not-yet-valid tokens, revoked issuers or proofs, missing caveat evidence, replay denial, mismatched Basalt policy, local fixture fallback attempts, and tampered receipt refs.

#### Scenario: Negative fixture fails closed before pass gate
- GIVEN a UCAN/Basalt negative fixture with one invalid authority binding
- WHEN the harness runs, replays, validates, or gates the report
- THEN the fixture fails closed before emitting pass evidence
- AND diagnostics identify the invalid binding class.
