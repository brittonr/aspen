# Testing Harness Delta: Boundary Coverage Gate

## ADDED Requirements

### Requirement: Boundary coverage is gateable
r[molten.testing.boundary_coverage.gate] Molten SHOULD provide a boundary coverage gate that evaluates harness reports or traceability receipts for exercised runtime boundary classes and emits canonical pass, deny, or exempt diagnostics.

#### Scenario: Unexercised policy denial is reported
- GIVEN a suite that exercises policy pass paths but no policy denial path
- WHEN the boundary coverage gate evaluates the report for a requirement that needs denial coverage
- THEN the gate denies or reports a missing policy-denial boundary diagnostic

### Requirement: Positive and negative boundary classes are tracked
r[molten.testing.boundary_coverage.positive_negative] Evidence-bearing requirements SHOULD declare or derive both positive and negative boundary coverage expectations unless an explicit exemption applies.

#### Scenario: Capability coverage includes grant and deny
- GIVEN a requirement covering capability admission
- WHEN boundary coverage is summarized
- THEN the summary identifies both admitted capability behavior and denied capability behavior or an explicit exemption

### Requirement: Boundary coverage exemptions are explicit
r[molten.testing.boundary_coverage.exemptions] Boundary coverage exemptions MUST carry reason class, evidence path or receipt ref, scope, and diagnostic-only caveats, and MUST NOT silently satisfy behavioral pass evidence.

#### Scenario: VM-unavailable exemption remains visible
- GIVEN a boundary class that requires VM support unavailable on the current host
- WHEN the boundary coverage gate evaluates the profile
- THEN it records an unavailable or exempt diagnostic without converting the missing VM boundary into pass evidence
