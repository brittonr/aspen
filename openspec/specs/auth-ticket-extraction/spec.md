# auth-ticket-extraction Specification

## Purpose
Define the in-workspace reusable boundary for Aspen's auth, capability, cluster ticket, and hook ticket crates. This spec keeps portable token/ticket APIs separate from runtime verifier and storage shells and records the evidence required before readiness labels can be raised.
## Requirements
### Requirement: Portable auth ticket API is owned
The auth ticket review MUST define canonical portable APIs for token, capability, cluster-ticket, and hook-ticket consumers before workspace readiness labels change.
ID: auth-ticket-extraction.portable-api-owned

#### Scenario: Portable auth ticket API is owned evidence
ID: auth-ticket-extraction.portable-api-owned.evidence
- GIVEN `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket` are reviewed
- WHEN the readiness decision is recorded
- THEN it SHALL identify stable types, canonical imports, compatibility re-exports, and owner expectations.

### Requirement: Runtime verifier leakage is rejected
The auth ticket review MUST prove portable consumers do not depend on runtime verifier, HMAC, revocation storage, filesystem, node, or handler crates by default.
ID: auth-ticket-extraction.runtime-leakage-rejected

#### Scenario: Runtime verifier leakage is rejected evidence
ID: auth-ticket-extraction.runtime-leakage-rejected.evidence
- GIVEN a downstream-style portable fixture and a negative runtime fixture
- WHEN dependency checks run
- THEN the portable graph SHALL exclude `aspen-auth` runtime verifier shells and runtime storage unless an explicit adapter feature is enabled.

### Requirement: Auth ticket workspace readiness is evidenced
The auth ticket review MUST promote the family only when docs, inventory, policy, fixtures, and readiness-checker outputs agree on the same readiness state.
ID: auth-ticket-extraction.workspace-readiness-evidenced

#### Scenario: Auth ticket workspace readiness is evidenced evidence
ID: auth-ticket-extraction.workspace-readiness-evidenced.evidence
- GIVEN the family remains blocked from publication or repo split by license/publication policy
- WHEN the in-workspace public API review is complete
- THEN the family SHALL be marked `extraction-ready-in-workspace` with verification artifacts and SHALL NOT claim publishable/repo-split readiness.
