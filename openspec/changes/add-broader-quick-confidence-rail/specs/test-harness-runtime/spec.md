## ADDED Requirements

### Requirement: Broader Quick Confidence Rail [r[test-harness-runtime.quick-confidence-rail]]

Aspen MUST provide a broader quick confidence rail that composes selected local checks into one bounded operator command or check profile with a structured summary and explicit non-proof boundaries.

#### Scenario: Quick rail runs selected local checks [r[test-harness-runtime.quick-confidence-rail.selected-checks]]

- GIVEN a developer wants broader local confidence without running full dogfood or gated VM proofs
- WHEN the quick confidence rail runs
- THEN it SHALL execute a documented set of local checks such as quick Rust tests, harness metadata checks, relevant docs guardrails, and OpenSpec validation
- AND it SHALL report each included check with pass/fail status

#### Scenario: Quick rail reports skipped gated proofs [r[test-harness-runtime.quick-confidence-rail.skipped-gated-proofs]]

- GIVEN gated runtime-host proofs require KVM, Uhyve, Hyperlight, or other expensive environment support
- WHEN the quick confidence rail completes without running those proofs
- THEN its summary SHALL explicitly state that those gated proofs were skipped and SHALL NOT claim runtime-host acceptance from the quick rail alone

#### Scenario: Quick rail failure is actionable [r[test-harness-runtime.quick-confidence-rail.actionable-failure]]

- GIVEN one selected check fails
- WHEN the rail reports the result
- THEN it SHALL identify the failing check name, command or check profile, exit status, and next diagnostic pointer without hiding earlier successful checks
