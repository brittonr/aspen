# Runtime Spine Delta: retention remote GC reconciliation

### Requirement: Remote retention clearance receipts
r[molten.retention.remote_gc_clearance_receipts] Molten MUST represent per-remote destructive retention clearance as canonical receipts that bind peer ref, requester ref, object ref, object kind, retention class, action, remote ref, policy ref, authority ref, freshness, revocation refs, retained remote refs, diagnostics, and checks.

#### Scenario: Clearance binds remote scope
- GIVEN a remote GC clearance receipt
- WHEN Molten parses or admits the receipt for destructive retention
- THEN the receipt ref is canonical and its peer, requester, object, class, action, policy, authority, freshness, revocation, and retained-ref fields are checked before it can support local mutation

r[molten.retention.remote_gc_all_remotes] Molten MUST require every configured or known remote ref supplied to destructive retention evidence to have a current passing clearance before deletion, tombstoning, redaction, or compaction.

#### Scenario: Partial remote clearance denies
- GIVEN destructive retention evidence naming multiple remote refs
- WHEN only a subset has matching current clearance receipts
- THEN Molten denies the destructive operation before local mutation and reports the missing remote refs

### Requirement: Remote reconciliation destructive gate
r[molten.retention.remote_gc_reconciliation_gate] Molten MUST gate ledger GC, chunk GC, and eval-cache invalidation on reconciled per-remote clearance in addition to local policy, authority, supporting evidence, reference-index, and remote-GC admissions.

#### Scenario: Remote uncertainty blocks apply
- GIVEN a destructive ledger, chunk, or cache candidate with stale, revoked, forged, wrong-scope, or retained-remote clearance evidence
- WHEN apply-mode cleanup evaluates the candidate
- THEN the subsystem emits denial evidence and leaves selected content readable

r[molten.retention.remote_gc_diagnostics] Molten MUST surface per-peer and per-remote clearance diagnostics in destructive subsystem receipts without treating clearance receipts as authority, policy, resource, provenance, transport, execution, or source-gate trust.

#### Scenario: Receipt records clearance diagnostics
- GIVEN destructive retention admission with remote refs
- WHEN the subsystem emits its GC or invalidation receipt
- THEN the receipt diagnostics identify missing, stale, revoked, wrong-peer, wrong-object, wrong-action, retained, or forged remote clearance evidence

r[molten.retention.remote_gc_cli] Molten MUST expose operator-facing CLI support for creating remote clearance receipts and supplying remote clearance refs to destructive retention commands.

#### Scenario: CLI supplies remote clearance
- GIVEN an operator has per-remote clearance receipts
- WHEN the operator runs destructive ledger, chunk, or cache cleanup with remote clearance refs
- THEN Molten binds those refs into destructive retention evidence before admission

r[molten.retention.remote_gc_reconciliation_tests] Molten MUST test remote GC reconciliation for partial remote sets, stale or revoked clearance, wrong peer, wrong object or action, retained remote refs, forged refs, and an all-clear pass.

#### Scenario: Tests prove fail-closed remote reconciliation
- GIVEN destructive candidates with incomplete or mismatched remote clearance evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact
