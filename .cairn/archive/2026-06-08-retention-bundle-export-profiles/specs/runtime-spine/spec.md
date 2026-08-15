# runtime-spine Spec Delta

## Requirements

### Requirement: Retention bundle export profiles
r[molten.retention.bundle_export_profiles] Molten MUST expose profile-controlled retention candidate bundle export evidence that distinguishes internal full-fidelity bundles from public deny-sensitive and diagnostic redacted-review handoffs without granting deletion authority.

#### Scenario: Public profile denies sensitive handoff
- GIVEN a retention candidate bundle whose explain artifact or packaged local artifacts contain sensitive markers such as private-secret retention class or encrypted-ref object kind
- WHEN an operator exports the bundle with the public profile
- THEN Molten emits `retention-candidate-bundle-profile-v1` evidence with decision `deny`, marker refs, and diagnostics identifying that public handoff is not safe

#### Scenario: Diagnostic profile writes redacted review view
- GIVEN a retention candidate bundle with sensitive markers
- WHEN an operator exports the bundle with the diagnostic profile
- THEN Molten emits `retention-candidate-bundle-profile-v1` evidence with decision `pass`, marker refs, diagnostic-only loss classification, and redacted review copies that replace sensitive marker tokens

#### Scenario: Profiles remain review evidence only
- GIVEN a passing `retention-candidate-bundle-profile-v1` artifact or diagnostic redacted review view
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat profile evidence or redacted views as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, verification, import, or deletion trust
