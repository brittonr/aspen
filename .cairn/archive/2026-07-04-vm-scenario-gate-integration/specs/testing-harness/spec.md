## ADDED Requirements

### Requirement: VM scenarios bind declarative fixture metadata
r[molten.testing.multinode.vm_scenario_metadata_gate] Molten SHOULD bind each VM multinode shard or aggregate run to validated multinode scenario fixture metadata before accepting the run as VM pass evidence.

#### Scenario: VM run consumes checked scenario metadata
- GIVEN a VM run and a checked multinode scenario fixture export with topology, profile, command surface, expected artifact kinds, variance refs, unavailable policy, and caveats
- WHEN the VM evidence is validated
- THEN the VM run binds the scenario metadata ref and fixture ref
- AND diagnostics identify any mismatch between the scenario declaration and the observed VM evidence.

#### Scenario: Wrong fixture cannot satisfy VM pass evidence
- GIVEN VM receipts from one scenario and metadata from a different scenario fixture
- WHEN the VM scenario gate evaluates them
- THEN the gate denies before pass evidence is accepted
- AND diagnostic logs cannot repair the mismatch.

### Requirement: VM evidence passes multinode reconciliation gates
r[molten.testing.multinode.vm_reconciliation_gate] Molten MUST run multinode topology membership, reconciliation, and live transport gates over VM evidence before a VM run claims cross-node topology, reconciliation, or live transport success.

#### Scenario: Reconciled VM nodes produce gate evidence
- GIVEN VM node summaries with matching topology refs, scenario fixture refs, required receipt refs, queue refs, ledger refs, dispatch refs, ack refs, protocol refs, and declared variance refs
- WHEN the VM reconciliation gate evaluates the summaries
- THEN it emits a passing reconciliation receipt bound into the VM shard or aggregate manifest.

#### Scenario: Divergent VM evidence denies without declared variance
- GIVEN VM node summaries with divergent queue, ledger, dispatch, ack, protocol, or semantic commit refs and no matching variance declaration
- WHEN the VM reconciliation gate evaluates the summaries
- THEN it denies before pass evidence is accepted
- AND the diagnostic names the divergent equality class.
