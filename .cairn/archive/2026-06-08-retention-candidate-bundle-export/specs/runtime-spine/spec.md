# runtime-spine Spec Delta

## Requirements

### Requirement: Retention candidate bundle export
r[molten.retention.candidate_bundle_export] Molten MUST expose a read-only retention candidate bundle export workflow that packages a supplied explain artifact, a canonical bundle manifest, and referenced local retention GC plan/apply/execute/audit, retention receipt, and tombstone artifacts without granting deletion authority.

#### Scenario: Bundle packages local explain evidence for handoff
- GIVEN a `retention-candidate-explain-v1` artifact that references local plan, apply, execute, audit, retention receipt, and tombstone artifacts
- WHEN an operator exports a retention candidate bundle
- THEN Molten writes `explain.preserves`, `bundle.preserves`, and grouped local artifact files for each readable referenced artifact

#### Scenario: Bundle reports missing local artifacts
- GIVEN an explain artifact that references a plan, apply, execute, audit, retention receipt, or tombstone artifact missing from the local retention root
- WHEN an operator exports a retention candidate bundle
- THEN Molten emits bundle diagnostics for the missing artifact and does not mint replacement evidence

#### Scenario: Bundle remains review evidence only
- GIVEN a passing `retention-candidate-bundle-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat bundle evidence as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust
