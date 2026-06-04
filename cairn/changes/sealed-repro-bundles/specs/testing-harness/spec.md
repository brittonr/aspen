# Testing Harness Delta: sealed repro bundles

### Requirement: Report repro bundles are sealed pass artifacts
r[molten.testing.sealed_repro_bundles.schema] Exported report repro bundles MUST include seal metadata, an embedded report gate receipt, and artifact refs for the report evidence required to validate and replay the run.

#### Scenario: Exported report bundle contains a seal
- GIVEN a deterministic report that validates and gates successfully
- WHEN `molten test repro export` exports the report
- THEN `refs.preserves` contains `<repro-seal ...>`, an embedded `<gate-receipt-v1 ...>`, and refs for the report, suite, actor registry, effect log, policy gate, capability gate, budget gate, and gate receipt

### Requirement: Sealed bundles validate embedded refs fail-closed
r[molten.testing.sealed_repro_bundles.validation] Parsing or gate checking a sealed bundle MUST recompute embedded report refs and receipt refs instead of trusting the bundle fields.

#### Scenario: Tampered embedded report fails
- GIVEN a sealed report repro bundle
- WHEN the embedded report is changed after sealing
- THEN gate checking the bundle fails closed before accepting it as pass evidence

#### Scenario: Tampered embedded receipt fails
- GIVEN a sealed report repro bundle
- WHEN the embedded gate receipt is changed after sealing
- THEN gate checking the bundle fails closed with a seal or receipt diagnostic

#### Scenario: Mismatched suite ref fails
- GIVEN a sealed report repro bundle
- WHEN the bundle suite ref or suite artifact ref no longer matches the embedded report
- THEN gate checking the bundle fails closed

### Requirement: Bundle gates recompute the report gate receipt
r[molten.testing.sealed_repro_bundles.gate] A sealed bundle MUST satisfy pass evidence only if its embedded report gate receipt exactly matches the receipt recomputed from the embedded report.

#### Scenario: Sealed bundle gates as repro-bundle artifact
- GIVEN a valid sealed report repro bundle
- WHEN `molten test gate check refs.preserves` runs
- THEN the embedded report receipt is validated
- AND the command emits a new gate receipt whose artifact kind is `repro-bundle`

### Requirement: Failure repro bundles remain diagnostics only
r[molten.testing.sealed_repro_bundles.failure_diagnostics] Failure repro bundles MUST NOT satisfy pass evidence gates.

#### Scenario: Failure bundle rejected by pass gate
- GIVEN a failure repro bundle
- WHEN `molten test gate check refs.preserves` runs
- THEN the gate rejects it as diagnostic evidence only
