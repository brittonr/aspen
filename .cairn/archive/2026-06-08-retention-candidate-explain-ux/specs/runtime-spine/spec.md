# runtime-spine Spec Delta

## Requirements

### Requirement: Retention candidate explain UX
r[molten.retention.candidate_explain_ux] Molten MUST expose a read-only retention candidate explain workflow that starts from an object ref, optionally narrows by object kind, retention class, action, and subsystem, and emits canonical evidence listing local pins, evidence admissions, remote clearances/imports, retention GC plans, applies, executions, audits, retention receipts, and tombstones without granting deletion authority.

#### Scenario: Explain lists known local evidence before destructive commands
- GIVEN a retention object with local pins, destructive evidence admissions, remote clearances, retention GC plan/apply/execute/audit artifacts, retention receipts, and tombstones
- WHEN an operator explains the candidate by object ref and optional scope filters
- THEN Molten emits `retention-candidate-explain-v1` evidence listing the matching refs and diagnostics without deleting, tombstoning, redacting, compacting, or invalidating content

#### Scenario: Explain is not a destructive gate
- GIVEN a passing `retention-candidate-explain-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat explain evidence as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust

#### Scenario: Audit artifacts become discoverable evidence
- GIVEN an operator has emitted a retention GC audit for an execution gate
- WHEN a later explain command scans the same retention root for that object
- THEN the explain artifact lists the known audit ref alongside the plan, apply, execute, retention receipt, and tombstone refs it explains
