# Testing Harness Delta: Hegel Counterexample Fixtures

## ADDED Requirements

### Requirement: Hegel counterexamples become replay fixtures
r[molten.testing.hegel_counterexample.replay_fixture] Hegel property failures SHOULD emit canonical counterexample fixtures that bind the property id, generator profile ref, generation seed, shrink path, final shrunk Preserves input, replay identity, trace refs, receipt refs, and diagnostics.

#### Scenario: Shrunk counterexample replays without generator
- GIVEN a Hegel property failure with a final shrunk input
- WHEN the harness writes a counterexample fixture
- THEN another run can replay the fixture from canonical data without invoking the property generator

### Requirement: Counterexample promotion is reviewed
r[molten.testing.hegel_counterexample.promotion] Promotion from a counterexample fixture to a deterministic regression case or known-deny fixture MUST record review metadata, source property refs, old and new fixture refs, reason class, and resulting status.

#### Scenario: Counterexample becomes regression
- GIVEN a reviewed Hegel counterexample fixture for a fixed bug
- WHEN it is promoted to a deterministic regression suite
- THEN the promotion record binds the source fixture ref, new suite entry ref, review reason, and post-fix pass evidence

### Requirement: Counterexample exports preserve confidentiality
r[molten.testing.hegel_counterexample.redaction] Counterexample fixture export MUST redact or encrypt sensitive generated inputs and capability-bearing traces before materializing shareable repro evidence.

#### Scenario: Sensitive generated input is redacted
- GIVEN a property failure whose shrunk Preserves input contains a secret marker
- WHEN the counterexample fixture is exported without reveal authority
- THEN the exported fixture uses redaction markers or encrypted refs and records transform evidence
