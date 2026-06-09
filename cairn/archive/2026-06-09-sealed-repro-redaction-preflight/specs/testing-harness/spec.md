# Testing Harness Delta: sealed repro redaction preflight

### Requirement: Sealed bundles include redaction evidence
r[molten.testing.sealed_repro_redaction_preflight.policy] Sealed report repro bundles MUST include canonical redaction policy evidence and redaction gate evidence before they can satisfy pass evidence gates.

#### Scenario: Normal sealed bundle includes redaction preflight
- GIVEN a deterministic report without sensitive markers
- WHEN `molten test repro export` exports a sealed bundle
- THEN the bundle contains `<redaction-policy-v1 ...>` and `<redaction-gate-v1 ...>` evidence
- AND gate/verify/unpack recompute the same redaction refs

### Requirement: Sensitive markers fail closed
r[molten.testing.sealed_repro_redaction_preflight.scan] Sealed report repro export MUST reject reports whose canonical Preserves values contain sensitive marker records.

#### Scenario: Secret marker blocks export
- GIVEN a deterministic report whose suite, observation, effect log, or report evidence contains `<secret ...>`
- WHEN sealed repro export runs
- THEN export fails closed with a redaction preflight diagnostic

#### Scenario: Confidential markers block export
- GIVEN a report containing `<confidential ...>`, `<credential ...>`, `<private ...>`, or `<encrypted-ref ...>`
- WHEN sealed repro export runs
- THEN export fails closed until explicit redaction/encryption validation exists

### Requirement: Tampered or missing redaction evidence fails gates
r[molten.testing.sealed_repro_redaction_preflight.validation] Bundle gate checks MUST reject missing, stale, or tampered redaction policy/gate evidence.

#### Scenario: Tampered redaction gate fails
- GIVEN a sealed report repro bundle
- WHEN its redaction gate evidence is changed after sealing
- THEN parsing, verification, unpacking, or gate checking rejects the bundle

#### Scenario: Unsealed report bundle no longer satisfies pass gate
- GIVEN a legacy report repro bundle without redaction preflight evidence
- WHEN `molten test gate check` runs on it
- THEN the gate fails closed because redaction preflight evidence is missing

### Requirement: Redaction gates are bound to report evidence
r[molten.testing.sealed_repro_redaction_preflight.gate] Redaction gate evidence MUST bind the embedded report ref, suite ref, policy ref, sensitive-marker scan result, and final allow-or-deny decision before a sealed bundle can satisfy pass gates.

#### Scenario: Redaction gate recomputes clean report refs
- GIVEN a clean deterministic report exported as a sealed bundle
- WHEN bundle verification recomputes redaction evidence
- THEN the recomputed redaction gate ref matches the bundle ref
- AND the gate decision remains pass for the same report and suite refs

### Requirement: Unsealed report bundles are rejected from pass gates
r[molten.testing.sealed_repro_redaction_preflight.unsealed_rejection] Pass-evidence gates MUST reject report repro bundles that do not include redaction preflight evidence.

#### Scenario: Legacy bundle fails pass evidence gate
- GIVEN a legacy report repro bundle without redaction policy and gate refs
- WHEN the bundle is used as pass evidence
- THEN the gate rejects it before trusting the embedded report or receipt

### Requirement: Redaction preflight has negative coverage
r[molten.testing.sealed_repro_redaction_preflight.tests] Redaction preflight tests SHOULD cover sensitive markers, missing evidence, stale evidence, and tampered redaction gate refs.

#### Scenario: Sensitive marker test denies export
- GIVEN a report fixture containing a sensitive marker record
- WHEN sealed repro export is exercised in tests
- THEN the export fails closed with a redaction diagnostic

### Requirement: Redaction preflight commands are documented
r[molten.testing.sealed_repro_redaction_preflight.docs] User-facing documentation SHOULD describe redaction policy evidence, redaction gate evidence, sensitive-marker failures, and legacy unsealed bundle rejection.

#### Scenario: Operator follows redaction docs
- GIVEN an operator reading repro redaction documentation
- WHEN they export a pass-evidence bundle
- THEN the documented evidence includes redaction policy and gate refs before verification or unpacking
