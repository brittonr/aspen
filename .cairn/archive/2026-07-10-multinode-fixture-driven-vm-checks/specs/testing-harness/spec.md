## ADDED Requirements

### Requirement: VM shard plans derive from typed multinode fixtures
r[molten.testing.multinode.fixture_driven_vm_plan] Molten SHOULD derive or validate NixOS VM shard plans from typed repository-owned multinode scenario fixtures that declare topology, execution profile, command surface, expected artifact kinds, required receipts, variance refs, unavailable policy, diagnostic logs, and evidence-only caveats before execution.

#### Scenario: Fixture drives VM shard metadata
- GIVEN a valid Nickel-authored multinode scenario fixture for a VM shard
- WHEN the fixture is exported or validated for VM execution
- THEN the resulting shard plan binds scenario id, topology profile, topology ref, command surface, expected artifact kinds, required receipts, variance refs, unavailable policy, diagnostics, and caveats
- AND the plan is derived without reading ambient runtime state.

#### Scenario: Fixture update changes expected evidence explicitly
- GIVEN a VM scenario changes its command surface or required artifact kinds
- WHEN the fixture is updated and validated
- THEN the expected VM evidence changes through the fixture metadata
- AND reviewers can see the scenario-scope change before execution.

### Requirement: VM evidence denies fixture mismatch
r[molten.testing.multinode.fixture_export_validation_gate] Molten MUST reject VM pass evidence when observed topology, execution profile, command surface, expected artifact kinds, child refs, unavailable policy, source language, or caveats do not match the checked scenario fixture export.

#### Scenario: Wrong fixture cannot satisfy VM run
- GIVEN VM receipts from one scenario and a fixture export for a different scenario
- WHEN the VM scenario gate evaluates the evidence
- THEN validation denies before pass evidence is accepted
- AND diagnostics identify the mismatched fixture field.

#### Scenario: Invalid fixture blocks VM pass evidence
- GIVEN a fixture missing required refs, variance declarations, unavailable policy, diagnostic log refs, or evidence-only caveats
- WHEN VM validation requests that fixture
- THEN validation denies even if the VM logs appear successful
- AND logs remain diagnostic-only.
