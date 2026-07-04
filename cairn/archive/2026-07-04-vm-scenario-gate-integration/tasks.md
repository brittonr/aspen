# Tasks: vm-scenario-gate-integration

## Phase 1: Gate core wiring

- [x] [parallel] r[molten.testing.multinode.vm_scenario_metadata_gate] Add a pure builder that binds VM run inputs to validated multinode scenario metadata and reports scenario/command/artifact mismatches.
- [x] [parallel] r[molten.testing.multinode.vm_reconciliation_gate] Add a pure VM reconciliation input builder from node evidence, child workflow receipts, equality classes, and declared variance refs.

## Phase 2: VM shell integration

- [x] [serial] r[molten.testing.multinode.vm_scenario_metadata_gate] Extend VM shard or aggregate scripts to provide the checked scenario fixture metadata and write scenario gate receipts into `vm-evidence`.
- [x] [serial] r[molten.testing.multinode.vm_reconciliation_gate] Extend VM evidence generation to emit topology membership, reconciliation, and live transport gate receipts where the scenario requires them.

## Phase 3: Positive and negative coverage

- [x] [parallel] r[molten.testing.multinode.vm_scenario_metadata_gate] Add positive tests for matching fixture metadata and negative tests for wrong fixture, stale receipt ref, unsupported pass claim, and mismatched artifact kind.
- [x] [parallel] r[molten.testing.multinode.vm_reconciliation_gate] Add negative tests for divergent queue or ledger refs, missing receive receipt, stale protocol gate, duplicate semantic commit, undeclared variance, and log-only reconciliation.
- [x] [serial] r[molten.testing.multinode.vm_reconciliation_gate] Run focused multinode gate tests and the smallest VM shard that emits gate receipts, or record host-support blockers and next best checks.
