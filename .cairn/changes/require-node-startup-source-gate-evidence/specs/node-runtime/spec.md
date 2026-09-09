## ADDED Requirements

### Requirement: Normal startup consumes real gate evidence
r[molten.audit_f02.real_evidence] Normal node startup MUST consume independently validated source-gate artifacts and MUST NOT substitute synthetic passing evidence.

#### Scenario: Exact gate artifacts admit startup
- GIVEN complete real gate artifacts for the selected candidate and policy
- WHEN normal startup validates them
- THEN source-gate admission passes with their exact references

#### Scenario: Missing artifacts deny startup
- GIVEN no independently validated source-gate artifacts
- WHEN normal startup evaluates admission
- THEN startup denies without a synthetic fallback

### Requirement: Gate evidence binds the candidate
r[molten.audit_f02.candidate_binding] Startup admission MUST validate candidate identity, policy, profile, toolchain, and required gate artifact linkage before activation.

#### Scenario: Matching candidate evidence remains bound
- GIVEN valid gate artifacts with matching candidate and policy bindings
- WHEN startup records acceptance
- THEN its receipt preserves those bindings

#### Scenario: Stale or tampered evidence denies
- GIVEN stale, malformed, tampered, or wrong-candidate gate artifacts
- WHEN startup evaluates the artifacts
- THEN startup denies and preserves prior lifecycle state

### Requirement: Fixture composition stays test-only
r[molten.audit_f02.fixture_isolation] Synthetic source-gate fixtures MUST remain limited to explicit test-only composition and unavailable as normal startup acceptance evidence.

#### Scenario: Explicit fixture tests remain possible
- GIVEN an explicit test-only composition for pure gate decisions
- WHEN the test supplies a synthetic gate value
- THEN the test can exercise that decision without claiming real Octet execution

#### Scenario: Compatibility wrapper cannot bypass validation
- GIVEN a normal startup compatibility wrapper and a synthetic gate value
- WHEN the wrapper reaches startup admission
- THEN admission rejects the value rather than treating fixture metadata as execution evidence

### Requirement: Startup evidence checks have bounded claims
r[molten.audit_f02.validation] F02 validation MUST cover normal-path artifact adapters, rejected-state preservation, and fixture isolation without presenting static analysis as executed startup evidence.

#### Scenario: Normal-path regressions record their scope
- GIVEN normal repository tests for real artifacts and rejected inputs
- WHEN those tests execute
- THEN the evidence identifies the candidate, tested adapter boundary, and results

#### Scenario: Fixture pass cannot replace gates
- GIVEN only a passing synthetic fixture test or the original static finding
- WHEN startup or completion needs real gate evidence
- THEN that evidence remains insufficient
