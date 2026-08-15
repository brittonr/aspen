## ADDED Requirements

### Requirement: Remaining Octet safety-polish warnings are tested to closure
r[molten.octet_burndown.safety_polish_finish] Molten MUST treat remaining no-disabled safety-polish warnings as active remediation work until each finding is removed or explicitly scoped by deterministic evidence, and MUST cover logic-affecting remediation with positive and negative tests.

#### Scenario: Logic-affecting safety remediation is tested
- GIVEN a remediation changes result handling, collection bounds, numeric conversion, arithmetic, boolean naming, or control-flow shape
- WHEN the remediation is counted as burn-down progress
- THEN focused positive and negative tests cover the changed behavior
- AND no-disabled Octet evidence records before/after movement for the touched warning family.

#### Scenario: Deferred safety warning remains visible
- GIVEN a safety-polish finding cannot be safely removed in the current slice
- WHEN remediation evidence is reported
- THEN Molten records the finding as an active caveat with rationale instead of hiding it behind configuration-clean source-gate evidence.
