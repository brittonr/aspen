# Runtime Spine Delta: retention destructive evidence admission

### Requirement: Retention evidence admission model
r[molten.retention.evidence_admission_model] Molten MUST represent destructive retention policy, authority, supporting evidence, reference-index, and remote-GC inputs as typed local admission receipts with canonical ref binding.

#### Scenario: Admission receipt binds canonical ref
- GIVEN a destructive retention evidence admission value
- WHEN Molten parses the supplied admission ref
- THEN the ref matches the canonical admission value and the receipt kind is one of policy, authority, supporting-evidence, reference-index, or remote-GC

r[molten.retention.evidence_scope_binding] Molten MUST require destructive retention admission receipts to bind requester, object ref, object kind, retention class, and action before they can authorize or support mutation.

#### Scenario: Mismatched evidence denies deletion
- GIVEN an admission receipt for a different requester, object, class, or action
- WHEN ledger GC, chunk GC, or cache invalidation evaluates a candidate
- THEN the destructive operation is denied before removal or tombstone mutation

### Requirement: Destructive admission gates
r[molten.retention.destructive_admission_gate] Molten MUST gate destructive ledger GC, chunk GC, and cache invalidation on admitted policy, authority, supporting evidence, reference-index, and remote-GC receipts rather than on syntactic refs alone.

#### Scenario: Forged refs fail closed
- GIVEN syntactically valid refs that do not resolve to passing local admission receipts
- WHEN apply-mode destruction evaluates candidates
- THEN deletion or tombstoning is denied and content remains readable

r[molten.retention.admission_receipt_diagnostics] Molten MUST surface admitted retention refs and admission diagnostics in destructive subsystem receipts without treating policy or support evidence as authority grants.

#### Scenario: Receipt records admission result
- GIVEN a destructive subsystem decision
- WHEN the subsystem emits a receipt
- THEN the receipt lists retention receipt refs, admitted evidence refs, and diagnostics for missing, stale, revoked, mismatched, retained, incomplete-index, or remote-uncertain evidence

r[molten.retention.admission_tests] Molten MUST test destructive retention evidence admission for forged refs, wrong requester, wrong action, wrong object or class, missing reference-index proof, retained refs, unresolved remote refs, and passing admitted evidence.

#### Scenario: Admission tests prove fail-closed mutation
- GIVEN destructive candidates with incomplete or mismatched admission evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact
