# Runtime Spine Spec Delta: Retention GC plan dry-run UX

## Requirements

### Requirement: Retention GC dry-run plans
r[molten.retention.gc_plan_dry_run_ux] Molten MUST expose canonical retention GC dry-run plans that bind a destructive candidate, subsystem, action, requester, computed reference index, explicit destructive evidence inputs, policy gate, authority gate, supporting-evidence gate, reference-index gate, remote-GC gate, imported remote-clearance gate, local retention diagnostics, and final dry-run decision before any destructive mutation.

#### Scenario: Plan lists every destructive gate before mutation
- GIVEN a destructive retention candidate with explicit policy, authority, supporting evidence, reference-index, remote-GC, and remote-clearance inputs
- WHEN an operator requests a retention GC plan
- THEN Molten emits a `retention-gc-plan-v1` artifact that lists each gate and diagnostics without writing retention receipts, tombstones, or deleting content

#### Scenario: Plan evidence is not deletion authority
- GIVEN a passing retention GC plan artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, or compaction
- THEN the subsystem MUST still run normal retention admission and receipt generation, and MUST NOT treat the plan as authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC clearance trust
