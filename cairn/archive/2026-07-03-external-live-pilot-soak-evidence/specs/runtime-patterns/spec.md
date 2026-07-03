## ADDED Requirements

### Requirement: External pilot network and resource bounds are canonical
r[molten.external_live_pilot_soak.network_resource_bounds] External pilot soak receipts SHOULD bind canonical network diagnostics, connectivity observations, resource envelope checks, replayability caveats, and degradation thresholds before a pilot decision can pass.

#### Scenario: Resource envelope breach denies pilot
- GIVEN a multi-host pilot run whose queue depth, delivery latency, memory, storage, or network resource evidence exceeds the configured pilot threshold
- WHEN the pilot decision validates the run
- THEN the decision denies or records the run as degraded outside pass scope
- AND logs alone cannot override the canonical resource evidence.

### Requirement: Retention readback remains non-destructive in pilot soak
r[molten.external_live_pilot_soak.retention_readback_boundary] External pilot soak workflows SHOULD bind retention readback, clearance review, or GC-plan evidence for cleanup-sensitive artifacts while denying destructive retention execution unless normal retention plan/apply/execute gates are separately present.

#### Scenario: Retention review does not authorize deletion
- GIVEN pilot soak evidence includes a retention candidate readback bundle
- WHEN a destructive cleanup operation is requested
- THEN the readback bundle alone is insufficient
- AND normal retention admission, apply, execute, tombstone, and audit evidence remains required.

### Requirement: External pilot validation covers positive and negative paths
r[molten.external_live_pilot_soak.validation] External pilot soak readiness SHOULD include deterministic local tests or fixtures for the decision law, negative boundary denials, and release-readback caveats before relying on operator-managed live evidence.

#### Scenario: Negative pilot fixture denies
- GIVEN a generated or fixture pilot evidence set with one required child receipt missing or stale
- WHEN the pilot decision validator runs
- THEN it emits a deny decision with an actionable missing-evidence diagnostic.
