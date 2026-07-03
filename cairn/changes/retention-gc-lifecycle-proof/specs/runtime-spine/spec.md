## ADDED Requirements

### Requirement: Retention GC lifecycle proof binds plan apply execute audit
r[molten.retention_gc_lifecycle_proof.ordered_chain] Molten MUST prove that retention GC audit evidence follows a stored dry-run plan, matching recomputed plan, passing apply receipt, matching execution gate, retention receipt, and tombstone evidence where destructive actions require tombstones.

#### Scenario: Audit rejects broken chain
- GIVEN an execution gate whose apply ref does not match the audited plan
- WHEN Molten validates the retention GC audit chain
- THEN the audit or proof receipt decision is `deny`
- AND diagnostics identify the broken plan/apply/execute binding.

### Requirement: Retention GC denies drift before mutation
r[molten.retention_gc_lifecycle_proof.drift_no_mutation] Molten MUST prove that plan drift, denied recomputation, missing normal destructive admission, missing remote clearance import, or missing apply refs deny before deletion, tombstoning, redaction, cache invalidation, or compaction mutation.

#### Scenario: Plan drift leaves content unchanged
- GIVEN a stored GC plan whose recomputed plan ref differs
- WHEN `gc-apply-plan` or a destructive subsystem evaluates the candidate
- THEN the decision is `deny`
- AND before/after state or content refs show no destructive mutation occurred.

### Requirement: Retention GC execution scope is exact
r[molten.retention_gc_lifecycle_proof.execution_scope] Molten MUST prove that a passing execution gate is accepted only for the same subsystem, action, object ref, object kind, retention class, retention receipt, and tombstone refs bound by the apply receipt.

#### Scenario: Scope mismatch denies execution
- GIVEN a passing apply receipt for one object ref
- WHEN an execution gate is requested for another object ref or action
- THEN execution gate decision is `deny`
- AND the destructive subsystem does not remove or tombstone the requested object.
