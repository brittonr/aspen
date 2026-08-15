# runtime-spine Spec Delta

## Requirements

### Requirement: Retention candidate bundle verification
r[molten.retention.candidate_bundle_verify] Molten MUST expose a read-only retention candidate bundle verification workflow that validates an exported bundle's manifest, explain artifact, and packaged local retention artifacts without granting deletion authority.

#### Scenario: Verification passes for an intact exported bundle
- GIVEN an exported retention candidate bundle whose `bundle.preserves`, `explain.preserves`, and grouped artifact files match their canonical refs and expected artifact kinds
- WHEN an operator verifies the bundle
- THEN Molten emits `retention-candidate-bundle-verify-v1` evidence with decision `pass`, the bundle ref, explain ref, listed artifact refs, observed file refs, and no diagnostics

#### Scenario: Verification diagnoses tampered or missing packaged artifacts
- GIVEN an exported retention candidate bundle with a missing, tampered, duplicate, unlisted, or unreferenced packaged artifact file
- WHEN an operator verifies the bundle
- THEN Molten emits `retention-candidate-bundle-verify-v1` evidence with decision `deny` and diagnostics identifying the inconsistent bundle evidence

#### Scenario: Verification remains review evidence only
- GIVEN a passing `retention-candidate-bundle-verify-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat verification evidence as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, import, or deletion trust
