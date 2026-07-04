# Authority Identity Revocation Delta: Basalt/UCAN authority hardening

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
