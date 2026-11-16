# Upgrade Sessions Delta

## ADDED Requirements

### Requirement: Generated upgrade plans bind authenticated source-gate evidence
r[molten.upgrades.authenticated_source_gate] Producers that generate upgrade plans from other workflows MUST bind source-gate receipt values obtained from authenticated source-binding admission of the executing candidate and MUST NOT fabricate or accept caller-supplied gate receipts. Upgrade-plan validation MUST still deny denied, stale or non-Octet gate content.

#### Scenario: Authenticated gate admits the plan
- GIVEN a source gate admitted by the authenticated source binding
- WHEN an upgrade plan is generated from it
- THEN the plan binds the gate receipt, the binding ref and the binding-validation ref, and its source-gate validation passes

#### Scenario: Denied or fixture gate content is rejected
- GIVEN gate content that is denied or is an unqualified fixture
- WHEN an upgrade plan is generated from it
- THEN plan validation fails and no plan is produced
