# runtime-spine Spec Delta

## Requirements

### Requirement: Operator dogfood retention GC workflow
r[molten.operator_dogfood_retention_gc.workflow] Molten MUST exercise retention GC deletion-safety rails in the local operator dogfood workflow by emitting canonical retention evidence admissions, remote-GC clearance evidence, dry-run plan, apply, execution gate, audit, explain, bundle export/profile/verify, and catalog/MCP discovery artifacts under the explicit dogfood state root.

#### Scenario: Dogfood records retention GC chain
- GIVEN a clean local dogfood state root
- WHEN the operator dogfood workflow runs
- THEN the workflow emits mandatory recorded or deterministic steps for retention GC plan, apply, execute, audit, review bundle verification, and read-only catalog discovery

r[molten.operator_dogfood_retention_gc.release_gate] Molten MUST bind retention GC dogfood refs into operator checkpoints, dogfood reports, release evidence, and local ledger/catalog imports so release review can inspect the deletion-safety chain.

#### Scenario: Release evidence includes retention review artifacts
- GIVEN a passing local dogfood run
- WHEN the dogfood report and release gate receipt are inspected
- THEN they include retention GC step receipts, bundle verification evidence, catalog discovery receipts, and imported retention ledger artifacts

r[molten.operator_dogfood_retention_gc.evidence_only] Molten MUST treat operator dogfood retention GC artifacts as evidence-only release diagnostics that do not replace retention admission, plan, apply, execution, remote clearance, tombstone, policy, authority, provenance, resource, transport, source-gate, remote-GC, or destructive subsystem gates.

#### Scenario: Dogfood evidence does not grant deletion authority
- GIVEN a passing dogfood retention GC workflow
- WHEN a destructive subsystem later attempts deletion, tombstoning, compaction, redaction, ledger GC, chunk GC, or cache invalidation
- THEN the subsystem still requires matching normal retention gates and MUST NOT treat dogfood report, release gate, bundle verify, audit, explain, or catalog search receipts as deletion authority

r[molten.operator_dogfood_retention_gc.tests] Molten MUST test the dogfood retention GC workflow with passing local fixtures and fail-closed coverage for missing or denied mandatory retention evidence.

#### Scenario: Tests cover dogfood retention evidence
- GIVEN the local dogfood test harness
- WHEN the dogfood workflow is executed in tests
- THEN tests assert the retention GC steps, bundle verify evidence, catalog discovery evidence, and pass report are present
