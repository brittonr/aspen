## ADDED Requirements

### Requirement: Production soak network diagnostics observability
r[molten.prod_soak.network_diagnostics_observability] Molten SHOULD bind network diagnostics, connectivity probes, watcher snapshots, metrics snapshots, relay latency, direct/relay path status, and resource-pressure refs into production-soak receipts.

#### Scenario: Soak reports relay latency degradation
- GIVEN a production-soak run observes relay latency above the configured diagnostic threshold
- WHEN the final soak receipt is emitted
- THEN it records degraded network diagnostics with relay-latency refs and resource-pressure refs
- AND it does not claim broad production transport correctness from the degraded run.

#### Scenario: Network diagnostics remain separate from side-effect gates
- GIVEN a soak run has passing network diagnostics
- WHEN node-control, job execution, retention, provenance, or coordination side effects are evaluated
- THEN those side effects still require their normal authority, policy, resource, provenance, source-gate, retention, and operation receipts independently of the diagnostics pass.
