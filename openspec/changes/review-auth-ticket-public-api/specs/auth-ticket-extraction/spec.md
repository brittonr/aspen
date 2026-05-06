# Review auth and ticket public API Delta

## ADDED Requirements

### Requirement: Portable auth ticket API is owned [r[auth-ticket-extraction.portable-api-owned]]
The auth ticket review MUST define canonical portable APIs for token, capability, and hook-ticket consumers before publication/readiness labels change.

#### Scenario: Portable auth ticket API is owned evidence [r[auth-ticket-extraction.portable-api-owned.evidence]]
- GIVEN `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket` are reviewed
- WHEN the readiness decision is recorded
- THEN it SHALL identify stable types, canonical imports, compatibility re-exports, and owner expectations.

### Requirement: Runtime verifier leakage is rejected [r[auth-ticket-extraction.runtime-leakage-rejected]]
The auth ticket review MUST prove portable consumers do not depend on runtime verifier, HMAC, revocation storage, filesystem, node, or handler crates by default.

#### Scenario: Runtime verifier leakage is rejected evidence [r[auth-ticket-extraction.runtime-leakage-rejected.evidence]]
- GIVEN a downstream-style portable fixture and a negative runtime fixture
- WHEN dependency checks run
- THEN the portable graph SHALL exclude `aspen-auth` runtime verifier shells and runtime storage unless an explicit adapter feature is enabled.
