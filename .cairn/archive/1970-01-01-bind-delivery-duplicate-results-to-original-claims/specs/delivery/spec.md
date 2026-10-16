# Delivery: F08 delta

## ADDED Requirements

### Requirement: Duplicate tokens bind the original operation
r[molten.audit_f08.original_binding] Molten MUST return a duplicate token only when its token reference matches the original applied operation.

#### Scenario: Immediate replay returns the original token
- GIVEN A holds the current token from its original claim
- WHEN A repeats the exact claim request
- THEN duplicate replay returns only that original token and operation reference.

#### Scenario: Reclaim cannot substitute another token
- GIVEN A's lease expired and B holds a new token for the same item
- WHEN A repeats its original claim
- THEN the response never returns B's token as A's prior result.

### Requirement: Missing historical tokens remain explicit
r[molten.audit_f08.unavailable_result] Molten MUST retain the original operation reference and return no token when the original token is unavailable.

#### Scenario: Available original result retains identity
- GIVEN the original token remains available with its exact saved reference
- WHEN duplicate reconstruction runs
- THEN it preserves the original token identity without a new claim.

#### Scenario: Original token no longer exists
- GIVEN only a later token or completed item remains
- WHEN duplicate reconstruction cannot recover the original token
- THEN the optional token is absent and the response does not imply a renewed lease.

### Requirement: Duplicate replay preserves independent admission
r[molten.audit_f08.state_authority] Molten MUST preserve state on duplicate or denied requests and MUST retain separate current-token, owner, and completion-authority checks.

#### Scenario: Valid delegated completion remains valid
- GIVEN a current token and the exact admitted delegated completion authority
- WHEN completion evaluates a different consumer
- THEN the existing completion rules determine admission independently from duplicate evidence.

#### Scenario: Wrong owner cannot use duplicate evidence
- GIVEN a stale token or a different owner without admitted delegated authority
- WHEN completion evaluates the request
- THEN it rejects without queue mutation or protected effects.

#### Scenario: Duplicate has no new effects
- GIVEN an exact previously applied claim request
- WHEN the service returns duplicate replay
- THEN durable revision remains unchanged and no timer, worker dispatch, or new claim occurs.

### Requirement: Validation distinguishes response faults from authority faults
r[molten.audit_f08.validation_claims] Molten MUST retain positive and negative repository tests and MUST limit F08 evidence to demonstrated response behavior.

#### Scenario: Regression survives durable reopen
- GIVEN the expiry and reclaim sequence from F08
- WHEN normal core and controlled-adapter tests repeat the original claim after reopen
- THEN its response binds the original operation and never substitutes the later token.

#### Scenario: Wrong response does not prove bypass
- GIVEN the original F08 counterexample without a successful unauthorized completion
- WHEN a report describes the finding
- THEN it states wrong-response evidence and does not claim an authority bypass or exactly-once effects.
