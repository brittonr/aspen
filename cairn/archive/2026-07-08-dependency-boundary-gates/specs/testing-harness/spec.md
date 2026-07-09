# Testing Harness Delta: Dependency Boundary Gates

### Requirement: Modularity rules are reviewed policy
r[molten.modularity.boundary_gates.policy] Repository dependency-boundary rules SHOULD be declared in reviewed source-controlled policy that names each rule, owning layer, allowed or denied dependency patterns, diagnostic guidance, and exemption class.

#### Scenario: Valid boundary policy loads
- GIVEN a reviewed dependency-boundary policy with unique rule ids and valid path patterns
- WHEN the boundary validator loads the policy
- THEN validation succeeds and preserves the reviewed rules deterministically

#### Scenario: Malformed boundary policy fails
- GIVEN duplicate rule ids, unknown layers, invalid path patterns, or contradictory allow/deny entries
- WHEN the boundary validator loads the policy
- THEN validation fails before generated policy or release evidence is refreshed

### Requirement: Boundary validator reports actionable diagnostics
r[molten.modularity.boundary_gates.validator] The dependency-boundary validator MUST report deterministic diagnostics that identify the rule id, source file, forbidden target or pattern, and remediation or exemption guidance for each violation.

#### Scenario: Forbidden dependency is reported
- GIVEN a source file imports a dependency forbidden by its layer rule
- WHEN the boundary validator scans the repository
- THEN it reports the violating file, the forbidden target, the rule id, and the expected remediation or exemption class

#### Scenario: Clean source passes
- GIVEN source files whose imports satisfy the reviewed boundary policy
- WHEN the boundary validator scans the repository
- THEN it reports a pass decision with no violation diagnostics

### Requirement: Boundary gate has positive and negative fixtures
r[molten.modularity.boundary_gates.fixtures] Boundary-gate validation SHOULD include positive fixtures for allowed imports and negative fixtures for representative forbidden dependencies and malformed policy inputs.

#### Scenario: Positive fixture passes
- GIVEN a fixture representing allowed core, codec, runtime, adapter, and CLI import relationships
- WHEN boundary validation runs on the fixture
- THEN the fixture passes without diagnostics

#### Scenario: Negative fixture fails for expected rule
- GIVEN a fixture representing core-to-adapter, runtime-to-CLI, codec-to-domain, or unclassified-public-export violation
- WHEN boundary validation runs on the fixture
- THEN it fails with the expected rule id and does not pass because of unrelated parser or policy errors

### Requirement: Boundary gate is runnable as focused validation
r[molten.modularity.boundary_gates.integration] The dependency-boundary gate SHOULD be runnable as a focused validation command or documented check and MAY later be wired into Nix, Octet, Cairn release-readiness, or CI evidence rails.

#### Scenario: Developer runs focused boundary check
- GIVEN a checkout with boundary policy and validator fixtures
- WHEN a developer runs the documented focused boundary command
- THEN the command checks the configured source scope and emits pass or violation diagnostics suitable for release review
