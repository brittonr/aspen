## ADDED Requirements

### Requirement: Production-shaped multi-node soak workflow
r[molten.prod_soak.multi_node_live_workflow] Molten MUST provide a production-shaped multi-node soak workflow that exercises live peer tickets, node-control workflow bundles, remote dataspace or service exchange, job worker execution, coordination operations, and evidence export across at least two persistent node state roots.

#### Scenario: Multi-node soak binds child evidence
- GIVEN two or more nodes with persistent identities and admitted peer evidence
- WHEN the soak workflow completes
- THEN the soak receipt binds node startup refs, peer-ticket refs, node-control workflow refs, remote/service refs, job refs, coordination refs, and evidence-export refs for every participating node.

### Requirement: Network and transport fault matrix
r[molten.prod_soak.network_fault_matrix] Molten SHOULD test live or simulated network and transport faults including delay, drop, partition, rejoin, stale tickets, wrong authority grants, duplicate operations, conflicting operations, and corrupted or missing transport receipts.

#### Scenario: Stale ticket denies before side effects
- GIVEN a soak scenario with a stale or wrong live peer ticket
- WHEN a node-control request is sent or applied
- THEN Molten emits deny diagnostics before receiver-side control side effects are accepted.

### Requirement: Durability and restart soak
r[molten.prod_soak.durability_restart] Molten MUST include restart and durability scenarios covering queued control requests, active locks, ledger readback, chunk/artifact availability, retention state, and recovery receipts.

#### Scenario: Restart preserves queued request semantics
- GIVEN a node restarts while a control request is queued but not fully dispatched
- WHEN the soak harness resumes the node
- THEN the resulting receipts show deterministic idempotent handling of the queued request or a fail-closed recovery denial with diagnostics.

### Requirement: Soak replay and evidence boundary
r[molten.prod_soak.replay_and_evidence] Molten MUST emit canonical soak receipts that bind topology refs, fault-profile refs, child evidence refs, replay status, first-divergence diagnostics where applicable, and explicit non-replayable live caveats.

#### Scenario: Live-only observation is excluded from deterministic pass claim
- GIVEN a soak scenario includes an unrecorded live transport observation
- WHEN the soak receipt is evaluated for deterministic pass evidence
- THEN the observation is marked non-replayable and excluded or denied unless a recorded delivery log binds the event.

### Requirement: Performance and resource envelope
r[molten.prod_soak.performance_resource_envelope] Molten SHOULD track production-soak resource envelopes for queue depth, receipt growth, store growth, delivery latency, recovery time, resource pressure, and retained state growth, with explicit thresholds and diagnostics.

#### Scenario: Resource envelope breach is visible
- GIVEN a soak run exceeds a configured queue-depth or store-growth threshold
- WHEN the final soak receipt is emitted
- THEN it records degraded or deny status with the relevant resource measurements and child receipt refs.
