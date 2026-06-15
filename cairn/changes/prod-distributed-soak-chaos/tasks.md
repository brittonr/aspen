## Phase 1: Soak harness

- [ ] [serial] r[molten.prod_soak.multi_node_live_workflow] Define and implement a production-shaped multi-node live workflow that covers peer tickets, node-control bundle lifecycle, remote dataspace/service exchange, job worker execution, coordination operation, and evidence export.
- [ ] [serial] r[molten.prod_soak.replay_and_evidence] Emit canonical soak run receipts with topology, fault profile, node evidence, replay status, diagnostics, and non-replayable caveats.

## Phase 2: Fault matrix

- [ ] [parallel] r[molten.prod_soak.network_fault_matrix] Add network and transport fault scenarios for delay, drop, partition, rejoin, stale ticket, wrong authority, duplicate operation, and conflicting operation ids.
- [ ] [parallel] r[molten.prod_soak.durability_restart] Add restart and durability scenarios covering queued control requests, ledger/readback, chunk/artifact availability, retention state, and recovery receipts.

## Phase 3: Resource envelope

- [ ] [parallel] r[molten.prod_soak.performance_resource_envelope] Track queue depth, receipt growth, store growth, delivery latency, recovery time, and resource pressure with explicit bounds and denial behavior.
- [ ] [serial] r[molten.prod_soak.multi_node_live_workflow] Document which soak evidence is sufficient for internal pilot and which broad production claims remain out of scope.
