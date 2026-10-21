# Content replication: F05 delta

## ADDED Requirements

### Requirement: Cleanup preserves the residual policy
r[molten.audit_f05.residual_policy] Molten MUST preserve desired replicas, minimum verified replicas, and minimum fault domains in the cumulative remainder after each cleanup selection.

#### Scenario: Redundant domain permits cleanup
- GIVEN A and B occupy zone A and C occupies zone B under a two-replica, two-domain policy
- WHEN all replicas have valid cleanup clearance and no pins
- THEN cleanup selects at most one of A and B and retains C.

#### Scenario: Unique domain blocks removal
- GIVEN a candidate is the only replica in a required domain
- WHEN its removal violates the minimum domain count
- THEN the planner does not select that candidate.

### Requirement: Cleanup selection is deterministic and conservative
r[molten.audit_f05.selection] Molten MUST evaluate safe alternatives deterministically and MUST retain replicas when no policy-preserving candidate exists.

#### Scenario: Input order does not change selection
- GIVEN equivalent inventories in different input orders
- WHEN the planner selects cleanup actions
- THEN the selected actions and plan reference match.

#### Scenario: Combined removals are unsafe
- GIVEN individual candidates appear safe against the original inventory but their combined removal violates policy
- WHEN the planner evaluates later candidates
- THEN it uses the cumulative remainder and blocks the unsafe removal.

### Requirement: Placement safety does not grant cleanup authority
r[molten.audit_f05.authority_preservation] Molten MUST retain separate cleanup admission and MUST preserve content and retention state after rejection.

#### Scenario: Admitted cleanup executes
- GIVEN a policy-safe action with current authority, valid clearance, and no active pin
- WHEN the shell executes cleanup
- THEN retention admission precedes the content cleanup call.

#### Scenario: Rejected cleanup has no deletion effect
- GIVEN missing authority, an active pin, stale clearance, or adapter denial
- WHEN cleanup admission rejects
- THEN no content deletion occurs and no successful deletion observation is emitted.

### Requirement: Cleanup evidence distinguishes plans from effects
r[molten.audit_f05.validation_claims] Molten MUST retain positive and negative repository tests and MUST distinguish safe-plan evidence from executed deletion evidence.

#### Scenario: Normal tests preserve domains
- GIVEN the F05 three-peer fixture
- WHEN normal planner and controlled-adapter tests run
- THEN they cover safe selection and rejected execution without ignored scratch files.

#### Scenario: Unsafe-plan evidence is not deletion evidence
- GIVEN the executed F05 pure-planner counterexample
- WHEN an audit or release summary describes its result
- THEN the summary identifies an unsafe plan and does not claim executed deletion or production data loss.
