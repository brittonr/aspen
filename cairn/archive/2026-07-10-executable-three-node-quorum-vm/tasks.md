# Tasks: executable-three-node-quorum-vm

## Phase 1: VM shard plan

- [x] [serial] r[molten.testing.three_node_quorum_vm.executable_shard] Define the executable three-node VM shard plan from the `vm-three-node-quorum` scenario fixture.
- [x] [parallel] r[molten.testing.three_node_quorum_vm.negatives] Extend pure validation diagnostics for wrong topology, missing quorum refs, subscriber-as-voter, transport-only authority, duplicate commit, and log-only quorum claims.

## Phase 2: Platform wiring

- [x] [serial] r[molten.testing.three_node_quorum_vm.executable_shard] Wire the VM/Nix check or shard command to collect majority, minority-denial, restart/rejoin, and duplicate-suppression receipts.
- [x] [parallel] r[molten.testing.three_node_quorum_vm.negatives] Add negative fixtures and failure-bundle export paths for quorum shard denials or unavailable host support.

## Phase 3: Aggregate and validation

- [x] [serial] r[molten.testing.three_node_quorum_vm.executable_shard] Bind the shard into VM scenario, reconciliation, and aggregate outputs with bounded-topology caveats.
- [x] [serial] r[molten.testing.three_node_quorum_vm.negatives] Run focused three-node pure tests, VM shard metadata tests, and traceability coverage updates.
