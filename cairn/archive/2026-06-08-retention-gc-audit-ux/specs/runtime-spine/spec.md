# runtime-spine Spec Delta

## Requirements

### Requirement: Retention GC audit UX
r[molten.retention.gc_audit_ux] Molten MUST expose a read-only retention GC audit workflow that starts from a stored execution gate ref, follows the bound plan, apply, retention receipt, and tombstone refs, emits canonical audit evidence with consistency diagnostics, and never treats the audit artifact as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, or deletion trust.

#### Scenario: Audit shows passing destructive chain
- GIVEN a passing retention GC plan, apply receipt, execution gate, retention receipt, and tombstone for a destructive subsystem candidate
- WHEN an operator audits the execution gate ref
- THEN Molten emits `retention-gc-audit-v1` evidence that lists the plan, apply, execution, retention receipt, and tombstone refs with a passing audit decision

#### Scenario: Audit denies inconsistent chain
- GIVEN an execution gate whose linked apply, plan, receipt, or tombstone is missing, denied, or scope-mismatched
- WHEN an operator audits the execution gate ref
- THEN Molten emits denial diagnostics and does not mutate retained content or create deletion authority

#### Scenario: Audit remains explanatory evidence
- GIVEN a passing `retention-gc-audit-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching apply and execution gates plus normal destructive admission and MUST NOT treat the audit artifact as authority or clearance
